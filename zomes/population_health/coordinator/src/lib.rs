//! Population Health Analytics Coordinator Zome
//!
//! Provides functions for population-level health analytics
//! with differential privacy protections for aggregate statistics.

use hdk::prelude::*;
use population_health_integrity::*;

/// Input for creating a population statistic
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateStatisticInput {
    pub statistic_id: String,
    pub metric_type: MetricType,
    pub condition_code: String,
    pub condition_name: String,
    pub region_id: String,
    pub geographic_level: GeographicLevel,
    pub period_start: Timestamp,
    pub period_end: Timestamp,
    pub time_granularity: TimeGranularity,
    pub value: String,
    pub unit: String,
    pub ci_lower: Option<String>,
    pub ci_upper: Option<String>,
    pub denominator: u32,
    pub dp_applied: bool,
    pub epsilon: Option<String>,
    pub source_count: u32,
}

/// Input for creating a health indicator
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateIndicatorInput {
    pub indicator_id: String,
    pub name: String,
    pub description: String,
    pub region_id: String,
    pub geographic_level: GeographicLevel,
    pub period: String,
    pub score: u32,
    pub rank: Option<u32>,
    pub peer_group_size: Option<u32>,
    pub components: String,
    pub trend: i32,
    pub benchmark_comparison: String,
}

/// Input for creating a surveillance report
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSurveillanceInput {
    pub report_id: String,
    pub condition_code: String,
    pub condition_name: String,
    pub period_start: Timestamp,
    pub period_end: Timestamp,
    pub region_id: String,
    pub geographic_level: GeographicLevel,
    pub case_count: u32,
    pub expected_count: u32,
    pub age_distribution: Option<String>,
    pub gender_distribution: Option<String>,
    pub notes: Option<String>,
}

/// Input for creating an alert
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateAlertInput {
    pub alert_id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub condition_code: String,
    pub condition_name: String,
    pub region_id: String,
    pub geographic_level: GeographicLevel,
    pub title: String,
    pub description: String,
    pub supporting_data: String,
    pub recommendations: Vec<String>,
    pub expires_at: Option<Timestamp>,
}

/// Input for disparity analysis
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateDisparityInput {
    pub analysis_id: String,
    pub metric_type: MetricType,
    pub condition_code: String,
    pub reference_group: String,
    pub comparison_group: String,
    pub stratification: String,
    pub region_id: String,
    pub period: String,
    pub reference_value: String,
    pub comparison_value: String,
    pub absolute_difference: String,
    pub relative_difference: String,
    pub p_value: Option<String>,
    pub difference_ci: Option<String>,
    pub trend: String,
    pub dp_applied: bool,
}

/// Input for quality indicator
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateQualityInput {
    pub indicator_id: String,
    pub measure_name: String,
    pub measure_code: String,
    pub domain: String,
    pub region_id: String,
    pub period: String,
    pub numerator: u32,
    pub denominator: u32,
    pub benchmark: Option<String>,
    pub percentile: Option<u32>,
    pub star_rating: Option<u32>,
    pub trend: i32,
    pub dp_applied: bool,
}

/// Input for data contribution
#[derive(Serialize, Deserialize, Debug)]
pub struct RecordContributionInput {
    pub contribution_id: String,
    pub data_type: String,
    pub record_count: u32,
    pub period_start: Timestamp,
    pub period_end: Timestamp,
    pub quality_score: u32,
    pub completeness: u32,
    pub epsilon_consumed: Option<String>,
}

/// Query parameters for statistics
#[derive(Serialize, Deserialize, Debug)]
pub struct StatisticQuery {
    pub region_id: Option<String>,
    pub condition_code: Option<String>,
    pub metric_type: Option<MetricType>,
    pub period_start: Option<Timestamp>,
    pub period_end: Option<Timestamp>,
}

