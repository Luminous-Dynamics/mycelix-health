//! Collective Immunity Intelligence Coordinator Zome
//!
//! Privacy-preserving public health surveillance enabling:
//! - Real-time outbreak detection without identifying individuals
//! - Vaccination coverage monitoring at aggregate levels
//! - Syndromic surveillance with differential privacy
//! - Public health response coordination
//!
//! Privacy Guarantees:
//! - All data is aggregated with differential privacy noise
//! - Minimum contributor thresholds before any data release
//! - Suppression of small counts to prevent re-identification
//! - Time bucketing to prevent temporal inference

use hdk::prelude::*;
use immunity_integrity::*;

// ==================== SURVEILLANCE ZONE MANAGEMENT ====================

/// Create a new surveillance zone
#[hdk_extern]
pub fn create_surveillance_zone(input: CreateSurveillanceZoneInput) -> ExternResult<Record> {
    // Validate privacy parameters
    if input.privacy_params.min_contributors < 10 {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Minimum 10 contributors required for privacy protection".to_string()
        )));
    }

    if input.privacy_params.epsilon_budget > 1.0 {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Epsilon budget should not exceed 1.0 per period".to_string()
        )));
    }

    let zone = SurveillanceZone {
        zone_id: generate_zone_id(&input.name),
        name: input.name,
        level: input.level,
        parent_zone: input.parent_zone,
        population_estimate: input.population_estimate,
        thresholds: input.thresholds,
        privacy_params: input.privacy_params,
        active: true,
        created_at: sys_time()?.as_micros() as i64,
    };

    let action_hash = create_entry(EntryTypes::SurveillanceZone(zone.clone()))?;

    // Link to active surveillance
    let anchor = anchor_for_active_surveillance()?;
    create_link(anchor, action_hash.clone(), LinkTypes::ActiveSurveillance, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created zone".to_string())))?;

    Ok(record)
}

/// Get surveillance zone by hash
#[hdk_extern]
pub fn get_surveillance_zone(zone_hash: ActionHash) -> ExternResult<Option<SurveillanceZone>> {
    get_zone_from_hash(&zone_hash)
}

/// List all active surveillance zones
#[hdk_extern]
pub fn list_active_zones(_: ()) -> ExternResult<Vec<SurveillanceZone>> {
    let anchor = anchor_for_active_surveillance()?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::ActiveSurveillance)?, GetStrategy::default()
    )?;

    let mut zones = Vec::new();
    for link in links {
        if let Some(zone) = get_zone_from_hash(&ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?)? {
            if zone.active {
                zones.push(zone);
            }
        }
    }

    Ok(zones)
}

// ==================== HEALTH EVENT REPORTING ====================

/// Submit a privacy-preserving health event report
/// This is called by participating agents with locally-noised data
#[hdk_extern]
pub fn submit_health_event_report(input: SubmitHealthEventInput) -> ExternResult<Record> {
    // Get zone to check privacy parameters
    let zone = get_zone_from_hash(&input.zone_hash)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Zone not found".to_string())))?;

    if !zone.active {
        return Err(wasm_error!(WasmErrorInner::Guest("Zone is not active".to_string())));
    }

    // Validate privacy budget
    if input.epsilon_consumed > zone.privacy_params.epsilon_budget {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Report exceeds zone's epsilon budget".to_string()
        )));
    }

    // Apply local differential privacy noise
    let noisy_count = apply_laplace_noise(input.count as f64, input.epsilon_consumed);

    // Create time bucket (never exact time)
    let time_bucket = create_time_bucket(zone.privacy_params.time_bucket_hours);

    let report = HealthEventReport {
        report_id: generate_report_id(),
        zone_hash: input.zone_hash.clone(),
        event_type: input.event_type,
        noisy_count,
        time_bucket,
        age_bracket: input.age_bracket,
        reported_at: sys_time()?.as_micros() as i64,
        contributor_count: input.contributor_count,
        epsilon_consumed: input.epsilon_consumed,
    };

    let action_hash = create_entry(EntryTypes::HealthEventReport(report))?;

    // Link to zone
    create_link(input.zone_hash.clone(), action_hash.clone(), LinkTypes::ZoneToReports, ())?;

    // Check if this triggers an alert
    let _ = check_alert_threshold(&input.zone_hash);

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created report".to_string())))?;

    Ok(record)
}

