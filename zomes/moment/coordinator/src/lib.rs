//! Universal Health Moment Coordinator Zome
//!
//! Provides global health context awareness enabling:
//! - Real-time understanding of community health
//! - Personalized health context based on location and demographics
//! - Proactive health recommendations
//! - Connection between individual wellness and collective patterns
//!
//! Philosophy: "Know your health moment" - Awareness leads to better decisions

use hdk::prelude::*;
use moment_integrity::*;

// ==================== HEALTH MOMENT MANAGEMENT ====================

/// Create or update a health moment for a region
#[hdk_extern]
pub fn create_health_moment(input: CreateHealthMomentInput) -> ExternResult<Record> {
    let now = sys_time()?.as_micros() as i64;

    let moment = HealthMoment {
        moment_id: generate_moment_id(&input.region),
        region: input.region.clone(),
        timestamp: now,
        conditions: input.conditions,
        environmental: input.environmental,
        community_indicators: input.community_indicators,
        active_advisories: input.active_advisory_ids,
        seasonal_context: input.seasonal_context,
        data_quality: input.data_quality,
    };

    let action_hash = create_entry(EntryTypes::HealthMoment(moment))?;

    // Link to region
    let region_anchor = anchor_for_region(&input.region)?;
    create_link(region_anchor, action_hash.clone(), LinkTypes::RegionToMoments, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created moment".to_string())))?;

    Ok(record)
}

/// Get the current health moment for a region
#[hdk_extern]
pub fn get_health_moment(region: RegionIdentifier) -> ExternResult<Option<HealthMoment>> {
    let region_anchor = anchor_for_region(&region)?;
    let links = get_links(
        LinkQuery::try_new(region_anchor, LinkTypes::RegionToMoments)?, GetStrategy::default()
    )?;

    // Get the most recent moment
    let mut latest_moment: Option<HealthMoment> = None;
    let mut latest_timestamp: i64 = 0;

    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(moment) = record.entry().to_app_option::<HealthMoment>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                if moment.timestamp > latest_timestamp {
                    latest_timestamp = moment.timestamp;
                    latest_moment = Some(moment);
                }
            }
        }
    }

    Ok(latest_moment)
}

/// Get health moment with personalized context
#[hdk_extern]
pub fn get_personalized_moment(context_hash: ActionHash) -> ExternResult<PersonalizedMoment> {
    // Get personal context
    let context = get(context_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Personal context not found".to_string())))?
        .entry()
        .to_app_option::<PersonalContext>()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize context".to_string())))?;

    // Get current moment for their region
    let moment = get_health_moment(context.home_region.clone())?;

    // Get relevant advisories
    let advisories = get_active_advisories_for_region(context.home_region.clone())?;

    // Filter advisories based on personal risk factors and age
    let relevant_advisories: Vec<HealthAdvisory> = advisories.into_iter()
        .filter(|a| {
            // Include if affects their age group or is for all
            a.affected_groups.contains(&context.age_group) ||
            a.affected_groups.contains(&AgeGroup::All)
        })
        .collect();

    // Get recommendations
    let recommendations = get_recommendations_for_region(context.home_region.clone())?;

    // Filter recommendations
    let relevant_recommendations: Vec<WellnessRecommendation> = recommendations.into_iter()
        .filter(|r| {
            r.target_ages.contains(&context.age_group) ||
            r.target_ages.contains(&AgeGroup::All)
        })
        .filter(|r| {
            // Include if matches their conditions or is general prevention
            r.applicable_conditions.is_empty() ||
            r.applicable_conditions.iter().any(|c| context.relevant_conditions.contains(c)) ||
            r.category == RecommendationCategory::Prevention
        })
        .collect();

    Ok(PersonalizedMoment {
        health_moment: moment,
        relevant_advisories,
        relevant_recommendations,
        personalized_risk_level: compute_personalized_risk(&context),
        context_last_updated: context.updated_at,
    })
}

// ==================== SEASONAL PATTERNS ====================

/// Create or update a seasonal pattern
#[hdk_extern]
pub fn create_seasonal_pattern(input: CreateSeasonalPatternInput) -> ExternResult<Record> {
    let pattern = SeasonalPattern {
        pattern_id: generate_pattern_id(&input.region, &input.season),
        region: input.region.clone(),
        season: input.season.clone(),
        typical_conditions: input.typical_conditions,
        environmental_norms: input.environmental_norms,
        baselines: input.baselines,
        updated_at: sys_time()?.as_micros() as i64,
    };

    let action_hash = create_entry(EntryTypes::SeasonalPattern(pattern))?;

    // Link to season anchor
    let season_anchor = anchor_for_season(&input.season)?;
    create_link(season_anchor, action_hash.clone(), LinkTypes::SeasonToPatterns, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created pattern".to_string())))?;

    Ok(record)
}

/// Get seasonal patterns for a region
#[hdk_extern]
pub fn get_seasonal_patterns(input: GetSeasonalPatternsInput) -> ExternResult<Vec<SeasonalPattern>> {
    let season_anchor = anchor_for_season(&input.season)?;
    let links = get_links(
        LinkQuery::try_new(season_anchor, LinkTypes::SeasonToPatterns)?, GetStrategy::default()
    )?;

    let mut patterns = Vec::new();

    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(pattern) = record.entry().to_app_option::<SeasonalPattern>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                // Filter by region if specified
                if pattern.region.country == input.region.country {
                    if input.region.state.is_none() || pattern.region.state == input.region.state {
                        patterns.push(pattern);
                    }
                }
            }
        }
    }

    Ok(patterns)
}