/// Create a new population statistic
#[hdk_extern]
pub fn create_population_statistic(input: CreateStatisticInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let statistic = PopulationStatistic {
        statistic_id: input.statistic_id,
        metric_type: input.metric_type,
        condition_code: input.condition_code.clone(),
        condition_name: input.condition_name,
        region_id: input.region_id.clone(),
        geographic_level: input.geographic_level,
        period_start: input.period_start,
        period_end: input.period_end,
        time_granularity: input.time_granularity,
        value: input.value,
        unit: input.unit,
        ci_lower: input.ci_lower,
        ci_upper: input.ci_upper,
        denominator: input.denominator,
        dp_applied: input.dp_applied,
        epsilon: input.epsilon,
        source_count: input.source_count,
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::PopulationStatistic(statistic))?;

    // Link from all statistics anchor
    let all_anchor = anchor_hash("all_pop_statistics")?;
    create_link(
        all_anchor,
        action_hash.clone(),
        LinkTypes::AllStatistics,
        (),
    )?;

    // Link by region
    let region_anchor = anchor_hash(&format!("region_{}", input.region_id))?;
    create_link(
        region_anchor,
        action_hash.clone(),
        LinkTypes::StatisticsByRegion,
        (),
    )?;

    // Link by condition
    let condition_anchor = anchor_hash(&format!("condition_{}", input.condition_code))?;
    create_link(
        condition_anchor,
        action_hash.clone(),
        LinkTypes::StatisticsByCondition,
        (),
    )?;

    Ok(action_hash)
}

/// Get statistics for a region
#[hdk_extern]
pub fn get_region_statistics(region_id: String) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("region_{}", region_id))?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::StatisticsByRegion)?, GetStrategy::default())?;

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

/// Get statistics for a condition
#[hdk_extern]
pub fn get_condition_statistics(condition_code: String) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("condition_{}", condition_code))?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::StatisticsByCondition)?, GetStrategy::default())?;

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

/// Create a health indicator
#[hdk_extern]
pub fn create_health_indicator(input: CreateIndicatorInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let indicator = HealthIndicator {
        indicator_id: input.indicator_id,
        name: input.name,
        description: input.description,
        region_id: input.region_id.clone(),
        geographic_level: input.geographic_level,
        period: input.period,
        score: input.score,
        rank: input.rank,
        peer_group_size: input.peer_group_size,
        components: input.components,
        trend: input.trend,
        benchmark_comparison: input.benchmark_comparison,
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::HealthIndicator(indicator))?;

    // Link by region
    let region_anchor = anchor_hash(&format!("indicators_{}", input.region_id))?;
    create_link(
        region_anchor,
        action_hash.clone(),
        LinkTypes::IndicatorsByRegion,
        (),
    )?;

    Ok(action_hash)
}

/// Get health indicators for a region
#[hdk_extern]
pub fn get_region_indicators(region_id: String) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("indicators_{}", region_id))?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::IndicatorsByRegion)?, GetStrategy::default())?;

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