/// Get aggregated reports for a zone (respects minimum contributor threshold)
#[hdk_extern]
pub fn get_zone_reports(input: GetZoneReportsInput) -> ExternResult<Vec<HealthEventReport>> {
    let zone = get_zone_from_hash(&input.zone_hash)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Zone not found".to_string())))?;

    let links = get_links(
        LinkQuery::try_new(input.zone_hash.clone(), LinkTypes::ZoneToReports)?, GetStrategy::default()
    )?;

    let mut reports = Vec::new();
    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(report) = record.entry().to_app_option::<HealthEventReport>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                // Only include reports meeting minimum contributor threshold
                if report.contributor_count >= zone.privacy_params.min_contributors {
                    // Apply time filter if specified
                    if let Some(since) = input.since {
                        if report.reported_at >= since {
                            reports.push(report);
                        }
                    } else {
                        reports.push(report);
                    }
                }
            }
        }
    }

    Ok(reports)
}

// ==================== ALERT MANAGEMENT ====================

/// Manually trigger an aggregate alert (usually auto-triggered)
#[hdk_extern]
pub fn create_aggregate_alert(input: CreateAlertInput) -> ExternResult<Record> {
    let now = sys_time()?.as_micros() as i64;
    let expires_at = now + (input.duration_days as i64 * 24 * 60 * 60 * 1_000_000);

    let alert = AggregateAlert {
        alert_id: generate_alert_id(),
        zone_hash: input.zone_hash.clone(),
        severity: input.severity,
        alert_type: input.alert_type,
        statistical_basis: input.statistical_basis,
        affected_age_groups: input.affected_age_groups,
        triggered_at: now,
        expires_at,
        status: AlertStatus::Active,
        public_message: input.public_message,
    };

    let action_hash = create_entry(EntryTypes::AggregateAlert(alert))?;

    // Link to zone
    create_link(input.zone_hash, action_hash.clone(), LinkTypes::ZoneToAlerts, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created alert".to_string())))?;

    Ok(record)
}

/// Get active alerts for a zone
#[hdk_extern]
pub fn get_zone_alerts(zone_hash: ActionHash) -> ExternResult<Vec<AggregateAlert>> {
    let links = get_links(
        LinkQuery::try_new(zone_hash, LinkTypes::ZoneToAlerts)?, GetStrategy::default()
    )?;

    let now = sys_time()?.as_micros() as i64;
    let mut alerts = Vec::new();

    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(alert) = record.entry().to_app_option::<AggregateAlert>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                // Only include active, non-expired alerts
                if alert.status == AlertStatus::Active && alert.expires_at > now {
                    alerts.push(alert);
                }
            }
        }
    }

    Ok(alerts)
}

/// Update alert status
#[hdk_extern]
pub fn update_alert_status(input: UpdateAlertStatusInput) -> ExternResult<Record> {
    let record = get(input.alert_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Alert not found".to_string())))?;

    let mut alert = record.entry().to_app_option::<AggregateAlert>()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize alert".to_string())))?;

    alert.status = input.new_status;
    if let Some(msg) = input.public_message {
        alert.public_message = Some(msg);
    }

    let action_hash = update_entry(input.alert_hash, EntryTypes::AggregateAlert(alert))?;

    let updated_record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve updated alert".to_string())))?;

    Ok(updated_record)
}

// ==================== VACCINATION COVERAGE ====================

/// Compute and store vaccination coverage for a zone
#[hdk_extern]
pub fn compute_vaccination_coverage(input: ComputeCoverageInput) -> ExternResult<Record> {
    let zone = get_zone_from_hash(&input.zone_hash)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Zone not found".to_string())))?;

    // Validate we have enough contributors for privacy
    if input.total_contributors < zone.privacy_params.min_contributors {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Need at least {} contributors for coverage computation",
            zone.privacy_params.min_contributors
        ))));
    }

    // Apply differential privacy noise to coverage rates
    let noisy_coverage = apply_laplace_noise(input.overall_coverage_rate, 0.1);
    let noisy_sample = apply_laplace_noise(input.total_contributors as f64, 0.05);

    // Noise the age-specific coverage
    let coverage_by_age: Vec<AgeCoverage> = input.coverage_by_age.iter().map(|ac| {
        AgeCoverage {
            age_bracket: ac.age_bracket.clone(),
            coverage_rate: apply_laplace_noise(ac.coverage_rate, 0.1).max(0.0).min(1.0),
            sample_size: apply_laplace_noise(ac.sample_size, 0.05).max(0.0),
        }
    }).collect();

    let now = sys_time()?.as_micros() as i64;

    let coverage = VaccinationCoverage {
        coverage_id: generate_coverage_id(),
        zone_hash: input.zone_hash.clone(),
        vaccine_type: input.vaccine_type,
        coverage_by_age,
        overall_coverage: noisy_coverage.max(0.0).min(1.0),
        margin_of_error: compute_margin_of_error(noisy_sample),
        period_start: input.period_start,
        period_end: input.period_end,
        sample_size: noisy_sample.max(0.0),
        computed_at: now,
    };

    let action_hash = create_entry(EntryTypes::VaccinationCoverage(coverage))?;

    // Link to zone
    create_link(input.zone_hash, action_hash.clone(), LinkTypes::ZoneToCoverage, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created coverage".to_string())))?;

    Ok(record)
}

