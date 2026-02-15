//! Data Dividends Coordinator Zome
//!
//! Provides extern functions for the Health Data Dividends system.

use hdk::prelude::*;
use dividends_integrity::*;
use mycelix_health_shared::{require_authorization, log_data_access, DataCategory, Permission};

// ==================== DATA CONTRIBUTIONS ====================

/// Record a data contribution
#[hdk_extern]
pub fn create_data_contribution(contribution: DataContribution) -> ExternResult<Record> {
    validate_data_contribution(&contribution)?;

    let auth = require_authorization(
        contribution.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Write,
        false,
    )?;

    let contrib_hash = create_entry(&EntryTypes::DataContribution(contribution.clone()))?;
    let record = get(contrib_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find contribution".to_string())))?;

    // Link to patient
    create_link(
        contribution.patient_hash.clone(),
        contrib_hash,
        LinkTypes::PatientToContributions,
        (),
    )?;

    log_data_access(
        contribution.patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get patient's contributions
#[hdk_extern]
pub fn get_patient_contributions(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToContributions)?,
        GetStrategy::default(),
    )?;

    let mut contributions = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                contributions.push(record);
            }
        }
    }

    if !contributions.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::FinancialData],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(contributions)
}

/// Revoke a contribution
#[hdk_extern]
pub fn revoke_contribution(input: RevokeContributionInput) -> ExternResult<Record> {
    let record = get(input.contribution_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Contribution not found".to_string())))?;

    let mut contribution: DataContribution = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid contribution".to_string())))?;

    let patient_hash = contribution.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Amend,
        false,
    )?;

    contribution.revoked = true;
    contribution.revoked_at = Some(sys_time()?.as_micros() as i64);

    let updated_hash = update_entry(input.contribution_hash, &contribution)?;

    log_data_access(
        patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Amend,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated contribution".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeContributionInput {
    pub contribution_hash: ActionHash,
}

/// Get active (non-revoked) contributions
#[hdk_extern]
pub fn get_active_contributions(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let all = get_patient_contributions(patient_hash)?;

    let active: Vec<Record> = all
        .into_iter()
        .filter(|record| {
            if let Some(contrib) = record.entry().to_app_option::<DataContribution>().ok().flatten() {
                !contrib.revoked
            } else {
                false
            }
        })
        .collect();

    Ok(active)
}

// ==================== DATA USAGE ====================

/// Record data usage
#[hdk_extern]
pub fn record_data_usage(usage: DataUsage) -> ExternResult<Record> {
    let contribution_record = get(usage.contribution_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Contribution not found".to_string())))?;
    let contribution: DataContribution = contribution_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid contribution entry".to_string())))?;

    let patient_hash = contribution.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Write,
        false,
    )?;

    let usage_hash = create_entry(&EntryTypes::DataUsage(usage.clone()))?;
    let record = get(usage_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find usage".to_string())))?;

    // Link to contribution
    create_link(
        usage.contribution_hash.clone(),
        usage_hash.clone(),
        LinkTypes::ContributionToUsages,
        (),
    )?;

    // Link to project
    create_link(
        usage.project_hash.clone(),
        usage_hash,
        LinkTypes::ProjectToUsages,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get usage records for a contribution
#[hdk_extern]
pub fn get_contribution_usages(contribution_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let contribution_record = get(contribution_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Contribution not found".to_string())))?;
    let contribution: DataContribution = contribution_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid contribution entry".to_string())))?;

    let patient_hash = contribution.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(contribution_hash, LinkTypes::ContributionToUsages)?,
        GetStrategy::default(),
    )?;

    let mut usages = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                usages.push(record);
            }
        }
    }

    if !usages.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::FinancialData],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(usages)
}

/// Get all usages for a patient (across all contributions)
#[hdk_extern]
pub fn get_patient_usages(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let contributions = get_patient_contributions(patient_hash)?;

    let mut all_usages = Vec::new();
    for contrib in contributions {
        let usages = get_contribution_usages(contrib.action_address().clone())?;
        all_usages.extend(usages);
    }

    Ok(all_usages)
}

// ==================== REVENUE EVENTS ====================