/// Create a surveillance report
#[hdk_extern]
pub fn create_surveillance_report(input: CreateSurveillanceInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    // Calculate ratio
    let ratio = if input.expected_count > 0 {
        format!("{:.2}", input.case_count as f64 / input.expected_count as f64)
    } else {
        "N/A".to_string()
    };

    // Determine if alert should be triggered (simple threshold)
    let ratio_val: f64 = ratio.parse().unwrap_or(0.0);
    let (alert_triggered, alert_severity) = if ratio_val > 2.0 {
        (true, Some(AlertSeverity::Critical))
    } else if ratio_val > 1.5 {
        (true, Some(AlertSeverity::Warning))
    } else if ratio_val > 1.2 {
        (true, Some(AlertSeverity::Info))
    } else {
        (false, None)
    };

    let report = SurveillanceReport {
        report_id: input.report_id,
        condition_code: input.condition_code.clone(),
        condition_name: input.condition_name.clone(),
        period_start: input.period_start,
        period_end: input.period_end,
        region_id: input.region_id.clone(),
        geographic_level: input.geographic_level.clone(),
        case_count: input.case_count,
        expected_count: input.expected_count,
        ratio,
        alert_triggered,
        alert_severity: alert_severity.clone(),
        age_distribution: input.age_distribution,
        gender_distribution: input.gender_distribution,
        notes: input.notes,
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::SurveillanceReport(report))?;

    // Link by condition
    let condition_anchor = anchor_hash(&format!("surveillance_{}", input.condition_code))?;
    create_link(
        condition_anchor,
        action_hash.clone(),
        LinkTypes::SurveillanceByCondition,
        (),
    )?;

    // Create alert if triggered
    if alert_triggered {
        let alert_input = CreateAlertInput {
            alert_id: format!("auto-{}", now.as_micros()),
            alert_type: AlertType::ThresholdBreached,
            severity: alert_severity.unwrap_or(AlertSeverity::Warning),
            condition_code: input.condition_code,
            condition_name: input.condition_name,
            region_id: input.region_id,
            geographic_level: input.geographic_level,
            title: format!("Elevated {} cases detected", input.case_count),
            description: format!(
                "Observed {} cases vs {} expected (ratio: {:.2})",
                input.case_count, input.expected_count, ratio_val
            ),
            supporting_data: "{}".to_string(),
            recommendations: vec!["Monitor closely".to_string(), "Review data sources".to_string()],
            expires_at: None,
        };
        create_public_health_alert(alert_input)?;
    }

    Ok(action_hash)
}

/// Get surveillance reports for a condition
#[hdk_extern]
pub fn get_condition_surveillance(condition_code: String) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("surveillance_{}", condition_code))?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::SurveillanceByCondition)?, GetStrategy::default())?;

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

/// Create a public health alert
#[hdk_extern]
pub fn create_public_health_alert(input: CreateAlertInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let alert = PublicHealthAlert {
        alert_id: input.alert_id,
        alert_type: input.alert_type,
        severity: input.severity,
        condition_code: input.condition_code,
        condition_name: input.condition_name,
        region_id: input.region_id.clone(),
        geographic_level: input.geographic_level,
        title: input.title,
        description: input.description,
        supporting_data: input.supporting_data,
        recommendations: input.recommendations,
        issued_at: now,
        expires_at: input.expires_at,
        acknowledged: false,
        acknowledged_by: None,
        acknowledged_at: None,
    };

    let action_hash = create_entry(EntryTypes::PublicHealthAlert(alert))?;

    // Link from active alerts anchor
    let active_anchor = anchor_hash("active_alerts")?;
    create_link(
        active_anchor,
        action_hash.clone(),
        LinkTypes::ActiveAlerts,
        (),
    )?;

    // Link by region
    let region_anchor = anchor_hash(&format!("alerts_{}", input.region_id))?;
    create_link(
        region_anchor,
        action_hash.clone(),
        LinkTypes::AlertsByRegion,
        (),
    )?;

    Ok(action_hash)
}

/// Get active alerts
#[hdk_extern]
pub fn get_active_alerts(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("active_alerts")?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::ActiveAlerts)?, GetStrategy::default())?;

    let now = sys_time()?;
    let mut records = Vec::new();

    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                // Filter out acknowledged and expired alerts
                let alert: PublicHealthAlert = record
                    .entry()
                    .to_app_option()
                    .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
                    .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid alert".to_string())))?;

                if !alert.acknowledged {
                    if let Some(expires) = alert.expires_at {
                        if expires > now {
                            records.push(record);
                        }
                    } else {
                        records.push(record);
                    }
                }
            }
        }
    }

    Ok(records)
}

/// Acknowledge an alert
#[hdk_extern]
pub fn acknowledge_alert(alert_hash: ActionHash) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let acknowledger = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let record = get(alert_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Alert not found".to_string())))?;

    let mut alert: PublicHealthAlert = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid alert entry".to_string())))?;

    alert.acknowledged = true;
    alert.acknowledged_by = Some(acknowledger);
    alert.acknowledged_at = Some(now);

    update_entry(alert_hash, alert)
}