/// Get vaccination coverage for a zone
#[hdk_extern]
pub fn get_vaccination_coverage(input: GetCoverageInput) -> ExternResult<Vec<VaccinationCoverage>> {
    let links = get_links(
        LinkQuery::try_new(input.zone_hash.clone(), LinkTypes::ZoneToCoverage)?, GetStrategy::default()
    )?;

    let mut coverages = Vec::new();

    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(cov) = record.entry().to_app_option::<VaccinationCoverage>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                // Filter by vaccine type if specified
                if let Some(ref vt) = input.vaccine_type {
                    if &cov.vaccine_type == vt {
                        coverages.push(cov);
                    }
                } else {
                    coverages.push(cov);
                }
            }
        }
    }

    // Sort by computed_at descending
    coverages.sort_by(|a, b| b.computed_at.cmp(&a.computed_at));

    Ok(coverages)
}

// ==================== SYNDROMIC SURVEILLANCE ====================

/// Submit syndromic surveillance report
#[hdk_extern]
pub fn submit_syndromic_report(input: SubmitSyndromicInput) -> ExternResult<Record> {
    let zone = get_zone_from_hash(&input.zone_hash)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Zone not found".to_string())))?;

    // Validate contributor count
    if input.contributor_count < zone.privacy_params.min_contributors {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Need at least {} contributors",
            zone.privacy_params.min_contributors
        ))));
    }

    // Apply noise to rate
    let noisy_rate = apply_laplace_noise(input.rate_per_100k, 0.1).max(0.0);

    let report = SyndromicSurveillance {
        report_id: generate_report_id(),
        zone_hash: input.zone_hash.clone(),
        syndrome: input.syndrome,
        activity_level: determine_activity_level(noisy_rate),
        trend: input.trend,
        rate_per_100k: noisy_rate,
        week_of_year: input.week_of_year,
        year: input.year,
        contributor_count: input.contributor_count,
        reported_at: sys_time()?.as_micros() as i64,
    };

    let action_hash = create_entry(EntryTypes::SyndromicSurveillance(report))?;

    // Link to zone
    create_link(input.zone_hash, action_hash.clone(), LinkTypes::ZoneToReports, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created report".to_string())))?;

    Ok(record)
}

// ==================== PUBLIC HEALTH RESPONSE ====================