/// Record a revenue event
#[hdk_extern]
pub fn create_revenue_event(event: RevenueEvent) -> ExternResult<Record> {
    validate_revenue_event(&event)?;

    let event_hash = create_entry(&EntryTypes::RevenueEvent(event.clone()))?;
    let record = get(event_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find event".to_string())))?;

    // Link to project
    create_link(
        event.project_hash.clone(),
        event_hash,
        LinkTypes::UsageToRevenue,
        (),
    )?;

    Ok(record)
}

/// Get revenue events for a project
#[hdk_extern]
pub fn get_project_revenue_events(project_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(project_hash, LinkTypes::UsageToRevenue)?,
        GetStrategy::default(),
    )?;

    let mut events = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                events.push(record);
            }
        }
    }

    Ok(events)
}

// ==================== DIVIDEND DISTRIBUTIONS ====================

/// Create dividend distribution
#[hdk_extern]
pub fn create_dividend_distribution(distribution: DividendDistribution) -> ExternResult<Record> {
    validate_dividend_distribution(&distribution)?;

    let auth = require_authorization(
        distribution.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Write,
        false,
    )?;

    let dist_hash = create_entry(&EntryTypes::DividendDistribution(distribution.clone()))?;
    let record = get(dist_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find distribution".to_string())))?;

    // Link to patient
    create_link(
        distribution.patient_hash.clone(),
        dist_hash.clone(),
        LinkTypes::PatientToDividends,
        (),
    )?;

    // Link to revenue event
    create_link(
        distribution.revenue_hash.clone(),
        dist_hash,
        LinkTypes::RevenueToDistributions,
        (),
    )?;

    log_data_access(
        distribution.patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get patient's dividends
#[hdk_extern]
pub fn get_patient_dividends(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToDividends)?,
        GetStrategy::default(),
    )?;

    let mut dividends = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                dividends.push(record);
            }
        }
    }

    if !dividends.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::FinancialData],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(dividends)
}

/// Get unclaimed dividends
#[hdk_extern]
pub fn get_unclaimed_dividends(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let all_dividends = get_patient_dividends(patient_hash)?;

    let unclaimed: Vec<Record> = all_dividends
        .into_iter()
        .filter(|record| {
            if let Some(dist) = record.entry().to_app_option::<DividendDistribution>().ok().flatten() {
                matches!(dist.status, DistributionStatus::Distributed) && dist.claimed_at.is_none()
            } else {
                false
            }
        })
        .collect();

    Ok(unclaimed)
}

