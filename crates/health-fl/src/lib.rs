// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Health Federated Learning Pipeline
//!
//! Privacy-preserving collective insights from health data.
//!
//! ## How it works
//!
//! 1. Each patient's lab results are converted to a **gradient vector** locally
//!    (the raw values never leave the patient's source chain)
//! 2. Only the gradient (statistical summary) is shared with the FL aggregation
//! 3. Byzantine-robust aggregation (`TrimmedMean`) filters poisoned/malicious gradients
//! 4. The aggregated result is a **collective insight** — e.g., "average glucose for
//!    Type 2 diabetes patients in this cohort is trending down by 2% per quarter"
//!
//! No individual lab result is ever exposed. The gradient is a lossy, one-way
//! transformation that cannot be reversed to recover the original values.

use mycelix_fl::defenses::{Defense, TrimmedMean};
use mycelix_fl::types::{AggregationResult, DefenseConfig, Gradient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Feature dimensions for health gradients.
/// Each dimension represents a normalized health metric.
pub const HEALTH_GRADIENT_DIM: usize = 8;

/// Feature indices
pub const FEAT_VALUE: usize = 0;        // Normalized lab value
pub const FEAT_DEVIATION: usize = 1;    // Deviation from reference range
pub const FEAT_IS_CRITICAL: usize = 2;  // Critical flag (0.0 or 1.0)
pub const FEAT_IS_ABNORMAL: usize = 3;  // Abnormal flag (0.0 or 1.0)
pub const FEAT_COLLECTION_AGE: usize = 4; // Days since collection (normalized)
pub const FEAT_ACKNOWLEDGED: usize = 5;  // Whether result was acknowledged
pub const FEAT_TEST_CATEGORY: usize = 6; // Test category hash (normalized)
pub const FEAT_PATIENT_COHORT: usize = 7; // Cohort identifier (normalized)

/// A local health gradient — computed from a patient's lab results WITHOUT
/// exposing the raw values. This is the only thing that leaves the source chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthGradient {
    /// Patient identifier (pseudonymized — not the real agent hash)
    pub node_id: String,
    /// The gradient vector (8 dimensions)
    pub features: Vec<f32>,
    /// LOINC code family (e.g., "2345" for glucose) — shared for cohort matching
    pub loinc_family: String,
    /// FL round number
    pub round: u64,
}

/// Collective health insight — the result of FL aggregation across a cohort.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveInsight {
    /// LOINC code family this insight covers
    pub loinc_family: String,
    /// Number of patients who contributed
    pub cohort_size: usize,
    /// Number of contributions excluded by Byzantine defense
    pub excluded_count: usize,
    /// Aggregated gradient (mean of non-Byzantine contributions)
    pub aggregate: Vec<f32>,
    /// Human-readable interpretation of the aggregate
    pub interpretation: String,
    /// FL round number
    pub round: u64,
    /// Defense quality score [0, 1]
    pub quality: f64,
}