// ==================== ENVIRONMENTAL FACTORS ====================

/// Record an environmental factor reading
#[hdk_extern]
pub fn record_environmental_factor(input: RecordEnvironmentalInput) -> ExternResult<Record> {
    let factor_type_clone = input.factor_type.clone();
    let impact = determine_impact(&factor_type_clone, input.current_value);
    let recommendations = generate_environmental_recommendations(&factor_type_clone, input.current_value);

    let factor = EnvironmentalFactor {
        factor_id: generate_factor_id(&input.region, &factor_type_clone),
        region: input.region.clone(),
        factor_type: input.factor_type,
        current_value: input.current_value,
        unit: input.unit,
        impact,
        forecast: input.forecast,
        recommendations,
        recorded_at: sys_time()?.as_micros() as i64,
    };

    let action_hash = create_entry(EntryTypes::EnvironmentalFactor(factor))?;

    // If impact is unhealthy or worse, create alert link
    if matches!(input.impact, QualityImpact::UnhealthyForSensitive |
                             QualityImpact::Unhealthy |
                             QualityImpact::VeryUnhealthy |
                             QualityImpact::Hazardous) {
        let alert_anchor = anchor_for_environmental_alerts()?;
        create_link(alert_anchor, action_hash.clone(), LinkTypes::EnvironmentalAlerts, ())?;
    }

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created factor".to_string())))?;

    Ok(record)
}

/// Get current environmental factors for a region
#[hdk_extern]
pub fn get_environmental_factors(region: RegionIdentifier) -> ExternResult<Vec<EnvironmentalFactor>> {
    // For now, return factors from environmental alerts
    // In production, would have region-specific queries
    let alert_anchor = anchor_for_environmental_alerts()?;
    let links = get_links(
        LinkQuery::try_new(alert_anchor, LinkTypes::EnvironmentalAlerts)?, GetStrategy::default()
    )?;

    let cutoff = sys_time()?.as_micros() as i64 - (24 * 60 * 60 * 1_000_000); // Last 24 hours
    let mut factors = Vec::new();

    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(factor) = record.entry().to_app_option::<EnvironmentalFactor>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                if factor.region.country == region.country && factor.recorded_at > cutoff {
                    factors.push(factor);
                }
            }
        }
    }

    Ok(factors)
}