/// Claim a dividend
#[hdk_extern]
pub fn claim_dividend(input: ClaimDividendInput) -> ExternResult<Record> {
    let record = get(input.distribution_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Distribution not found".to_string())))?;

    let mut distribution: DividendDistribution = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid distribution".to_string())))?;

    let patient_hash = distribution.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Amend,
        false,
    )?;

    distribution.status = DistributionStatus::Claimed;
    distribution.claimed_at = Some(sys_time()?.as_micros() as i64);

    let updated_hash = update_entry(input.distribution_hash, &distribution)?;

    log_data_access(
        patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Amend,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated distribution".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClaimDividendInput {
    pub distribution_hash: ActionHash,
}

/// Calculate total dividends for a patient
#[hdk_extern]
pub fn get_dividend_summary(patient_hash: ActionHash) -> ExternResult<DividendSummary> {
    let dividends = get_patient_dividends(patient_hash)?;

    let mut total_earned: f64 = 0.0;
    let mut total_claimed: f64 = 0.0;
    let mut total_donated: f64 = 0.0;
    let mut pending: f64 = 0.0;

    for record in &dividends {
        if let Some(dist) = record.entry().to_app_option::<DividendDistribution>().ok().flatten() {
            let amount = dist.amount.value;
            total_earned += amount;

            match dist.status {
                DistributionStatus::Claimed => total_claimed += amount,
                DistributionStatus::Distributed => pending += amount,
                DistributionStatus::ReturnedToPool => total_donated += amount,
                _ => {}
            }
        }
    }

    Ok(DividendSummary {
        total_earned,
        total_claimed,
        total_donated,
        pending_amount: pending,
        dividend_count: dividends.len() as u32,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DividendSummary {
    pub total_earned: f64,
    pub total_claimed: f64,
    pub total_donated: f64,
    pub pending_amount: f64,
    pub dividend_count: u32,
}

// ==================== DIVIDEND PREFERENCES ====================

/// Set dividend preferences
#[hdk_extern]
pub fn set_dividend_preferences(prefs: DividendPreferences) -> ExternResult<Record> {
    validate_dividend_preferences(&prefs)?;

    let auth = require_authorization(
        prefs.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Write,
        false,
    )?;

    let prefs_hash = create_entry(&EntryTypes::DividendPreferences(prefs.clone()))?;
    let record = get(prefs_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find preferences".to_string())))?;

    // Link to patient
    let patient_hash = prefs.patient_hash.clone();
    create_link(
        patient_hash.clone(),
        prefs_hash,
        LinkTypes::PatientToPreferences,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get patient's dividend preferences
#[hdk_extern]
pub fn get_dividend_preferences(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToPreferences)?,
        GetStrategy::default(),
    )?;

    if let Some(link) = links.last() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            let record = get(hash, GetOptions::default())?;
            if record.is_some() {
                log_data_access(
                    patient_hash,
                    vec![DataCategory::FinancialData],
                    Permission::Read,
                    auth.consent_hash,
                    auth.emergency_override,
                    None,
                )?;
            }
            return Ok(record);
        }
    }

    Ok(None)
}

// ==================== RESEARCH PROJECTS ====================

/// Create research project
#[hdk_extern]
pub fn create_research_project(project: ResearchProject) -> ExternResult<Record> {
    validate_research_project(&project)?;

    let project_hash = create_entry(&EntryTypes::ResearchProject(project.clone()))?;
    let record = get(project_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find project".to_string())))?;

    // Link to active projects
    let anchor = anchor_hash("active_projects")?;
    create_link(
        anchor,
        project_hash,
        LinkTypes::ActiveProjects,
        (),
    )?;

    Ok(record)
}

/// Get active research projects
#[hdk_extern]
pub fn get_active_projects(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("active_projects")?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::ActiveProjects)?,
        GetStrategy::default(),
    )?;

    let mut projects = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(proj) = record.entry().to_app_option::<ResearchProject>().ok().flatten() {
                    if !matches!(proj.status, ProjectStatus::Completed | ProjectStatus::Terminated) {
                        projects.push(record);
                    }
                }
            }
        }
    }

    Ok(projects)
}

/// Add contribution to project
#[hdk_extern]
pub fn add_contribution_to_project(input: AddContributionInput) -> ExternResult<()> {
    create_link(
        input.project_hash,
        input.contribution_hash,
        LinkTypes::ProjectToContributions,
        (),
    )?;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddContributionInput {
    pub project_hash: ActionHash,
    pub contribution_hash: ActionHash,
}

/// Get project's contributions
#[hdk_extern]
pub fn get_project_contributions(project_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let caller = agent_info()?.agent_initial_pubkey;
    let project_record = get(project_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Project not found".to_string())))?;
    if project_record.action().author() != &caller {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the project creator can view contributions".to_string()
        )));
    }

    let links = get_links(
        LinkQuery::try_new(project_hash, LinkTypes::ProjectToContributions)?,
        GetStrategy::default(),
    )?;

    let mut contributions = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                contributions.push(record);
            }
        }
    }

    Ok(contributions)
}

/// Update project status
#[hdk_extern]
pub fn update_project_status(input: UpdateProjectStatusInput) -> ExternResult<Record> {
    let record = get(input.project_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Project not found".to_string())))?;

    let caller = agent_info()?.agent_initial_pubkey;
    if record.action().author() != &caller {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the project creator can update status".to_string()
        )));
    }

    let mut project: ResearchProject = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid project".to_string())))?;

    project.status = input.new_status;

    // Check project.status since input.new_status was moved
    if matches!(project.status, ProjectStatus::Completed | ProjectStatus::Terminated) {
        project.actual_end_date = Some(sys_time()?.as_micros() as i64);
    }

    let updated_hash = update_entry(input.project_hash, &project)?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated project".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateProjectStatusInput {
    pub project_hash: ActionHash,
    pub new_status: ProjectStatus,
}

// ==================== ATTRIBUTION CHAINS ====================

