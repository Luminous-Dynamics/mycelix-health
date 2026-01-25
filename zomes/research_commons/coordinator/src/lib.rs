//! Research Commons Coordinator Zome
//!
//! Provides extern functions for managing de-identified research datasets,
//! data access agreements, and research data sharing.

use hdk::prelude::*;
use research_commons_integrity::*;

// ==================== DATASET MANAGEMENT ====================

/// Create a new research dataset
#[hdk_extern]
pub fn create_dataset(dataset: ResearchDataset) -> ExternResult<Record> {
    let dataset_hash = create_entry(&EntryTypes::ResearchDataset(dataset.clone()))?;
    let record = get(dataset_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find dataset".to_string())))?;

    // Link to all datasets
    let all_anchor = anchor_hash("all_datasets")?;
    create_link(all_anchor, dataset_hash.clone(), LinkTypes::AllDatasets, ())?;

    // Link by each category
    for category in &dataset.categories {
        let cat_anchor = anchor_hash(&format!("category_{:?}", category))?;
        create_link(cat_anchor, dataset_hash.clone(), LinkTypes::DatasetsByCategory, ())?;
    }

    // Link by access level
    let level_anchor = anchor_hash(&format!("access_{:?}", dataset.access_level))?;
    create_link(level_anchor, dataset_hash, LinkTypes::DatasetsByAccessLevel, ())?;

    Ok(record)
}

/// Get a dataset by hash
#[hdk_extern]
pub fn get_dataset(dataset_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(dataset_hash, GetOptions::default())
}

/// Get all available datasets
#[hdk_extern]
pub fn get_all_datasets(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("all_datasets")?;
    get_linked_records(anchor, LinkTypes::AllDatasets)
}

/// Get datasets by category
#[hdk_extern]
pub fn get_datasets_by_category(category: ResearchDataCategory) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("category_{:?}", category))?;
    get_linked_records(anchor, LinkTypes::DatasetsByCategory)
}

/// Get datasets by access level
#[hdk_extern]
pub fn get_datasets_by_access_level(level: DataAccessLevel) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("access_{:?}", level))?;
    get_linked_records(anchor, LinkTypes::DatasetsByAccessLevel)
}