// ==================== COMMUNITY PULSE ====================

/// Update community health pulse
#[hdk_extern]
pub fn update_community_pulse(input: UpdateCommunityPulseInput) -> ExternResult<Record> {
    let pulse = CommunityPulse {
        pulse_id: generate_pulse_id(&input.region),
        region: input.region.clone(),
        timestamp: sys_time()?.as_micros() as i64,
        health_sentiment: input.health_sentiment,
        active_concerns: input.active_concerns,
        positive_trends: input.positive_trends,
        resource_status: input.resource_status,
        last_updated: sys_time()?.as_micros() as i64,
    };

    let action_hash = create_entry(EntryTypes::CommunityPulse(pulse))?;

    // Link to region
    let region_anchor = anchor_for_region(&input.region)?;
    create_link(region_anchor, action_hash.clone(), LinkTypes::RegionToMoments, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created pulse".to_string())))?;

    Ok(record)
}

/// Get community pulse for a region
#[hdk_extern]
pub fn get_community_pulse(region: RegionIdentifier) -> ExternResult<Option<CommunityPulse>> {
    let region_anchor = anchor_for_region(&region)?;
    let links = get_links(
        LinkQuery::try_new(region_anchor, LinkTypes::RegionToMoments)?, GetStrategy::default()
    )?;

    let mut latest_pulse: Option<CommunityPulse> = None;
    let mut latest_timestamp: i64 = 0;

    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(pulse) = record.entry().to_app_option::<CommunityPulse>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                if pulse.timestamp > latest_timestamp {
                    latest_timestamp = pulse.timestamp;
                    latest_pulse = Some(pulse);
                }
            }
        }
    }

    Ok(latest_pulse)
}

// ==================== HEALTH ADVISORIES ====================

/// Issue a health advisory
#[hdk_extern]
pub fn issue_health_advisory(input: IssueAdvisoryInput) -> ExternResult<Record> {
    let advisory = HealthAdvisory {
        advisory_id: generate_advisory_id(),
        region: input.region.clone(),
        advisory_type: input.advisory_type,
        severity: input.severity,
        title: input.title,
        description: input.description,
        affected_groups: input.affected_groups,
        recommended_actions: input.recommended_actions,
        issued_at: sys_time()?.as_micros() as i64,
        expires_at: input.expires_at,
        status: AdvisoryStatus::Active,
        source: input.source,
    };

    let action_hash = create_entry(EntryTypes::HealthAdvisory(advisory))?;

    // Link to active advisories
    let active_anchor = anchor_for_active_advisories()?;
    create_link(active_anchor, action_hash.clone(), LinkTypes::ActiveAdvisories, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created advisory".to_string())))?;

    Ok(record)
}

/// Get active advisories for a region
#[hdk_extern]
pub fn get_active_advisories(region: RegionIdentifier) -> ExternResult<Vec<HealthAdvisory>> {
    get_active_advisories_for_region(region)
}

fn get_active_advisories_for_region(region: RegionIdentifier) -> ExternResult<Vec<HealthAdvisory>> {
    let active_anchor = anchor_for_active_advisories()?;
    let links = get_links(
        LinkQuery::try_new(active_anchor, LinkTypes::ActiveAdvisories)?, GetStrategy::default()
    )?;

    let now = sys_time()?.as_micros() as i64;
    let mut advisories = Vec::new();

    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(advisory) = record.entry().to_app_option::<HealthAdvisory>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                // Check if active and not expired
                if advisory.status == AdvisoryStatus::Active {
                    let not_expired = advisory.expires_at.map_or(true, |exp| exp > now);
                    let same_region = advisory.region.country == region.country &&
                        (region.state.is_none() || advisory.region.state == region.state);

                    if not_expired && same_region {
                        advisories.push(advisory);
                    }
                }
            }
        }
    }

    // Sort by severity (most severe first)
    advisories.sort_by(|a, b| severity_rank(&b.severity).cmp(&severity_rank(&a.severity)));

    Ok(advisories)
}