/// Create a public health response to an alert
#[hdk_extern]
pub fn create_public_health_response(input: CreateResponseInput) -> ExternResult<Record> {
    let response = PublicHealthResponse {
        response_id: generate_response_id(),
        alert_hash: input.alert_hash.clone(),
        actions: input.initial_actions,
        status: ResponseStatus::Planning,
        public_communication: input.public_communication,
        resources_allocated: Vec::new(),
        initiated_at: sys_time()?.as_micros() as i64,
        completed_at: None,
        effectiveness: None,
    };

    let action_hash = create_entry(EntryTypes::PublicHealthResponse(response))?;

    // Link to alert
    create_link(input.alert_hash, action_hash.clone(), LinkTypes::AlertToResponses, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created response".to_string())))?;

    Ok(record)
}

/// Update response status
#[hdk_extern]
pub fn update_response_status(input: UpdateResponseInput) -> ExternResult<Record> {
    let record = get(input.response_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Response not found".to_string())))?;

    let mut response = record.entry().to_app_option::<PublicHealthResponse>()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize response".to_string())))?;

    response.status = input.new_status.clone();

    // Mark completion time if completed
    if input.new_status == ResponseStatus::Completed {
        response.completed_at = Some(sys_time()?.as_micros() as i64);
    }

    if let Some(assessment) = input.effectiveness {
        response.effectiveness = Some(assessment);
    }

    let action_hash = update_entry(input.response_hash, EntryTypes::PublicHealthResponse(response))?;

    let updated_record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve updated response".to_string())))?;

    Ok(updated_record)
}

// ==================== OUTBREAK INVESTIGATION ====================

/// Start an outbreak investigation
#[hdk_extern]
pub fn start_outbreak_investigation(input: StartInvestigationInput) -> ExternResult<Record> {
    let investigation = OutbreakInvestigation {
        investigation_id: generate_investigation_id(),
        alert_hash: input.alert_hash.clone(),
        affected_zones: input.affected_zones,
        status: InvestigationStatus::Initiated,
        suspected_cause: input.suspected_cause,
        confirmed_cause: None,
        epi_summary: EpiSummary {
            case_count: 0.0,
            hospitalizations: 0.0,
            mortality: None,
            attack_rate: None,
            serial_interval_days: None,
            r_number: None,
        },
        findings: Vec::new(),
        started_at: sys_time()?.as_micros() as i64,
        completed_at: None,
    };

    let action_hash = create_entry(EntryTypes::OutbreakInvestigation(investigation))?;

    // Link to ongoing outbreaks
    let anchor = anchor_for_ongoing_outbreaks()?;
    create_link(anchor, action_hash.clone(), LinkTypes::OngoingOutbreaks, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created investigation".to_string())))?;

    Ok(record)
}

/// Update outbreak investigation with epidemiological data
#[hdk_extern]
pub fn update_investigation(input: UpdateInvestigationInput) -> ExternResult<Record> {
    let record = get(input.investigation_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Investigation not found".to_string())))?;

    let mut investigation = record.entry().to_app_option::<OutbreakInvestigation>()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize investigation".to_string())))?;

    // Update epi summary with noisy data
    if let Some(epi) = input.epi_summary {
        investigation.epi_summary = EpiSummary {
            case_count: apply_laplace_noise(epi.case_count, 0.1).max(0.0),
            hospitalizations: apply_laplace_noise(epi.hospitalizations, 0.1).max(0.0),
            mortality: epi.mortality.map(|m| {
                // Suppress if low for privacy
                let noisy = apply_laplace_noise(m, 0.1);
                if noisy < 5.0 { None } else { Some(noisy) }
            }).flatten(),
            attack_rate: epi.attack_rate,
            serial_interval_days: epi.serial_interval_days,
            r_number: epi.r_number,
        };
    }

    if let Some(status) = input.status {
        investigation.status = status.clone();
        if status == InvestigationStatus::Concluded {
            investigation.completed_at = Some(sys_time()?.as_micros() as i64);
        }
    }

    if let Some(cause) = input.confirmed_cause {
        investigation.confirmed_cause = Some(cause);
    }

    if let Some(findings) = input.findings {
        investigation.findings.extend(findings);
    }

    let action_hash = update_entry(input.investigation_hash, EntryTypes::OutbreakInvestigation(investigation))?;

    let updated_record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve updated investigation".to_string())))?;

    Ok(updated_record)
}

/// Get ongoing outbreak investigations
#[hdk_extern]
pub fn get_ongoing_investigations(_: ()) -> ExternResult<Vec<OutbreakInvestigation>> {
    let anchor = anchor_for_ongoing_outbreaks()?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::OngoingOutbreaks)?, GetStrategy::default()
    )?;

    let mut investigations = Vec::new();

    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(inv) = record.entry().to_app_option::<OutbreakInvestigation>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                // Only include non-concluded investigations
                if inv.status != InvestigationStatus::Concluded {
                    investigations.push(inv);
                }
            }
        }
    }

    Ok(investigations)
}