/// Update dataset status
#[hdk_extern]
pub fn update_dataset_status(input: UpdateDatasetStatusInput) -> ExternResult<Record> {
    let record = get(input.dataset_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Dataset not found".to_string())))?;

    let mut dataset: ResearchDataset = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid dataset".to_string())))?;

    dataset.status = input.status;
    dataset.updated_at = sys_time()?;

    let updated_hash = update_entry(input.dataset_hash, &dataset)?;
    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated dataset".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateDatasetStatusInput {
    pub dataset_hash: ActionHash,
    pub status: DatasetStatus,
}

/// Search datasets by criteria
#[hdk_extern]
pub fn search_datasets(criteria: DatasetSearchCriteria) -> ExternResult<Vec<Record>> {
    let all = get_all_datasets(())?;

    let filtered: Vec<Record> = all
        .into_iter()
        .filter(|record| {
            if let Some(dataset) = record.entry().to_app_option::<ResearchDataset>().ok().flatten() {
                matches_criteria(&dataset, &criteria)
            } else {
                false
            }
        })
        .collect();

    Ok(filtered)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatasetSearchCriteria {
    pub categories: Option<Vec<ResearchDataCategory>>,
    pub access_level: Option<DataAccessLevel>,
    pub min_records: Option<u64>,
    pub condition_codes: Option<Vec<String>>,
    pub keyword: Option<String>,
}

fn matches_criteria(dataset: &ResearchDataset, criteria: &DatasetSearchCriteria) -> bool {
    // Check status
    if dataset.status != DatasetStatus::Available {
        return false;
    }

    // Check categories
    if let Some(ref cats) = criteria.categories {
        if !cats.iter().any(|c| dataset.categories.contains(c)) {
            return false;
        }
    }

    // Check access level
    if let Some(ref level) = criteria.access_level {
        if &dataset.access_level != level {
            return false;
        }
    }

    // Check minimum records
    if let Some(min) = criteria.min_records {
        if dataset.record_count < min {
            return false;
        }
    }

    // Check condition codes
    if let Some(ref codes) = criteria.condition_codes {
        if !codes.iter().any(|c| dataset.condition_codes.contains(c)) {
            return false;
        }
    }

    // Check keyword
    if let Some(ref keyword) = criteria.keyword {
        let kw_lower = keyword.to_lowercase();
        if !dataset.title.to_lowercase().contains(&kw_lower)
            && !dataset.description.to_lowercase().contains(&kw_lower)
        {
            return false;
        }
    }

    true
}

// ==================== ACCESS AGREEMENTS ====================

/// Submit a data access request
#[hdk_extern]
pub fn submit_access_request(agreement: DataAccessAgreement) -> ExternResult<Record> {
    let agreement_hash = create_entry(&EntryTypes::DataAccessAgreement(agreement.clone()))?;
    let record = get(agreement_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find agreement".to_string())))?;

    // Link to dataset
    create_link(
        agreement.dataset_hash.clone(),
        agreement_hash.clone(),
        LinkTypes::DatasetToAgreements,
        (),
    )?;

    // Link to researcher
    create_link(
        agreement.researcher_hash,
        agreement_hash,
        LinkTypes::ResearcherToAgreements,
        (),
    )?;

    Ok(record)
}

/// Get access agreement by hash
#[hdk_extern]
pub fn get_access_agreement(agreement_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(agreement_hash, GetOptions::default())
}

/// Get all access requests for a dataset
#[hdk_extern]
pub fn get_dataset_access_requests(dataset_hash: ActionHash) -> ExternResult<Vec<Record>> {
    get_linked_records(dataset_hash, LinkTypes::DatasetToAgreements)
}

/// Get researcher's access agreements
#[hdk_extern]
pub fn get_researcher_agreements(researcher_hash: ActionHash) -> ExternResult<Vec<Record>> {
    get_linked_records(researcher_hash, LinkTypes::ResearcherToAgreements)
}

/// Approve an access request
#[hdk_extern]
pub fn approve_access_request(input: ApproveAccessInput) -> ExternResult<Record> {
    let record = get(input.agreement_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Agreement not found".to_string())))?;

    let mut agreement: DataAccessAgreement = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid agreement".to_string())))?;

    agreement.status = AccessRequestStatus::Approved;
    agreement.approved_at = Some(sys_time()?);
    agreement.approved_by = Some(ActionHash::from_raw_36(agent_info()?.agent_initial_pubkey.get_raw_36().to_vec()));
    agreement.expires_at = input.expires_at;
    agreement.conditions = input.conditions;

    let updated_hash = update_entry(input.agreement_hash, &agreement)?;
    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated agreement".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApproveAccessInput {
    pub agreement_hash: ActionHash,
    pub expires_at: Option<Timestamp>,
    pub conditions: Vec<String>,
}

/// Deny an access request
#[hdk_extern]
pub fn deny_access_request(input: DenyAccessInput) -> ExternResult<Record> {
    let record = get(input.agreement_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Agreement not found".to_string())))?;

    let mut agreement: DataAccessAgreement = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid agreement".to_string())))?;

    agreement.status = AccessRequestStatus::Denied;
    agreement.conditions = vec![input.reason];

    let updated_hash = update_entry(input.agreement_hash, &agreement)?;
    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated agreement".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DenyAccessInput {
    pub agreement_hash: ActionHash,
    pub reason: String,
}

/// Revoke an access agreement
#[hdk_extern]
pub fn revoke_access(input: RevokeAccessInput) -> ExternResult<Record> {
    let record = get(input.agreement_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Agreement not found".to_string())))?;

    let mut agreement: DataAccessAgreement = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid agreement".to_string())))?;

    agreement.status = AccessRequestStatus::Revoked;
    agreement.conditions.push(format!("Revoked: {}", input.reason));

    let updated_hash = update_entry(input.agreement_hash, &agreement)?;
    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated agreement".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeAccessInput {
    pub agreement_hash: ActionHash,
    pub reason: String,
}

// ==================== ACCESS LOGGING ====================

/// Log data access
#[hdk_extern]
pub fn log_data_access(log: DataAccessLog) -> ExternResult<Record> {
    // Verify valid agreement
    let agreement_record = get(log.agreement_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Agreement not found".to_string())))?;

    let agreement: DataAccessAgreement = agreement_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid agreement".to_string())))?;

    // Check agreement is approved and not expired
    if agreement.status != AccessRequestStatus::Approved {
        return Err(wasm_error!(WasmErrorInner::Guest("Access agreement not approved".to_string())));
    }

    if let Some(expires) = agreement.expires_at {
        if expires < sys_time()? {
            return Err(wasm_error!(WasmErrorInner::Guest("Access agreement has expired".to_string())));
        }
    }

    let log_hash = create_entry(&EntryTypes::DataAccessLog(log.clone()))?;
    let record = get(log_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find log".to_string())))?;

    // Link to dataset
    create_link(log.dataset_hash, log_hash, LinkTypes::DatasetToAccessLogs, ())?;

    Ok(record)
}

/// Get access logs for a dataset
#[hdk_extern]
pub fn get_dataset_access_logs(dataset_hash: ActionHash) -> ExternResult<Vec<Record>> {
    get_linked_records(dataset_hash, LinkTypes::DatasetToAccessLogs)
}

// ==================== DATA CONTRIBUTIONS ====================

/// Record a data contribution
#[hdk_extern]
pub fn record_contribution(contribution: DataContribution) -> ExternResult<Record> {
    let contribution_hash = create_entry(&EntryTypes::DataContribution(contribution.clone()))?;
    let record = get(contribution_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find contribution".to_string())))?;

    // Link to dataset
    create_link(
        contribution.dataset_hash,
        contribution_hash,
        LinkTypes::DatasetToContributions,
        (),
    )?;

    Ok(record)
}

/// Get contributions to a dataset
#[hdk_extern]
pub fn get_dataset_contributions(dataset_hash: ActionHash) -> ExternResult<Vec<Record>> {
    get_linked_records(dataset_hash, LinkTypes::DatasetToContributions)
}

/// Get contribution statistics for a dataset
#[hdk_extern]
pub fn get_contribution_stats(dataset_hash: ActionHash) -> ExternResult<ContributionStats> {
    let contributions = get_dataset_contributions(dataset_hash)?;

    let mut total_records = 0u64;
    let mut total_contributors = 0u32;
    let mut dividend_eligible_count = 0u32;

    for record in contributions {
        if let Some(contribution) = record.entry().to_app_option::<DataContribution>().ok().flatten() {
            total_records += contribution.record_count;
            total_contributors += 1;
            if contribution.dividend_eligible {
                dividend_eligible_count += 1;
            }
        }
    }

    Ok(ContributionStats {
        total_records,
        total_contributors,
        dividend_eligible_count,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContributionStats {
    pub total_records: u64,
    pub total_contributors: u32,
    pub dividend_eligible_count: u32,
}

// ==================== DATA QUALITY ====================

/// Submit a quality report
#[hdk_extern]
pub fn submit_quality_report(report: DataQualityReport) -> ExternResult<Record> {
    let report_hash = create_entry(&EntryTypes::DataQualityReport(report.clone()))?;
    let record = get(report_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find report".to_string())))?;

    // Link to dataset
    create_link(
        report.dataset_hash,
        report_hash,
        LinkTypes::DatasetToQualityReports,
        (),
    )?;

    Ok(record)
}

/// Get quality reports for a dataset
#[hdk_extern]
pub fn get_dataset_quality_reports(dataset_hash: ActionHash) -> ExternResult<Vec<Record>> {
    get_linked_records(dataset_hash, LinkTypes::DatasetToQualityReports)
}

/// Get latest quality score for a dataset
#[hdk_extern]
pub fn get_latest_quality_score(dataset_hash: ActionHash) -> ExternResult<Option<u32>> {
    let reports = get_dataset_quality_reports(dataset_hash)?;

    let latest = reports
        .iter()
        .filter_map(|r| r.entry().to_app_option::<DataQualityReport>().ok().flatten())
        .max_by_key(|r| r.assessed_at.as_micros());

    Ok(latest.map(|r| r.quality_score))
}

// ==================== HELPER FUNCTIONS ====================

/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}

fn get_linked_records(base: impl Into<AnyLinkableHash>, link_type: LinkTypes) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(base.into(), link_type)?,
        GetStrategy::default()
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}