/// Update advisory status
#[hdk_extern]
pub fn update_advisory_status(input: UpdateAdvisoryStatusInput) -> ExternResult<Record> {
    let record = get(input.advisory_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Advisory not found".to_string())))?;

    let mut advisory = record.entry().to_app_option::<HealthAdvisory>()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize advisory".to_string())))?;

    advisory.status = input.new_status;

    let action_hash = update_entry(input.advisory_hash, EntryTypes::HealthAdvisory(advisory))?;

    let updated_record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve updated advisory".to_string())))?;

    Ok(updated_record)
}

// ==================== WELLNESS RECOMMENDATIONS ====================

/// Create a wellness recommendation
#[hdk_extern]
pub fn create_recommendation(input: CreateRecommendationInput) -> ExternResult<Record> {
    let recommendation = WellnessRecommendation {
        recommendation_id: generate_recommendation_id(),
        region: input.region.clone(),
        category: input.category,
        title: input.title,
        description: input.description,
        applicable_conditions: input.applicable_conditions,
        target_ages: input.target_ages,
        priority: input.priority,
        evidence_level: input.evidence_level,
        created_at: sys_time()?.as_micros() as i64,
        valid_until: input.valid_until,
    };

    let action_hash = create_entry(EntryTypes::WellnessRecommendation(recommendation))?;

    // Link to region moments
    let region_anchor = anchor_for_region(&input.region)?;
    create_link(region_anchor, action_hash.clone(), LinkTypes::MomentToRecommendations, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created recommendation".to_string())))?;

    Ok(record)
}

/// Get recommendations for a region
#[hdk_extern]
pub fn get_recommendations(region: RegionIdentifier) -> ExternResult<Vec<WellnessRecommendation>> {
    get_recommendations_for_region(region)
}

fn get_recommendations_for_region(region: RegionIdentifier) -> ExternResult<Vec<WellnessRecommendation>> {
    let region_anchor = anchor_for_region(&region)?;
    let links = get_links(
        LinkQuery::try_new(region_anchor, LinkTypes::MomentToRecommendations)?, GetStrategy::default()
    )?;

    let now = sys_time()?.as_micros() as i64;
    let mut recommendations = Vec::new();

    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(rec) = record.entry().to_app_option::<WellnessRecommendation>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                // Check if still valid
                let valid = rec.valid_until.map_or(true, |exp| exp > now);
                if valid {
                    recommendations.push(rec);
                }
            }
        }
    }

    // Sort by priority
    recommendations.sort_by(|a, b| priority_rank(&b.priority).cmp(&priority_rank(&a.priority)));

    Ok(recommendations)
}

// ==================== PERSONAL CONTEXT ====================

/// Create or update personal health context
#[hdk_extern]
pub fn create_personal_context(input: CreatePersonalContextInput) -> ExternResult<Record> {
    let my_pubkey = agent_info()?.agent_initial_pubkey;

    let context = PersonalContext {
        context_id: generate_context_id(&my_pubkey),
        agent_hash: my_pubkey,
        home_region: input.home_region,
        risk_factors: input.risk_factors,
        age_group: input.age_group,
        relevant_conditions: input.relevant_conditions,
        notification_prefs: input.notification_prefs,
        updated_at: sys_time()?.as_micros() as i64,
    };

    let action_hash = create_entry(EntryTypes::PersonalContext(context))?;

    // Link to personal contexts (private)
    let personal_anchor = anchor_for_personal_contexts()?;
    create_link(personal_anchor, action_hash.clone(), LinkTypes::PersonalContexts, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created context".to_string())))?;

    Ok(record)
}

/// Get my personal context
#[hdk_extern]
pub fn get_my_personal_context(_: ()) -> ExternResult<Option<PersonalContext>> {
    let my_pubkey = agent_info()?.agent_initial_pubkey;
    let personal_anchor = anchor_for_personal_contexts()?;
    let links = get_links(
        LinkQuery::try_new(personal_anchor, LinkTypes::PersonalContexts)?, GetStrategy::default()
    )?;

    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(context) = record.entry().to_app_option::<PersonalContext>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                if context.agent_hash == my_pubkey {
                    return Ok(Some(context));
                }
            }
        }
    }

    Ok(None)
}