/// Create a disparity analysis
#[hdk_extern]
pub fn create_disparity_analysis(input: CreateDisparityInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let analysis = DisparityAnalysis {
        analysis_id: input.analysis_id,
        metric_type: input.metric_type,
        condition_code: input.condition_code,
        reference_group: input.reference_group,
        comparison_group: input.comparison_group,
        stratification: input.stratification,
        region_id: input.region_id,
        period: input.period,
        reference_value: input.reference_value,
        comparison_value: input.comparison_value,
        absolute_difference: input.absolute_difference,
        relative_difference: input.relative_difference,
        p_value: input.p_value,
        difference_ci: input.difference_ci,
        trend: input.trend,
        dp_applied: input.dp_applied,
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::DisparityAnalysis(analysis))?;

    // Link from disparities anchor
    let anchor = anchor_hash("disparity_analyses")?;
    create_link(
        anchor,
        action_hash.clone(),
        LinkTypes::DisparityAnalyses,
        (),
    )?;

    Ok(action_hash)
}

/// Get all disparity analyses
#[hdk_extern]
pub fn get_disparity_analyses(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("disparity_analyses")?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::DisparityAnalyses)?, GetStrategy::default())?;

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

/// Create a quality indicator
#[hdk_extern]
pub fn create_quality_indicator(input: CreateQualityInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    // Calculate rate
    let rate = if input.denominator > 0 {
        format!("{:.1}", (input.numerator as f64 / input.denominator as f64) * 100.0)
    } else {
        "0.0".to_string()
    };

    let indicator = QualityIndicator {
        indicator_id: input.indicator_id,
        measure_name: input.measure_name,
        measure_code: input.measure_code,
        domain: input.domain,
        region_id: input.region_id.clone(),
        period: input.period,
        numerator: input.numerator,
        denominator: input.denominator,
        rate,
        benchmark: input.benchmark,
        percentile: input.percentile,
        star_rating: input.star_rating,
        trend: input.trend,
        dp_applied: input.dp_applied,
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::QualityIndicator(indicator))?;

    // Link by region
    let region_anchor = anchor_hash(&format!("quality_{}", input.region_id))?;
    create_link(
        region_anchor,
        action_hash.clone(),
        LinkTypes::QualityByRegion,
        (),
    )?;

    Ok(action_hash)
}

/// Get quality indicators for a region
#[hdk_extern]
pub fn get_region_quality(region_id: String) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("quality_{}", region_id))?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::QualityByRegion)?, GetStrategy::default())?;

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

/// Record a data contribution
#[hdk_extern]
pub fn record_data_contribution(input: RecordContributionInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let source_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let contribution = DataContribution {
        contribution_id: input.contribution_id,
        source_hash: source_hash.clone(),
        data_type: input.data_type,
        record_count: input.record_count,
        period_start: input.period_start,
        period_end: input.period_end,
        quality_score: input.quality_score,
        completeness: input.completeness,
        epsilon_consumed: input.epsilon_consumed,
        contributed_at: now,
    };

    let action_hash = create_entry(EntryTypes::DataContribution(contribution))?;

    // Link from source
    create_link(
        source_hash,
        action_hash.clone(),
        LinkTypes::ContributionsBySource,
        (),
    )?;

    Ok(action_hash)
}

/// Get contributions from current source
#[hdk_extern]
pub fn get_my_contributions(_: ()) -> ExternResult<Vec<Record>> {
    let agent = agent_info()?.agent_initial_pubkey;
    let source_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let links = get_links(LinkQuery::try_new(source_hash, LinkTypes::ContributionsBySource)?, GetStrategy::default())?;

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

// Helper function to create anchor hash
/// Anchor for linking entries
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor: &str) -> ExternResult<AnyLinkableHash> {
    let anchor = Anchor(anchor.to_string());
    Ok(hash_entry(&anchor)?.into())
}