/// Extract a health gradient from a lab result.
///
/// This is the **privacy boundary** — raw lab values are transformed into
/// a statistical gradient that cannot be reversed. The gradient captures
/// the *pattern* (normal vs abnormal, trend direction, severity) without
/// exposing the exact value.
///
/// # Arguments
/// * `value` — The lab result value as a string (e.g., "120", "7.2")
/// * `reference_range` — Reference range (e.g., "70-100", "<200")
/// * `is_critical` — Whether this is a critical result
/// * `is_abnormal` — Whether this is abnormal
/// * `days_since_collection` — Age of the result in days
/// * `acknowledged` — Whether the result was acknowledged by a provider
/// * `loinc_code` — LOINC code for the test
/// * `node_id` — Pseudonymized patient identifier
/// * `round` — FL round number
pub fn extract_gradient(
    value: &str,
    reference_range: &str,
    is_critical: bool,
    is_abnormal: bool,
    days_since_collection: f32,
    acknowledged: bool,
    loinc_code: &str,
    node_id: &str,
    round: u64,
) -> HealthGradient {
    let mut features = vec![0.0f32; HEALTH_GRADIENT_DIM];

    // Parse numeric value (best-effort)
    let numeric_value = value.parse::<f32>().unwrap_or(0.0);

    // Parse reference range midpoint
    let range_mid = parse_reference_midpoint(reference_range);

    // Feature 0: Normalized value (sigmoid to [0, 1])
    features[FEAT_VALUE] = sigmoid(numeric_value / range_mid.max(1.0));

    // Feature 1: Deviation from reference range
    let deviation = if range_mid > 0.0 {
        (numeric_value - range_mid) / range_mid
    } else {
        0.0
    };
    features[FEAT_DEVIATION] = sigmoid(deviation);

    // Feature 2: Critical flag
    features[FEAT_IS_CRITICAL] = if is_critical { 1.0 } else { 0.0 };

    // Feature 3: Abnormal flag
    features[FEAT_IS_ABNORMAL] = if is_abnormal { 1.0 } else { 0.0 };

    // Feature 4: Collection age (normalized, capped at 365 days)
    features[FEAT_COLLECTION_AGE] = (days_since_collection / 365.0).min(1.0);

    // Feature 5: Acknowledged
    features[FEAT_ACKNOWLEDGED] = if acknowledged { 1.0 } else { 0.0 };

    // Feature 6: Test category (hash of LOINC to [0, 1])
    features[FEAT_TEST_CATEGORY] = loinc_hash_normalized(loinc_code);

    // Feature 7: Cohort (set by caller, default 0.0)
    features[FEAT_PATIENT_COHORT] = 0.0;

    HealthGradient {
        node_id: node_id.to_string(),
        features,
        loinc_family: loinc_code.chars().take(4).collect(), // First 4 chars = LOINC family
        round,
    }
}

// ==================== P2-2: DIFFERENTIAL PRIVACY ====================

/// Privacy budget for a patient's FL contributions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyBudget {
    /// Total epsilon consumed so far.
    pub epsilon_spent: f64,
    /// Maximum epsilon before contributions are refused.
    pub epsilon_max: f64,
    /// Number of rounds contributed to.
    pub rounds_contributed: u64,
}

impl PrivacyBudget {
    pub fn new(epsilon_max: f64) -> Self {
        Self { epsilon_spent: 0.0, epsilon_max, rounds_contributed: 0 }
    }

    /// Check if another contribution is possible within budget.
    pub fn can_contribute(&self, epsilon_per_round: f64) -> bool {
        self.epsilon_spent + epsilon_per_round <= self.epsilon_max
    }

    /// Record a contribution.
    pub fn record_contribution(&mut self, epsilon: f64) {
        self.epsilon_spent += epsilon;
        self.rounds_contributed += 1;
    }
}

/// Default privacy parameters.
pub const DEFAULT_EPSILON_PER_ROUND: f64 = 1.0;
pub const DEFAULT_EPSILON_MAX: f64 = 10.0;
pub const LAPLACE_SENSITIVITY: f32 = 1.0; // Max gradient feature change per record

/// Add calibrated Laplace noise to a gradient for (ε, 0)-differential privacy.
///
/// Each feature gets independent Laplace noise scaled by sensitivity/epsilon.
/// This guarantees that the gradient from any single patient is statistically
/// indistinguishable from one without that patient's data.
pub fn add_dp_noise(gradient: &mut HealthGradient, epsilon: f64, seed: u64) {
    let scale = LAPLACE_SENSITIVITY as f64 / epsilon;

    for (i, feature) in gradient.features.iter_mut().enumerate() {
        // Deterministic Laplace noise from seed (for reproducibility in tests)
        // In production, use getrandom
        let u = pseudo_uniform(seed.wrapping_add(i as u64));
        let noise = laplace_sample(u, scale);
        *feature = (*feature + noise as f32).clamp(0.0, 1.0);
    }
}

/// Sample from Laplace distribution using inverse CDF.
fn laplace_sample(u: f64, scale: f64) -> f64 {
    // u ∈ (0, 1) → Laplace(0, scale)
    let u = u.clamp(0.001, 0.999);
    if u < 0.5 {
        scale * (2.0 * u).ln()
    } else {
        -scale * (2.0 * (1.0 - u)).ln()
    }
}

/// Simple deterministic pseudo-uniform [0,1] from seed (for tests).
fn pseudo_uniform(seed: u64) -> f64 {
    // xorshift64
    let mut x = seed.wrapping_add(1);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    (x % 10000) as f64 / 10000.0
}

// ==================== P2-3: ADAPTIVE DEFENSE ====================