/// Create attribution chain
#[hdk_extern]
pub fn create_attribution_chain(chain: AttributionChain) -> ExternResult<Record> {
    let chain_hash = create_entry(&EntryTypes::AttributionChain(chain.clone()))?;
    let record = get(chain_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find chain".to_string())))?;

    // Link to chains anchor
    let anchor = anchor_hash("attribution_chains")?;
    create_link(
        anchor,
        chain_hash,
        LinkTypes::AttributionChains,
        (),
    )?;

    Ok(record)
}

/// Add link to attribution chain
#[hdk_extern]
pub fn add_attribution_link(input: AddAttributionLinkInput) -> ExternResult<Record> {
    let record = get(input.chain_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Chain not found".to_string())))?;

    let mut chain: AttributionChain = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid chain".to_string())))?;

    let next_step = chain.chain.len() as u32 + 1;
    chain.chain.push(AttributionLink {
        step: next_step,
        link_type: input.link_type,
        actor: input.actor,
        description: input.description,
        timestamp: sys_time()?.as_micros() as i64,
        record_hash: input.record_hash,
    });
    chain.updated_at = sys_time()?.as_micros() as i64;

    let updated_hash = update_entry(input.chain_hash, &chain)?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated chain".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddAttributionLinkInput {
    pub chain_hash: ActionHash,
    pub link_type: AttributionLinkType,
    pub actor: String,
    pub description: String,
    pub record_hash: Option<ActionHash>,
}

/// Set chain outcome
#[hdk_extern]
pub fn set_chain_outcome(input: SetChainOutcomeInput) -> ExternResult<Record> {
    let record = get(input.chain_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Chain not found".to_string())))?;

    let mut chain: AttributionChain = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid chain".to_string())))?;

    chain.final_outcome = Some(input.outcome);
    chain.updated_at = sys_time()?.as_micros() as i64;

    let updated_hash = update_entry(input.chain_hash, &chain)?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated chain".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetChainOutcomeInput {
    pub chain_hash: ActionHash,
    pub outcome: ChainOutcome,
}

// ==================== DIVIDEND POOLS ====================

/// Create dividend pool
#[hdk_extern]
pub fn create_dividend_pool(pool: DividendPool) -> ExternResult<Record> {
    let pool_hash = create_entry(&EntryTypes::DividendPool(pool.clone()))?;
    let record = get(pool_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find pool".to_string())))?;

    // Link to pools anchor
    let anchor = anchor_hash("dividend_pools")?;
    create_link(
        anchor,
        pool_hash,
        LinkTypes::DividendPools,
        (),
    )?;

    Ok(record)
}

/// Get all dividend pools
#[hdk_extern]
pub fn get_dividend_pools(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("dividend_pools")?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::DividendPools)?,
        GetStrategy::default(),
    )?;

    let mut pools = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                pools.push(record);
            }
        }
    }

    Ok(pools)
}