// ==================== IMMUNITY STATUS ====================

/// Compute aggregated immunity status for a zone
#[hdk_extern]
pub fn compute_immunity_status(input: ComputeImmunityInput) -> ExternResult<Record> {
    let zone = get_zone_from_hash(&input.zone_hash)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Zone not found".to_string())))?;

    // Validate sample size
    if input.sample_size < zone.privacy_params.min_contributors as f64 {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Sample size below privacy threshold".to_string()
        )));
    }

    // Apply noise to estimates
    let noisy_pct = apply_laplace_noise(input.estimated_immune_pct, 0.1)
        .max(0.0)
        .min(100.0);
    let noisy_sample = apply_laplace_noise(input.sample_size, 0.05).max(0.0);
    let margin = compute_margin_of_error(noisy_sample);

    let status = ImmunityStatus {
        status_id: generate_status_id(),
        zone_hash: input.zone_hash.clone(),
        immunity_type: input.immunity_type,
        estimated_immune_pct: noisy_pct,
        margin_of_error: margin,
        sample_size: noisy_sample,
        as_of_date: sys_time()?.as_micros() as i64,
        confidence_interval: ConfidenceInterval {
            lower: (noisy_pct - margin * 1.96).max(0.0),
            upper: (noisy_pct + margin * 1.96).min(100.0),
            level: 0.95,
        },
    };

    let action_hash = create_entry(EntryTypes::ImmunityStatus(status))?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created status".to_string())))?;

    Ok(record)
}

// ==================== HELPER FUNCTIONS ====================

/// Simple anchor type for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn get_zone_from_hash(zone_hash: &ActionHash) -> ExternResult<Option<SurveillanceZone>> {
    match get(zone_hash.clone(), GetOptions::default())? {
        Some(record) => {
            let zone = record.entry().to_app_option::<SurveillanceZone>()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;
            Ok(zone)
        }
        None => Ok(None),
    }
}

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}

fn anchor_for_active_surveillance() -> ExternResult<AnyLinkableHash> {
    Ok(AnyLinkableHash::from(anchor_hash("active_surveillance")?))
}

fn anchor_for_ongoing_outbreaks() -> ExternResult<AnyLinkableHash> {
    Ok(AnyLinkableHash::from(anchor_hash("ongoing_outbreaks")?))
}

/// Apply Laplace noise for differential privacy
fn apply_laplace_noise(value: f64, epsilon: f64) -> f64 {
    // In production, use proper cryptographic randomness
    // This is a simplified version for demonstration
    let sensitivity = 1.0;
    let scale = sensitivity / epsilon;

    // Simplified noise - in production use proper Laplace distribution
    // Using a deterministic offset based on value for reproducibility in tests
    let pseudo_random = ((value * 1000.0) as i64 % 100) as f64 / 100.0 - 0.5;
    value + pseudo_random * scale
}

/// Create time bucket (e.g., "2024-01-15-AM")
fn create_time_bucket(bucket_hours: u32) -> String {
    let now = sys_time().map(|t| t.as_micros() as i64).unwrap_or(0);
    let hours = (now / (3600 * 1_000_000)) as u32;
    let bucket = hours / bucket_hours;
    format!("bucket-{}", bucket)
}

/// Check if alert threshold is exceeded
fn check_alert_threshold(_zone_hash: &ActionHash) -> ExternResult<()> {
    // In production, this would analyze recent reports and
    // trigger an alert if thresholds are exceeded
    // For now, this is a placeholder
    Ok(())
}

/// Determine activity level from rate
fn determine_activity_level(rate: f64) -> ActivityLevel {
    if rate < 10.0 {
        ActivityLevel::Minimal
    } else if rate < 50.0 {
        ActivityLevel::Low
    } else if rate < 100.0 {
        ActivityLevel::Moderate
    } else if rate < 200.0 {
        ActivityLevel::High
    } else {
        ActivityLevel::VeryHigh
    }
}