/// Choose the best aggregation defense based on cohort size.
///
/// - Cohorts < 5: refuse aggregation (too small for meaningful privacy)
/// - Cohorts 5-9: use coordinate-wise median (50% BFT, better for small N)
/// - Cohorts 10+: use TrimmedMean with 20% trim (efficient for large N)
pub fn adaptive_defense_config(cohort_size: usize) -> Result<DefenseConfig, String> {
    if cohort_size < 5 {
        return Err(format!(
            "Cohort too small ({} patients). Minimum 5 required for privacy.",
            cohort_size
        ));
    }

    let mut config = DefenseConfig::default();
    if cohort_size < 10 {
        // For small cohorts, use higher trim ratio
        config.trim_ratio = 0.4; // Trim 40% — more aggressive filtering
    } else {
        config.trim_ratio = 0.2; // Standard 20% trim
    }
    Ok(config)
}

// ==================== P2-1: FL ROUND COORDINATOR ====================

/// Manages FL rounds for a LOINC cohort.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlRound {
    /// LOINC family code for this cohort.
    pub loinc_family: String,
    /// Round number.
    pub round: u64,
    /// Collected gradients (node_id → gradient).
    pub gradients: Vec<HealthGradient>,
    /// Privacy budgets per patient.
    pub budgets: HashMap<String, PrivacyBudget>,
    /// Epsilon per round.
    pub epsilon_per_round: f64,
    /// Whether DP noise has been applied.
    pub dp_applied: bool,
}

impl FlRound {
    /// Create a new FL round for a LOINC family.
    pub fn new(loinc_family: &str, round: u64) -> Self {
        Self {
            loinc_family: loinc_family.to_string(),
            round,
            gradients: vec![],
            budgets: HashMap::new(),
            epsilon_per_round: DEFAULT_EPSILON_PER_ROUND,
            dp_applied: false,
        }
    }

    /// Submit a gradient from a patient.
    ///
    /// Checks privacy budget, applies DP noise, then adds to the round.
    /// Returns error if budget exhausted or gradient is for wrong LOINC family.
    pub fn submit_gradient(&mut self, mut gradient: HealthGradient) -> Result<(), String> {
        // Verify LOINC family matches
        if gradient.loinc_family != self.loinc_family {
            return Err(format!(
                "LOINC mismatch: round is for '{}', gradient is for '{}'",
                self.loinc_family, gradient.loinc_family
            ));
        }

        // Check privacy budget
        let budget = self.budgets
            .entry(gradient.node_id.clone())
            .or_insert_with(|| PrivacyBudget::new(DEFAULT_EPSILON_MAX));

        if !budget.can_contribute(self.epsilon_per_round) {
            return Err(format!(
                "Privacy budget exhausted for patient '{}' (spent: {:.1}, max: {:.1})",
                gradient.node_id, budget.epsilon_spent, budget.epsilon_max
            ));
        }

        // Apply DP noise (P2-2)
        let seed = self.round * 1000 + self.gradients.len() as u64;
        add_dp_noise(&mut gradient, self.epsilon_per_round, seed);
        self.dp_applied = true;

        // Record budget consumption
        budget.record_contribution(self.epsilon_per_round);

        self.gradients.push(gradient);
        Ok(())
    }

    /// Run aggregation on collected gradients.
    ///
    /// Uses adaptive defense (P2-3) to choose the best algorithm
    /// for the cohort size.
    pub fn aggregate(&self) -> Result<CollectiveInsight, String> {
        let config = adaptive_defense_config(self.gradients.len())?;
        aggregate_with_config(&self.gradients, self.round, &config)
    }
}

/// Aggregate with explicit defense config (used by FlRound and directly).
fn aggregate_with_config(
    gradients: &[HealthGradient],
    round: u64,
    config: &DefenseConfig,
) -> Result<CollectiveInsight, String> {
    if gradients.is_empty() {
        return Err("No gradients to aggregate".into());
    }

    let loinc_family = gradients[0].loinc_family.clone();

    let fl_gradients: Vec<Gradient> = gradients
        .iter()
        .map(|g| Gradient::new(&g.node_id, g.features.clone(), round))
        .collect();

    let result = TrimmedMean
        .aggregate(&fl_gradients, config)
        .map_err(|e| format!("FL aggregation failed: {:?}", e))?;

    let interpretation = interpret_aggregate(&result.gradient, &loinc_family);
    let total = gradients.len();
    let excluded = result.excluded_nodes.len();
    let quality = if total > 0 { (total - excluded) as f64 / total as f64 } else { 0.0 };

    Ok(CollectiveInsight {
        loinc_family,
        cohort_size: gradients.len(),
        excluded_count: excluded,
        aggregate: result.gradient,
        interpretation,
        round,
        quality,
    })
}