// ==================== GLOBAL DASHBOARD ====================

/// Update global health dashboard
#[hdk_extern]
pub fn update_global_dashboard(input: UpdateGlobalDashboardInput) -> ExternResult<Record> {
    let dashboard = GlobalDashboard {
        dashboard_id: generate_dashboard_id(),
        timestamp: sys_time()?.as_micros() as i64,
        global_wellness_score: input.global_wellness_score,
        regional_highlights: input.regional_highlights,
        global_concerns: input.global_concerns,
        positive_trends: input.positive_trends,
        data_quality: input.data_quality,
    };

    let action_hash = create_entry(EntryTypes::GlobalDashboard(dashboard))?;

    // Link to global dashboards
    let global_anchor = anchor_for_global_dashboards()?;
    create_link(global_anchor, action_hash.clone(), LinkTypes::GlobalDashboards, ())?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not retrieve created dashboard".to_string())))?;

    Ok(record)
}

/// Get current global dashboard
#[hdk_extern]
pub fn get_global_dashboard(_: ()) -> ExternResult<Option<GlobalDashboard>> {
    let global_anchor = anchor_for_global_dashboards()?;
    let links = get_links(
        LinkQuery::try_new(global_anchor, LinkTypes::GlobalDashboards)?, GetStrategy::default()
    )?;

    let mut latest_dashboard: Option<GlobalDashboard> = None;
    let mut latest_timestamp: i64 = 0;

    for link in links {
        if let Some(record) = get(ActionHash::try_from(link.target).map_err(|_| {
            wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string()))
        })?, GetOptions::default())? {
            if let Some(dashboard) = record.entry().to_app_option::<GlobalDashboard>().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(e.to_string()))
            })? {
                if dashboard.timestamp > latest_timestamp {
                    latest_timestamp = dashboard.timestamp;
                    latest_dashboard = Some(dashboard);
                }
            }
        }
    }

    Ok(latest_dashboard)
}

// ==================== HELPER FUNCTIONS ====================

/// Simple anchor type for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}

fn anchor_for_region(region: &RegionIdentifier) -> ExternResult<AnyLinkableHash> {
    let anchor_str = format!("region:{}:{}", region.country,
        region.state.as_ref().unwrap_or(&"*".to_string()));
    Ok(AnyLinkableHash::from(anchor_hash(&anchor_str)?))
}

fn anchor_for_season(season: &Season) -> ExternResult<AnyLinkableHash> {
    let anchor_str = format!("season:{:?}", season);
    Ok(AnyLinkableHash::from(anchor_hash(&anchor_str)?))
}

fn anchor_for_active_advisories() -> ExternResult<AnyLinkableHash> {
    Ok(AnyLinkableHash::from(anchor_hash("active_advisories")?))
}

fn anchor_for_environmental_alerts() -> ExternResult<AnyLinkableHash> {
    Ok(AnyLinkableHash::from(anchor_hash("environmental_alerts")?))
}

fn anchor_for_personal_contexts() -> ExternResult<AnyLinkableHash> {
    Ok(AnyLinkableHash::from(anchor_hash("personal_contexts")?))
}

fn anchor_for_global_dashboards() -> ExternResult<AnyLinkableHash> {
    Ok(AnyLinkableHash::from(anchor_hash("global_dashboards")?))
}