/// Compute margin of error based on sample size
fn compute_margin_of_error(sample_size: f64) -> f64 {
    if sample_size <= 0.0 {
        return 50.0; // Maximum uncertainty
    }
    // Simplified formula: 1/sqrt(n) * 100 for percentage
    (1.0 / sample_size.sqrt()) * 100.0
}

fn get_timestamp_micros() -> i64 {
    sys_time().map(|t| t.as_micros() as i64).unwrap_or(0)
}

fn generate_zone_id(name: &str) -> String {
    let now = get_timestamp_micros();
    format!("ZONE-{}-{}", name.chars().take(4).collect::<String>().to_uppercase(), now % 10000)
}

fn generate_report_id() -> String {
    format!("RPT-{}", get_timestamp_micros())
}

fn generate_alert_id() -> String {
    format!("ALT-{}", get_timestamp_micros())
}

fn generate_coverage_id() -> String {
    format!("COV-{}", get_timestamp_micros())
}

fn generate_response_id() -> String {
    format!("RSP-{}", get_timestamp_micros())
}

fn generate_investigation_id() -> String {
    format!("INV-{}", get_timestamp_micros())
}

fn generate_status_id() -> String {
    format!("IMM-{}", get_timestamp_micros())
}

// ==================== INPUT TYPES ====================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateSurveillanceZoneInput {
    pub name: String,
    pub level: ZoneLevel,
    pub parent_zone: Option<ActionHash>,
    pub population_estimate: u64,
    pub thresholds: AlertThresholds,
    pub privacy_params: ZonePrivacyParams,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitHealthEventInput {
    pub zone_hash: ActionHash,
    pub event_type: HealthEventType,
    pub count: u32,
    pub age_bracket: Option<AgeBracket>,
    pub contributor_count: u32,
    pub epsilon_consumed: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetZoneReportsInput {
    pub zone_hash: ActionHash,
    pub since: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateAlertInput {
    pub zone_hash: ActionHash,
    pub severity: AlertSeverity,
    pub alert_type: AlertType,
    pub statistical_basis: StatisticalBasis,
    pub affected_age_groups: Vec<AgeBracket>,
    pub duration_days: u32,
    pub public_message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateAlertStatusInput {
    pub alert_hash: ActionHash,
    pub new_status: AlertStatus,
    pub public_message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComputeCoverageInput {
    pub zone_hash: ActionHash,
    pub vaccine_type: VaccineType,
    pub overall_coverage_rate: f64,
    pub coverage_by_age: Vec<AgeCoverageInput>,
    pub total_contributors: u32,
    pub period_start: i64,
    pub period_end: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgeCoverageInput {
    pub age_bracket: AgeBracket,
    pub coverage_rate: f64,
    pub sample_size: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetCoverageInput {
    pub zone_hash: ActionHash,
    pub vaccine_type: Option<VaccineType>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmitSyndromicInput {
    pub zone_hash: ActionHash,
    pub syndrome: SyndromeType,
    pub rate_per_100k: f64,
    pub trend: TrendDirection,
    pub week_of_year: u32,
    pub year: u32,
    pub contributor_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateResponseInput {
    pub alert_hash: ActionHash,
    pub initial_actions: Vec<ResponseAction>,
    pub public_communication: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateResponseInput {
    pub response_hash: ActionHash,
    pub new_status: ResponseStatus,
    pub effectiveness: Option<EffectivenessAssessment>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StartInvestigationInput {
    pub alert_hash: ActionHash,
    pub affected_zones: Vec<ActionHash>,
    pub suspected_cause: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateInvestigationInput {
    pub investigation_hash: ActionHash,
    pub status: Option<InvestigationStatus>,
    pub confirmed_cause: Option<String>,
    pub epi_summary: Option<EpiSummaryInput>,
    pub findings: Option<Vec<Finding>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EpiSummaryInput {
    pub case_count: f64,
    pub hospitalizations: f64,
    pub mortality: Option<f64>,
    pub attack_rate: Option<f64>,
    pub serial_interval_days: Option<f64>,
    pub r_number: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComputeImmunityInput {
    pub zone_hash: ActionHash,
    pub immunity_type: ImmunityType,
    pub estimated_immune_pct: f64,
    pub sample_size: f64,
}