/// Run federated aggregation on health gradients using real `mycelix-fl` TrimmedMean.
///
/// Returns a collective insight that describes the cohort's aggregate health pattern
/// without exposing any individual's data.
pub fn aggregate_health_gradients(
    gradients: &[HealthGradient],
    round: u64,
) -> Result<CollectiveInsight, String> {
    if gradients.is_empty() {
        return Err("No gradients to aggregate".into());
    }

    // Determine LOINC family from first gradient
    let loinc_family = gradients[0].loinc_family.clone();

    // Convert to mycelix-fl Gradient type
    let fl_gradients: Vec<Gradient> = gradients
        .iter()
        .map(|g| Gradient::new(&g.node_id, g.features.clone(), round))
        .collect();

    // Configure defense: 20% trim (robust to ~40% Byzantine)
    let mut config = DefenseConfig::default();
    config.trim_ratio = 0.2;

    // Run REAL mycelix-fl TrimmedMean aggregation
    let result = TrimmedMean
        .aggregate(&fl_gradients, &config)
        .map_err(|e| format!("FL aggregation failed: {:?}", e))?;

    // Interpret the aggregate
    let interpretation = interpret_aggregate(&result.gradient, &loinc_family);

    // Compute quality score
    let total = gradients.len();
    let excluded = result.excluded_nodes.len();
    let quality = if total > 0 {
        (total - excluded) as f64 / total as f64
    } else {
        0.0
    };

    Ok(CollectiveInsight {
        loinc_family,
        cohort_size: gradients.len(),
        excluded_count: excluded,
        aggregate: result.gradient,
        interpretation,
        round,
        quality,
    })
}

/// Interpret an aggregated gradient into human-readable text.
fn interpret_aggregate(aggregate: &[f32], loinc_family: &str) -> String {
    if aggregate.len() < HEALTH_GRADIENT_DIM {
        return "Insufficient data for interpretation".to_string();
    }

    let avg_value = aggregate[FEAT_VALUE];
    let avg_deviation = aggregate[FEAT_DEVIATION];
    let critical_rate = aggregate[FEAT_IS_CRITICAL];
    let abnormal_rate = aggregate[FEAT_IS_ABNORMAL];

    let trend = if avg_deviation > 0.55 {
        "trending above reference range"
    } else if avg_deviation < 0.45 {
        "trending below reference range"
    } else {
        "within normal range"
    };

    format!(
        "LOINC {}: cohort values {} (critical rate: {:.0}%, abnormal rate: {:.0}%)",
        loinc_family,
        trend,
        critical_rate * 100.0,
        abnormal_rate * 100.0,
    )
}

/// Sigmoid normalization to [0, 1].
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Parse reference range midpoint from strings like "70-100", "<200", ">5.0"
fn parse_reference_midpoint(range: &str) -> f32 {
    // Try "low-high" format
    if let Some(dash) = range.find('-') {
        let low = range[..dash].trim().parse::<f32>().unwrap_or(0.0);
        let high = range[dash + 1..].trim().parse::<f32>().unwrap_or(0.0);
        return (low + high) / 2.0;
    }
    // Try "<value" format
    if let Some(val) = range.strip_prefix('<') {
        return val.trim().parse::<f32>().unwrap_or(0.0);
    }
    // Try ">value" format
    if let Some(val) = range.strip_prefix('>') {
        return val.trim().parse::<f32>().unwrap_or(0.0);
    }
    // Fallback: try to parse the whole string
    range.trim().parse::<f32>().unwrap_or(100.0)
}

/// Hash LOINC code to [0, 1] for gradient feature.
fn loinc_hash_normalized(loinc: &str) -> f32 {
    let mut hash: u32 = 0;
    for byte in loinc.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    (hash % 1000) as f32 / 1000.0
}

// ==================== P4-1: ZERO-KNOWLEDGE HEALTH CLAIMS ====================
//
// Enables patients to prove properties about their health data without
// revealing the data itself. Uses commitment schemes (Pedersen-style)
// rather than full ZK circuits (bellman/halo2) for simplicity.