fn determine_impact(factor_type: &EnvironmentalType, value: f64) -> QualityImpact {
    match factor_type {
        EnvironmentalType::AirQualityIndex => {
            if value <= 50.0 { QualityImpact::Good }
            else if value <= 100.0 { QualityImpact::Moderate }
            else if value <= 150.0 { QualityImpact::UnhealthyForSensitive }
            else if value <= 200.0 { QualityImpact::Unhealthy }
            else if value <= 300.0 { QualityImpact::VeryUnhealthy }
            else { QualityImpact::Hazardous }
        }
        EnvironmentalType::UVIndex => {
            if value <= 2.0 { QualityImpact::Good }
            else if value <= 5.0 { QualityImpact::Moderate }
            else if value <= 7.0 { QualityImpact::UnhealthyForSensitive }
            else if value <= 10.0 { QualityImpact::Unhealthy }
            else { QualityImpact::VeryUnhealthy }
        }
        EnvironmentalType::PollenCount => {
            if value <= 20.0 { QualityImpact::Good }
            else if value <= 80.0 { QualityImpact::Moderate }
            else if value <= 200.0 { QualityImpact::UnhealthyForSensitive }
            else if value <= 500.0 { QualityImpact::Unhealthy }
            else { QualityImpact::VeryUnhealthy }
        }
        _ => QualityImpact::Moderate,
    }
}

fn generate_environmental_recommendations(factor_type: &EnvironmentalType, value: f64) -> Vec<String> {
    let mut recs = Vec::new();

    match factor_type {
        EnvironmentalType::AirQualityIndex if value > 100.0 => {
            recs.push("Limit outdoor activities".to_string());
            recs.push("Keep windows closed".to_string());
            if value > 150.0 {
                recs.push("Consider wearing a mask outdoors".to_string());
            }
        }
        EnvironmentalType::UVIndex if value > 5.0 => {
            recs.push("Use SPF 30+ sunscreen".to_string());
            recs.push("Wear protective clothing".to_string());
            if value > 7.0 {
                recs.push("Avoid midday sun exposure".to_string());
            }
        }
        EnvironmentalType::PollenCount if value > 80.0 => {
            recs.push("Check allergy medication".to_string());
            recs.push("Shower after outdoor activities".to_string());
        }
        _ => {}
    }

    recs
}

fn compute_personalized_risk(context: &PersonalContext) -> RiskLevel {
    // Simple risk computation based on context
    let base_risk = match context.age_group {
        AgeGroup::Infants | AgeGroup::Elderly => 2,
        AgeGroup::Seniors => 1,
        _ => 0,
    };

    let condition_risk = context.relevant_conditions.len();
    let factor_risk = context.risk_factors.len();

    let total_risk = base_risk + condition_risk + factor_risk;

    if total_risk == 0 { RiskLevel::Low }
    else if total_risk <= 2 { RiskLevel::Moderate }
    else if total_risk <= 4 { RiskLevel::Elevated }
    else if total_risk <= 6 { RiskLevel::High }
    else { RiskLevel::Severe }
}

fn severity_rank(severity: &AdvisorySeverity) -> u8 {
    match severity {
        AdvisorySeverity::Information => 0,
        AdvisorySeverity::Watch => 1,
        AdvisorySeverity::Warning => 2,
        AdvisorySeverity::Urgent => 3,
        AdvisorySeverity::Emergency => 4,
    }
}

fn priority_rank(priority: &Priority) -> u8 {
    match priority {
        Priority::Optional => 0,
        Priority::Suggested => 1,
        Priority::Recommended => 2,
        Priority::StronglyRecommended => 3,
        Priority::Critical => 4,
    }
}

fn get_timestamp_micros() -> i64 {
    sys_time().map(|t| t.as_micros() as i64).unwrap_or(0)
}

fn generate_moment_id(region: &RegionIdentifier) -> String {
    let now = get_timestamp_micros();
    format!("MOM-{}-{}", region.country, now % 100000)
}

fn generate_pattern_id(region: &RegionIdentifier, season: &Season) -> String {
    let now = get_timestamp_micros();
    format!("PAT-{}-{:?}-{}", region.country, season, now % 10000)
}