/// Update pool balance
#[hdk_extern]
pub fn update_pool_balance(input: UpdatePoolBalanceInput) -> ExternResult<Record> {
    let record = get(input.pool_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Pool not found".to_string())))?;

    let mut pool: DividendPool = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid pool".to_string())))?;

    pool.balance = input.new_balance;

    if input.is_distribution {
        pool.total_distributed += input.distributed_amount.unwrap_or(0.0);
        pool.last_distribution_at = Some(sys_time()?.as_micros() as i64);
    }

    let updated_hash = update_entry(input.pool_hash, &pool)?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated pool".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdatePoolBalanceInput {
    pub pool_hash: ActionHash,
    pub new_balance: f64,
    pub is_distribution: bool,
    pub distributed_amount: Option<f64>,
}

// ==================== HELPER FUNCTIONS ====================

/// Calculate dividends for a revenue event
#[hdk_extern]
pub fn calculate_distributions(input: CalculateDistributionsInput) -> ExternResult<Vec<CalculatedDistribution>> {
    let event_record = get(input.revenue_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Revenue event not found".to_string())))?;

    let event: RevenueEvent = event_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid revenue event".to_string())))?;

    let patient_pool = event.total_value * (event.patient_pool_percent as f64 / 100.0);

    // Get all contributions and their quality scores
    let mut total_weighted_score = 0.0;
    let mut contrib_scores: Vec<(ActionHash, ActionHash, f64)> = Vec::new(); // (contrib_hash, patient_hash, score)

    for contrib_hash in &event.contributing_data {
        if let Some(record) = get(contrib_hash.clone(), GetOptions::default())? {
            if let Some(contrib) = record.entry().to_app_option::<DataContribution>().ok().flatten() {
                if !contrib.revoked {
                    let weighted_score = calculate_contribution_weight(&contrib);
                    total_weighted_score += weighted_score;
                    contrib_scores.push((contrib_hash.clone(), contrib.patient_hash, weighted_score));
                }
            }
        }
    }

    // Calculate each patient's share
    let mut distributions = Vec::new();
    for (contrib_hash, patient_hash, score) in contrib_scores {
        let share_percent = if total_weighted_score > 0.0 {
            (score / total_weighted_score) as f32
        } else {
            0.0
        };

        let amount = patient_pool * (share_percent as f64);

        distributions.push(CalculatedDistribution {
            patient_hash,
            contribution_hash: contrib_hash,
            amount,
            share_percent,
            contribution_weight: score as f32,
        });
    }

    Ok(distributions)
}

/// Calculate contribution weight (used for distribution)
fn calculate_contribution_weight(contribution: &DataContribution) -> f64 {
    let base_score = contribution.quality_score as f64;
    let size_factor = (contribution.contribution_size.data_point_count as f64).log10().max(1.0);
    let time_factor = (contribution.contribution_size.time_span_days as f64 / 365.0).min(2.0);

    base_score * size_factor * time_factor
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CalculateDistributionsInput {
    pub revenue_hash: ActionHash,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CalculatedDistribution {
    pub patient_hash: ActionHash,
    pub contribution_hash: ActionHash,
    pub amount: f64,
    pub share_percent: f32,
    pub contribution_weight: f32,
}

/// Get impact summary for a patient
#[hdk_extern]
pub fn get_patient_impact_summary(patient_hash: ActionHash) -> ExternResult<PatientImpactSummary> {
    let contributions = get_patient_contributions(patient_hash.clone())?;
    let usages = get_patient_usages(patient_hash.clone())?;
    let dividends = get_patient_dividends(patient_hash)?;

    let mut total_data_points: u64 = 0;
    let mut total_earnings: f64 = 0.0;

    for record in &contributions {
        if let Some(contrib) = record.entry().to_app_option::<DataContribution>().ok().flatten() {
            total_data_points += contrib.contribution_size.data_point_count;
        }
    }

    // Count unique projects
    let mut unique_projects: Vec<ActionHash> = Vec::new();
    for record in &usages {
        if let Some(usage) = record.entry().to_app_option::<DataUsage>().ok().flatten() {
            if !unique_projects.contains(&usage.project_hash) {
                unique_projects.push(usage.project_hash);
            }
        }
    }
    let projects_contributed = unique_projects.len() as u32;

    for record in &dividends {
        if let Some(dist) = record.entry().to_app_option::<DividendDistribution>().ok().flatten() {
            total_earnings += dist.amount.value;
        }
    }

    Ok(PatientImpactSummary {
        total_contributions: contributions.len() as u32,
        total_data_points,
        projects_contributed,
        total_usages: usages.len() as u32,
        total_earnings,
        total_dividends: dividends.len() as u32,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PatientImpactSummary {
    pub total_contributions: u32,
    pub total_data_points: u64,
    pub projects_contributed: u32,
    pub total_usages: u32,
    pub total_earnings: f64,
    pub total_dividends: u32,
}

// ==================== CLINICAL TRIALS INTEGRATION ====================
// Handler functions for trials zome integration

/// Input for creating trial data contribution (from trials zome)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrialDataContributionInput {
    pub contribution_id: String,
    pub patient_hash: ActionHash,
    pub data_type: String,
    pub data_categories: Vec<String>,
    pub consent_hash: ActionHash,
    pub trial_hash: ActionHash,
    pub trial_nct: String,
    pub trial_title: String,
    pub trial_phase: String,
    pub contributed_at: i64,
    pub permitted_uses: Vec<String>,
    pub prohibited_uses: Vec<String>,
}

/// Create a data contribution for clinical trial enrollment
/// Called by the trials zome when a patient enrolls in a trial
#[hdk_extern]
pub fn create_trial_contribution(input: TrialDataContributionInput) -> ExternResult<Record> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Write,
        false,
    )?;

    // Convert string data categories to enum
    let data_categories: Vec<DataContributionCategory> = input.data_categories
        .iter()
        .map(|cat| string_to_contribution_category(cat))
        .collect();

    // Convert permitted uses
    let permitted_uses: Vec<PermittedUse> = input.permitted_uses
        .iter()
        .filter_map(|use_str| string_to_permitted_use(use_str))
        .collect();

    // Convert prohibited uses
    let prohibited_uses: Vec<ProhibitedUse> = input.prohibited_uses
        .iter()
        .filter_map(|use_str| string_to_prohibited_use(use_str))
        .collect();

    // Create a hash for the data (in practice this would be a real data hash)
    let mut data_hash = [0u8; 32];
    let hash_input = format!(
        "TRIAL:{}:{}:{}",
        input.trial_nct,
        input.patient_hash,
        input.contributed_at
    );
    for (i, byte) in hash_input.bytes().take(32).enumerate() {
        data_hash[i] = byte;
    }

    let contribution = DataContribution {
        contribution_id: input.contribution_id,
        patient_hash: input.patient_hash.clone(),
        data_type: ContributedDataType::TreatmentOutcomes,
        data_categories,
        data_hash,
        contribution_size: ContributionSize {
            record_count: 1, // Initial enrollment
            time_span_days: 0, // Will grow as trial progresses
            data_point_count: 0, // Will grow with visits
            size_bytes: None,
        },
        quality_score: 0.95, // Trial data is typically high quality
        consent_hash: input.consent_hash,
        permitted_uses,
        prohibited_uses,
        contributed_at: input.contributed_at,
        valid_until: None, // Trial data doesn't expire
        revoked: false,
        revoked_at: None,
    };

    // Create the contribution entry
    let contrib_hash = create_entry(&EntryTypes::DataContribution(contribution.clone()))?;
    let record = get(contrib_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find contribution".to_string())))?;

    // Link to patient
    create_link(
        input.patient_hash.clone(),
        contrib_hash.clone(),
        LinkTypes::PatientToContributions,
        (),
    )?;

    // Link to trial (using project link type)
    create_link(
        input.trial_hash,
        contrib_hash,
        LinkTypes::ProjectToContributions,
        (),
    )?;

    log_data_access(
        input.patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Input for tracking trial visit data usage (from trials zome)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrialVisitDataUsageInput {
    pub usage_id: String,
    pub patient_hash: ActionHash,
    pub trial_hash: ActionHash,
    pub visit_id: String,
    pub visit_number: u32,
    pub data_points_count: u64,
    pub collected_at: i64,
}

/// Track data collection from a clinical trial visit
/// Called by the trials zome when visit data is recorded
#[hdk_extern]
pub fn track_trial_data_usage(input: TrialVisitDataUsageInput) -> ExternResult<Record> {
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::FinancialData,
        Permission::Write,
        false,
    )?;

    // Find the patient's contribution for this trial
    let patient_contributions = get_patient_contributions(input.patient_hash.clone())?;

    // Find contribution linked to this trial
    let contribution_hash = patient_contributions.iter()
        .find_map(|record| {
            // Check if this contribution is linked to the trial
            let links = get_links(
                LinkQuery::try_new(input.trial_hash.clone(), LinkTypes::ProjectToContributions).ok()?,
                GetStrategy::default(),
            ).ok()?;

            links.iter()
                .find(|link| link.target.clone().into_action_hash() == Some(record.action_address().clone()))
                .map(|_| record.action_address().clone())
        });

    let contribution_hash = match contribution_hash {
        Some(hash) => hash,
        None => {
            // No contribution found - this shouldn't happen if enrollment worked
            return Err(wasm_error!(WasmErrorInner::Guest(
                "No contribution found for this trial".to_string()
            )));
        }
    };

    // Update the contribution size
    if let Some(record) = get(contribution_hash.clone(), GetOptions::default())? {
        if let Some(mut contribution) = record.entry().to_app_option::<DataContribution>().ok().flatten() {
            contribution.contribution_size.record_count += 1;
            contribution.contribution_size.data_point_count += input.data_points_count;
            update_entry(contribution_hash.clone(), &contribution)?;
        }
    }

    // Create usage record
    let usage = DataUsage {
        usage_id: input.usage_id,
        contribution_hash: contribution_hash.clone(),
        project_hash: input.trial_hash.clone(),
        usage_type: UsageType::ClinicalTrial,
        usage_description: format!(
            "Trial Visit {} - {} data points collected",
            input.visit_number, input.data_points_count
        ),
        started_at: input.collected_at,
        ended_at: Some(input.collected_at), // Visit data collection is instantaneous
        impact_metrics: Some(ImpactMetrics {
            contribution_weight: 0.5, // Individual contribution weight
            citations: None,
            patients_benefited: None,
            cost_savings: None,
            evidence_level: Some(EvidenceLevel::Level2), // Level 2 = RCT (clinical trial)
        }),
        revenue_generating: false, // Not directly revenue generating
        revenue_hash: None,
    };

    let usage_hash = create_entry(&EntryTypes::DataUsage(usage.clone()))?;
    let record = get(usage_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find usage".to_string())))?;

    // Link to contribution
    create_link(
        contribution_hash,
        usage_hash.clone(),
        LinkTypes::ContributionToUsages,
        (),
    )?;

    // Link to project (trial)
    create_link(
        input.trial_hash,
        usage_hash,
        LinkTypes::ProjectToUsages,
        (),
    )?;

    log_data_access(
        input.patient_hash,
        vec![DataCategory::FinancialData],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Convert string to DataContributionCategory enum
fn string_to_contribution_category(cat: &str) -> DataContributionCategory {
    match cat {
        "Demographics" => DataContributionCategory::Demographics,
        "Diagnoses" => DataContributionCategory::Diagnoses,
        "Medications" => DataContributionCategory::Medications,
        "Procedures" => DataContributionCategory::Procedures,
        "LabResults" => DataContributionCategory::LabResults,
        "VitalSigns" => DataContributionCategory::VitalSigns,
        "Immunizations" => DataContributionCategory::Immunizations,
        "Allergies" => DataContributionCategory::Allergies,
        "FamilyHistory" => DataContributionCategory::FamilyHistory,
        "SocialHistory" => DataContributionCategory::SocialHistory,
        "MentalHealth" => DataContributionCategory::MentalHealth,
        "Genomics" => DataContributionCategory::Genomics,
        "Imaging" => DataContributionCategory::Imaging,
        "Outcomes" | _ => DataContributionCategory::Outcomes,
    }
}

/// Convert string to PermittedUse enum
fn string_to_permitted_use(use_str: &str) -> Option<PermittedUse> {
    match use_str {
        "AcademicResearch" => Some(PermittedUse::AcademicResearch),
        "CommercialResearch" => Some(PermittedUse::CommercialResearch),
        "DrugDevelopment" => Some(PermittedUse::DrugDevelopment),
        "AITraining" => Some(PermittedUse::AITraining),
        "PublicHealth" => Some(PermittedUse::PublicHealth),
        "QualityImprovement" => Some(PermittedUse::QualityImprovement),
        "PopulationHealth" => Some(PermittedUse::PopulationHealth),
        "DiseaseSurveillance" => Some(PermittedUse::DiseaseSurveillance),
        "ClinicalDecisionSupport" => Some(PermittedUse::ClinicalDecisionSupport),
        _ => None,
    }
}

/// Convert string to ProhibitedUse enum
fn string_to_prohibited_use(use_str: &str) -> Option<ProhibitedUse> {
    match use_str {
        "Marketing" => Some(ProhibitedUse::Marketing),
        "InsuranceUnderwriting" => Some(ProhibitedUse::InsuranceUnderwriting),
        "EmploymentDecisions" => Some(ProhibitedUse::EmploymentDecisions),
        "LawEnforcement" => Some(ProhibitedUse::LawEnforcement),
        "ReIdentification" => Some(ProhibitedUse::ReIdentification),
        "DataSale" => Some(ProhibitedUse::DataSale),
        "WeaponsDevelopment" => Some(ProhibitedUse::WeaponsDevelopment),
        _ => None,
    }
}

// ==================== ANCHOR SUPPORT ====================

/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}