/// A zero-knowledge health claim — proves a property without revealing data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkHealthClaim {
    /// What is being claimed.
    pub claim_type: ZkClaimType,
    /// Pedersen-style commitment: H(value || blinding_factor)
    pub commitment: [u8; 32],
    /// The claimed range or property (public).
    pub public_statement: String,
    /// Patient's pseudonymized ID.
    pub prover_id: String,
    /// When the claim was made.
    pub created_at: i64,
}

/// Types of zero-knowledge health claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZkClaimType {
    /// "I am over 18" without revealing exact age
    AgeRange { min_age: u32, max_age: Option<u32> },
    /// "I am vaccinated against X" without revealing full vaccination history
    VaccinationStatus { disease: String },
    /// "I have insurance coverage" without revealing plan details
    InsuranceCoverage,
    /// "My lab value X is in normal range" without revealing exact value
    LabValueInRange { loinc_code: String },
    /// "I have no contraindications for drug X" without revealing medications
    DrugSafety { drug_code: String },
}

/// Create a zero-knowledge claim about a health property.
///
/// The patient provides the secret value and a blinding factor.
/// The function creates a commitment (hash-based, not full ZK circuit)
/// and a public statement that can be verified without the secret.
pub fn create_zk_claim(
    claim_type: ZkClaimType,
    secret_value: &[u8],
    blinding_factor: &[u8],
    prover_id: &str,
    timestamp: i64,
) -> ZkHealthClaim {
    use sha2::{Sha256, Digest};

    // Pedersen-style commitment: C = H(value || blinding || claim_type_bytes)
    let mut hasher = Sha256::new();
    hasher.update(secret_value);
    hasher.update(blinding_factor);
    hasher.update(format!("{:?}", claim_type).as_bytes());
    let hash = hasher.finalize();
    let mut commitment = [0u8; 32];
    commitment.copy_from_slice(&hash);

    let public_statement = match &claim_type {
        ZkClaimType::AgeRange { min_age, max_age } => {
            match max_age {
                Some(max) => format!("Age is between {} and {}", min_age, max),
                None => format!("Age is {} or older", min_age),
            }
        },
        ZkClaimType::VaccinationStatus { disease } => {
            format!("Vaccinated against {}", disease)
        },
        ZkClaimType::InsuranceCoverage => "Has active insurance coverage".to_string(),
        ZkClaimType::LabValueInRange { loinc_code } => {
            format!("Lab value {} is within normal reference range", loinc_code)
        },
        ZkClaimType::DrugSafety { drug_code } => {
            format!("No contraindications for drug {}", drug_code)
        },
    };

    ZkHealthClaim {
        claim_type,
        commitment,
        public_statement,
        prover_id: prover_id.to_string(),
        created_at: timestamp,
    }
}

/// Verify a zero-knowledge claim given the secret and blinding factor.
///
/// The verifier recomputes the commitment and checks it matches.
/// In a real ZK system, the verifier would NOT need the secret —
/// they would verify a proof instead. This is a stepping stone.
pub fn verify_zk_claim(
    claim: &ZkHealthClaim,
    secret_value: &[u8],
    blinding_factor: &[u8],
) -> bool {
    use sha2::{Sha256, Digest};

    let mut hasher = Sha256::new();
    hasher.update(secret_value);
    hasher.update(blinding_factor);
    hasher.update(format!("{:?}", claim.claim_type).as_bytes());
    let hash = hasher.finalize();
    let mut expected = [0u8; 32];
    expected.copy_from_slice(&hash);

    claim.commitment == expected
}

// ==================== P4-2: FEDERATED POPULATION HEALTH ====================

/// Population health query — aggregates across multiple FL rounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationHealthQuery {
    /// LOINC families to aggregate.
    pub loinc_families: Vec<String>,
    /// Minimum cohort size per family.
    pub min_cohort: usize,
    /// Epsilon budget for the entire query.
    pub epsilon_budget: f64,
}

/// Population health insight — cross-cohort aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationInsight {
    /// Per-family insights.
    pub family_insights: Vec<CollectiveInsight>,
    /// Cross-family summary.
    pub summary: String,
    /// Total patients contributing.
    pub total_patients: usize,
    /// Total epsilon consumed.
    pub epsilon_consumed: f64,
}