fn generate_factor_id(region: &RegionIdentifier, factor: &EnvironmentalType) -> String {
    let now = get_timestamp_micros();
    format!("ENV-{}-{:?}-{}", region.country, factor, now % 10000)
}

fn generate_pulse_id(region: &RegionIdentifier) -> String {
    let now = get_timestamp_micros();
    format!("PLS-{}-{}", region.country, now % 100000)
}

fn generate_advisory_id() -> String {
    format!("ADV-{}", get_timestamp_micros())
}

fn generate_recommendation_id() -> String {
    let now = get_timestamp_micros();
    format!("REC-{}", now)
}

fn generate_context_id(agent: &AgentPubKey) -> String {
    let now = get_timestamp_micros();
    let agent_str = format!("{:?}", agent);
    format!("CTX-{}-{}", &agent_str[..8.min(agent_str.len())], now % 10000)
}

fn generate_dashboard_id() -> String {
    format!("GDB-{}", get_timestamp_micros())
}

// ==================== INPUT/OUTPUT TYPES ====================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateHealthMomentInput {
    pub region: RegionIdentifier,
    pub conditions: Vec<ActiveCondition>,
    pub environmental: Vec<EnvironmentalReading>,
    pub community_indicators: CommunityIndicators,
    pub active_advisory_ids: Vec<String>,
    pub seasonal_context: SeasonalContext,
    pub data_quality: DataQuality,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonalizedMoment {
    pub health_moment: Option<HealthMoment>,
    pub relevant_advisories: Vec<HealthAdvisory>,
    pub relevant_recommendations: Vec<WellnessRecommendation>,
    pub personalized_risk_level: RiskLevel,
    pub context_last_updated: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateSeasonalPatternInput {
    pub region: RegionIdentifier,
    pub season: Season,
    pub typical_conditions: Vec<TypicalCondition>,
    pub environmental_norms: Vec<EnvironmentalNorm>,
    pub baselines: HistoricalBaselines,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetSeasonalPatternsInput {
    pub region: RegionIdentifier,
    pub season: Season,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecordEnvironmentalInput {
    pub region: RegionIdentifier,
    pub factor_type: EnvironmentalType,
    pub current_value: f64,
    pub unit: String,
    pub impact: QualityImpact,
    pub forecast: Option<EnvironmentalForecast>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateCommunityPulseInput {
    pub region: RegionIdentifier,
    pub health_sentiment: HealthSentiment,
    pub active_concerns: Vec<CommunityConcern>,
    pub positive_trends: Vec<PositiveTrend>,
    pub resource_status: ResourceStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IssueAdvisoryInput {
    pub region: RegionIdentifier,
    pub advisory_type: AdvisoryType,
    pub severity: AdvisorySeverity,
    pub title: String,
    pub description: String,
    pub affected_groups: Vec<AgeGroup>,
    pub recommended_actions: Vec<String>,
    pub expires_at: Option<i64>,
    pub source: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateAdvisoryStatusInput {
    pub advisory_hash: ActionHash,
    pub new_status: AdvisoryStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateRecommendationInput {
    pub region: RegionIdentifier,
    pub category: RecommendationCategory,
    pub title: String,
    pub description: String,
    pub applicable_conditions: Vec<ConditionType>,
    pub target_ages: Vec<AgeGroup>,
    pub priority: Priority,
    pub evidence_level: EvidenceLevel,
    pub valid_until: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreatePersonalContextInput {
    pub home_region: RegionIdentifier,
    pub risk_factors: Vec<String>,
    pub age_group: AgeGroup,
    pub relevant_conditions: Vec<ConditionType>,
    pub notification_prefs: NotificationPreferences,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateGlobalDashboardInput {
    pub global_wellness_score: f64,
    pub regional_highlights: Vec<RegionalHighlight>,
    pub global_concerns: Vec<GlobalConcern>,
    pub positive_trends: Vec<PositiveTrend>,
    pub data_quality: DataQuality,
}
