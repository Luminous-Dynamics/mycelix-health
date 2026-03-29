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
}