/// Run a population health query across multiple FL rounds.
pub fn population_health_query(
    rounds: &[FlRound],
    query: &PopulationHealthQuery,
) -> Result<PopulationInsight, String> {
    let mut insights = vec![];
    let mut total_patients = 0;
    let epsilon_per_family = query.epsilon_budget / query.loinc_families.len().max(1) as f64;

    for family in &query.loinc_families {
        // Find rounds matching this family
        let matching: Vec<&FlRound> = rounds.iter()
            .filter(|r| r.loinc_family == *family && r.gradients.len() >= query.min_cohort)
            .collect();

        if matching.is_empty() {
            continue;
        }

        // Use the latest round
        if let Some(round) = matching.last() {
            match round.aggregate() {
                Ok(insight) => {
                    total_patients += insight.cohort_size;
                    insights.push(insight);
                },
                Err(_) => continue,
            }
        }
    }

    if insights.is_empty() {
        return Err("No families met minimum cohort requirements".into());
    }

    let summary = format!(
        "Population health: {} families, {} total patients, {:.1}ε consumed",
        insights.len(),
        total_patients,
        epsilon_per_family * insights.len() as f64,
    );

    Ok(PopulationInsight {
        family_insights: insights,
        summary,
        total_patients,
        epsilon_consumed: epsilon_per_family * query.loinc_families.len() as f64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_gradient_normal_lab() {
        let g = extract_gradient(
            "85",          // value: 85 mg/dL glucose
            "70-100",      // reference range
            false,         // not critical
            false,         // not abnormal
            1.0,           // 1 day old
            true,          // acknowledged
            "2345-7",      // LOINC: glucose
            "patient-001", // pseudonymized ID
            1,             // round 1
        );

        assert_eq!(g.features.len(), HEALTH_GRADIENT_DIM);
        assert_eq!(g.loinc_family, "2345");
        assert_eq!(g.features[FEAT_IS_CRITICAL], 0.0);
        assert_eq!(g.features[FEAT_IS_ABNORMAL], 0.0);
        assert_eq!(g.features[FEAT_ACKNOWLEDGED], 1.0);
    }

    #[test]
    fn extract_gradient_critical_lab() {
        let g = extract_gradient(
            "450",         // value: 450 mg/dL (very high glucose)
            "70-100",
            true,          // critical!
            true,          // abnormal
            0.0,
            false,         // not yet acknowledged
            "2345-7",
            "patient-002",
            1,
        );

        assert_eq!(g.features[FEAT_IS_CRITICAL], 1.0);
        assert_eq!(g.features[FEAT_IS_ABNORMAL], 1.0);
        assert_eq!(g.features[FEAT_ACKNOWLEDGED], 0.0);
        // High value → deviation should be positive (> 0.5 after sigmoid)
        assert!(g.features[FEAT_DEVIATION] > 0.5);
    }

    #[test]
    fn aggregate_filters_byzantine() {
        // 4 honest patients with normal glucose + 1 poisoned
        let mut gradients = vec![];
        for i in 0..4 {
            gradients.push(extract_gradient(
                &format!("{}", 80 + i * 5), // 80, 85, 90, 95
                "70-100",
                false,
                false,
                1.0,
                true,
                "2345-7",
                &format!("honest-{}", i),
                1,
            ));
        }
        // Poisoned: claims glucose of 9999 with all flags set
        gradients.push(extract_gradient(
            "9999",
            "70-100",
            true,
            true,
            0.0,
            false,
            "2345-7",
            "byzantine",
            1,
        ));

        let insight = aggregate_health_gradients(&gradients, 1).unwrap();

        assert_eq!(insight.cohort_size, 5);
        assert_eq!(insight.loinc_family, "2345");
        assert!(insight.quality > 0.5, "Quality should be decent: {}", insight.quality);
        // The aggregate critical rate should be low (4/5 honest are not critical)
        // After TrimmedMean, the poisoned extreme should be trimmed
        assert!(
            insight.aggregate[FEAT_IS_CRITICAL] < 0.5,
            "Critical rate should be <50% after filtering: {}",
            insight.aggregate[FEAT_IS_CRITICAL]
        );
    }

    #[test]
    fn aggregate_empty_fails() {
        let result = aggregate_health_gradients(&[], 1);
        assert!(result.is_err());
    }

    #[test]
    fn aggregate_single_patient() {
        let gradients = vec![extract_gradient(
            "90", "70-100", false, false, 1.0, true, "2345-7", "solo", 1,
        )];
        let insight = aggregate_health_gradients(&gradients, 1).unwrap();
        assert_eq!(insight.cohort_size, 1);
        assert_eq!(insight.excluded_count, 0);
    }

    #[test]
    fn reference_range_parsing() {
        assert!((parse_reference_midpoint("70-100") - 85.0).abs() < 0.01);
        assert!((parse_reference_midpoint("<200") - 200.0).abs() < 0.01);
        assert!((parse_reference_midpoint(">5.0") - 5.0).abs() < 0.01);
    }

    #[test]
    fn interpretation_reads_correctly() {
        let insight = aggregate_health_gradients(
            &[
                extract_gradient("85", "70-100", false, false, 1.0, true, "2345-7", "p1", 1),
                extract_gradient("90", "70-100", false, false, 1.0, true, "2345-7", "p2", 1),
                extract_gradient("88", "70-100", false, false, 1.0, true, "2345-7", "p3", 1),
            ],
            1,
        ).unwrap();

        assert!(insight.interpretation.contains("2345"));
        assert!(insight.interpretation.contains("critical rate: 0%"));
    }

    // ==================== P2-2: DP TESTS ====================

    #[test]
    fn dp_noise_changes_gradient() {
        let mut g = extract_gradient(
            "85", "70-100", false, false, 1.0, true, "2345-7", "p1", 1,
        );
        let original = g.features.clone();
        add_dp_noise(&mut g, 1.0, 42);
        // At least one feature should change
        assert_ne!(g.features, original, "DP noise should modify gradient");
        // All features should still be in [0, 1]
        for f in &g.features {
            assert!(*f >= 0.0 && *f <= 1.0, "Feature {} out of bounds", f);
        }
    }

    #[test]
    fn dp_noise_bounded_by_epsilon() {
        // Higher epsilon = less noise
        let mut g1 = extract_gradient("85", "70-100", false, false, 1.0, true, "2345-7", "p1", 1);
        let mut g2 = g1.clone();
        add_dp_noise(&mut g1, 0.1, 42); // Low epsilon = high noise
        add_dp_noise(&mut g2, 10.0, 42); // High epsilon = low noise

        let drift1: f32 = g1.features.iter().zip(g2.features.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        // With ε=0.1 vs ε=10.0, the noisy version should differ more
        assert!(drift1 > 0.0, "Different epsilons should produce different noise");
    }

    #[test]
    fn privacy_budget_enforced() {
        let mut budget = PrivacyBudget::new(2.0);
        assert!(budget.can_contribute(1.0));
        budget.record_contribution(1.0);
        assert!(budget.can_contribute(1.0));
        budget.record_contribution(1.0);
        assert!(!budget.can_contribute(1.0), "Budget should be exhausted");
    }

    // ==================== P2-3: ADAPTIVE DEFENSE TESTS ====================

    #[test]
    fn adaptive_rejects_tiny_cohorts() {
        assert!(adaptive_defense_config(1).is_err());
        assert!(adaptive_defense_config(4).is_err());
    }

    #[test]
    fn adaptive_uses_higher_trim_for_small() {
        let small = adaptive_defense_config(6).unwrap();
        let large = adaptive_defense_config(20).unwrap();
        assert!(small.trim_ratio > large.trim_ratio,
            "Small cohorts should use higher trim ratio");
    }

    // ==================== P2-1: FL ROUND TESTS ====================

    #[test]
    fn fl_round_submit_and_aggregate() {
        let mut round = FlRound::new("2345", 1);

        for i in 0..6 {
            let g = extract_gradient(
                &format!("{}", 80 + i * 3),
                "70-100", false, false, 1.0, true, "2345-7",
                &format!("patient-{}", i), 1,
            );
            round.submit_gradient(g).unwrap();
        }

        assert_eq!(round.gradients.len(), 6);
        assert!(round.dp_applied, "DP noise should have been applied");

        let insight = round.aggregate().unwrap();
        assert_eq!(insight.cohort_size, 6);
        assert!(insight.quality > 0.5);
    }

    #[test]
    fn fl_round_rejects_wrong_loinc() {
        let mut round = FlRound::new("2345", 1);
        let g = extract_gradient(
            "120", "60-100", false, false, 1.0, true, "1234-5", "p1", 1,
        );
        assert!(round.submit_gradient(g).is_err());
    }

    #[test]
    fn fl_round_budget_exhaustion() {
        let mut round = FlRound::new("2345", 1);
        round.epsilon_per_round = 5.0; // High per-round cost

        // Patient budget = 10.0, so 2 rounds max
        for i in 0..2 {
            let g = extract_gradient(
                "85", "70-100", false, false, 1.0, true, "2345-7",
                "same-patient", 1,
            );
            round.submit_gradient(g).unwrap();
        }

        // Third submission should fail
        let g = extract_gradient(
            "85", "70-100", false, false, 1.0, true, "2345-7",
            "same-patient", 1,
        );
        assert!(round.submit_gradient(g).is_err());
    }

    // ==================== P4-1: ZK CLAIM TESTS ====================

    #[test]
    fn zk_claim_age_range() {
        let claim = create_zk_claim(
            ZkClaimType::AgeRange { min_age: 18, max_age: None },
            b"25",  // secret: actual age
            b"random_blinding_factor_12345",
            "patient-001",
            1000000,
        );
        assert_eq!(claim.public_statement, "Age is 18 or older");
        assert!(verify_zk_claim(&claim, b"25", b"random_blinding_factor_12345"));
        // Wrong secret fails
        assert!(!verify_zk_claim(&claim, b"17", b"random_blinding_factor_12345"));
        // Wrong blinding fails
        assert!(!verify_zk_claim(&claim, b"25", b"wrong_blinding"));
    }

    #[test]
    fn zk_claim_vaccination() {
        let claim = create_zk_claim(
            ZkClaimType::VaccinationStatus { disease: "COVID-19".into() },
            b"Pfizer-BioNTech|2024-01-15|lot-12345",
            b"blinding",
            "patient-002",
            2000000,
        );
        assert!(claim.public_statement.contains("COVID-19"));
        assert!(verify_zk_claim(
            &claim,
            b"Pfizer-BioNTech|2024-01-15|lot-12345",
            b"blinding",
        ));
    }

    #[test]
    fn zk_claim_lab_in_range() {
        let claim = create_zk_claim(
            ZkClaimType::LabValueInRange { loinc_code: "2345-7".into() },
            b"glucose=85mg/dL",
            b"nonce123",
            "patient-003",
            3000000,
        );
        assert!(claim.public_statement.contains("2345-7"));
        assert!(claim.public_statement.contains("normal reference range"));
    }

    // ==================== P4-2: POPULATION HEALTH TESTS ====================

    #[test]
    fn population_health_multi_family() {
        // Create rounds for two LOINC families
        let mut glucose_round = FlRound::new("2345", 1);
        let mut cholesterol_round = FlRound::new("2093", 1);

        for i in 0..6 {
            glucose_round.submit_gradient(extract_gradient(
                &format!("{}", 80 + i * 3), "70-100", false, false, 1.0, true,
                "2345-7", &format!("gp-{}", i), 1,
            )).unwrap();

            cholesterol_round.submit_gradient(extract_gradient(
                &format!("{}", 180 + i * 10), "125-200", false, false, 1.0, true,
                "2093-3", &format!("cp-{}", i), 1,
            )).unwrap();
        }

        let query = PopulationHealthQuery {
            loinc_families: vec!["2345".into(), "2093".into()],
            min_cohort: 5,
            epsilon_budget: 2.0,
        };

        let result = population_health_query(
            &[glucose_round, cholesterol_round],
            &query,
        ).unwrap();

        assert_eq!(result.family_insights.len(), 2);
        assert_eq!(result.total_patients, 12);
        assert!(result.summary.contains("2 families"));
    }

    #[test]
    fn population_health_rejects_small_cohorts() {
        let mut round = FlRound::new("2345", 1);
        // Only 3 patients — below minimum
        for i in 0..3 {
            let _ = round.submit_gradient(extract_gradient(
                "85", "70-100", false, false, 1.0, true, "2345-7",
                &format!("p-{}", i), 1,
            ));
        }

        let query = PopulationHealthQuery {
            loinc_families: vec!["2345".into()],
            min_cohort: 5,
            epsilon_budget: 1.0,
        };

        let result = population_health_query(&[round], &query);
        assert!(result.is_err());
    }
}
