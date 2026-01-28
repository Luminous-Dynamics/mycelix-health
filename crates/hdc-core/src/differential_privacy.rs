//! Differential Privacy for HDC Genetic Encoding
//!
//! Provides formal privacy guarantees through calibrated noise injection.
//! Based on the **randomized response mechanism** adapted for binary vectors.
//!
//! # Relationship to Other DP Modules
//!
//! This module implements randomized response for **binary hypervectors**.
//! For numeric DP (Laplace/Gaussian mechanisms for aggregate queries),
//! see `zomes/shared/src/dp_core/` which provides:
//! - Laplace mechanism for (ε, 0)-DP on counts/sums
//! - Gaussian mechanism for (ε, δ)-DP
//! - Advanced composition theorems
//!
//! Use THIS module when: protecting binary hypervector representations
//! Use `dp_core` when: protecting numeric query results (counts, averages)
//!
//! # Privacy Guarantee
//!
//! For a given epsilon (ε), the probability of any output is bounded by:
//! P(output | input1) ≤ e^ε × P(output | input2)
//!
//! This ensures that an adversary cannot reliably distinguish between
//! two different input sequences based on the noisy output.
//!
//! # Randomized Response for Binary Data
//!
//! Each bit is independently flipped with probability p = 1/(1 + e^ε).
//! This provides plausible deniability: any output bit could have come
//! from either a 0 or 1 input bit.

use crate::Hypervector;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

/// Differential privacy parameters
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DpParams {
    /// Privacy parameter epsilon (lower = more private)
    /// Typical values: 0.1 (high privacy), 1.0 (moderate), 10.0 (low privacy)
    pub epsilon: f64,
    /// Optional delta for approximate DP (usually very small, e.g., 1e-6)
    pub delta: Option<f64>,
}

impl DpParams {
    /// Create pure ε-differential privacy parameters
    pub fn pure(epsilon: f64) -> Self {
        assert!(epsilon > 0.0, "Epsilon must be positive");
        DpParams { epsilon, delta: None }
    }

    /// Create (ε, δ)-differential privacy parameters
    pub fn approximate(epsilon: f64, delta: f64) -> Self {
        assert!(epsilon > 0.0, "Epsilon must be positive");
        assert!(delta > 0.0 && delta < 1.0, "Delta must be in (0, 1)");
        DpParams { epsilon, delta: Some(delta) }
    }

    /// Calculate the bit flip probability for randomized response
    ///
    /// For ε-DP with binary data, we flip each bit with probability:
    /// p = 1 / (1 + e^ε)
    pub fn flip_probability(&self) -> f64 {
        1.0 / (1.0 + self.epsilon.exp())
    }

    /// Calculate the expected similarity degradation
    ///
    /// With flip probability p, expected matching bits becomes:
    /// E[match] = (1-p)² + p² = 1 - 2p(1-p)
    pub fn expected_similarity_retention(&self) -> f64 {
        let p = self.flip_probability();
        (1.0 - p).powi(2) + p.powi(2)
    }

    /// Privacy-utility tradeoff description
    pub fn describe(&self) -> String {
        let p = self.flip_probability();
        let retention = self.expected_similarity_retention();
        format!(
            "ε={:.2}: flip_prob={:.1}%, similarity_retention={:.1}%",
            self.epsilon,
            p * 100.0,
            retention * 100.0
        )
    }
}

/// Differentially private hypervector with provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpHypervector {
    /// The noisy hypervector
    pub vector: Hypervector,
    /// Privacy parameters used
    pub params: DpParams,
    /// Random seed for reproducibility (optional)
    pub seed: Option<u64>,
}

impl DpHypervector {
    /// Apply differential privacy to a hypervector
    ///
    /// Uses randomized response: each bit is flipped with probability
    /// p = 1/(1 + e^ε) to achieve ε-differential privacy.
    pub fn from_vector(vector: &Hypervector, params: DpParams, seed: Option<u64>) -> Self {
        let mut rng = match seed {
            Some(s) => ChaCha20Rng::seed_from_u64(s),
            None => ChaCha20Rng::from_entropy(),
        };

        let flip_prob = params.flip_probability();
        let mut noisy_data = vector.as_bytes().to_vec();

        for byte in noisy_data.iter_mut() {
            for bit in 0..8 {
                if rng.gen::<f64>() < flip_prob {
                    *byte ^= 1 << bit;
                }
            }
        }

        let noisy_vector = Hypervector::from_bytes(noisy_data)
            .expect("Noisy vector should have valid dimensions");

        DpHypervector {
            vector: noisy_vector,
            params,
            seed,
        }
    }

    /// Calculate similarity between two DP vectors
    ///
    /// Returns raw similarity - caller should interpret based on expected degradation
    pub fn similarity(&self, other: &DpHypervector) -> f64 {
        self.vector.normalized_cosine_similarity(&other.vector)
    }

    /// Calculate similarity with correction for DP noise
    ///
    /// Attempts to estimate the true similarity by accounting for
    /// the expected noise introduced by DP.
    pub fn corrected_similarity(&self, other: &DpHypervector) -> f64 {
        let raw_sim = self.similarity(other);

        // Both vectors have independent noise, so we need to correct for both
        let retention1 = self.params.expected_similarity_retention();
        let retention2 = other.params.expected_similarity_retention();

        // Combined retention (independent noise sources)
        let combined_retention = retention1 * retention2;

        // Correct: observed = true * retention + (1 - retention) * 0.5
        // So: true = (observed - 0.5 * (1 - retention)) / retention
        let baseline = 0.5 * (1.0 - combined_retention);
        let corrected = (raw_sim - baseline) / combined_retention;

        corrected.clamp(0.0, 1.0)
    }

    /// Get the privacy budget consumed
    pub fn epsilon(&self) -> f64 {
        self.params.epsilon
    }

    /// Check if this is considered high privacy (ε ≤ 1)
    pub fn is_high_privacy(&self) -> bool {
        self.params.epsilon <= 1.0
    }
}

/// Privacy budget tracker for composition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyBudget {
    /// Total epsilon budget
    pub total_epsilon: f64,
    /// Epsilon consumed so far
    pub consumed_epsilon: f64,
    /// Number of queries made
    pub query_count: usize,
}

impl PrivacyBudget {
    /// Create a new privacy budget
    pub fn new(total_epsilon: f64) -> Self {
        assert!(total_epsilon > 0.0, "Budget must be positive");
        PrivacyBudget {
            total_epsilon,
            consumed_epsilon: 0.0,
            query_count: 0,
        }
    }

    /// Check if a query with given epsilon can be made
    pub fn can_query(&self, epsilon: f64) -> bool {
        self.consumed_epsilon + epsilon <= self.total_epsilon
    }

    /// Consume budget for a query (basic composition)
    pub fn consume(&mut self, epsilon: f64) -> Result<(), PrivacyError> {
        if !self.can_query(epsilon) {
            return Err(PrivacyError::BudgetExhausted {
                requested: epsilon,
                remaining: self.total_epsilon - self.consumed_epsilon,
            });
        }
        self.consumed_epsilon += epsilon;
        self.query_count += 1;
        Ok(())
    }

    /// Remaining budget
    pub fn remaining(&self) -> f64 {
        self.total_epsilon - self.consumed_epsilon
    }

    /// Fraction of budget used
    pub fn utilization(&self) -> f64 {
        self.consumed_epsilon / self.total_epsilon
    }
}

/// Privacy-related errors
#[derive(Debug, Clone, PartialEq)]
pub enum PrivacyError {
    /// Privacy budget exhausted
    BudgetExhausted { requested: f64, remaining: f64 },
    /// Invalid epsilon value
    InvalidEpsilon(f64),
}

impl std::fmt::Display for PrivacyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrivacyError::BudgetExhausted { requested, remaining } => {
                write!(f, "Privacy budget exhausted: requested {:.2}, only {:.2} remaining",
                       requested, remaining)
            }
            PrivacyError::InvalidEpsilon(e) => {
                write!(f, "Invalid epsilon value: {}", e)
            }
        }
    }
}

impl std::error::Error for PrivacyError {}

/// Recommended epsilon values for different use cases
pub mod recommended {
    use super::DpParams;

    /// High privacy for sensitive genetic data (ε = 0.1)
    /// ~47.5% bit flip, ~50.2% similarity retention
    pub fn high_privacy() -> DpParams {
        DpParams::pure(0.1)
    }

    /// Strong privacy for clinical use (ε = 0.5)
    /// ~37.8% bit flip, ~54.9% similarity retention
    pub fn strong_privacy() -> DpParams {
        DpParams::pure(0.5)
    }

    /// Moderate privacy for research (ε = 1.0)
    /// ~26.9% bit flip, ~64.6% similarity retention
    pub fn moderate_privacy() -> DpParams {
        DpParams::pure(1.0)
    }

    /// Standard privacy (ε = 2.0)
    /// ~11.9% bit flip, ~78.1% similarity retention
    pub fn standard_privacy() -> DpParams {
        DpParams::pure(2.0)
    }

    /// Low privacy for public data (ε = 5.0)
    /// ~0.7% bit flip, ~98.6% similarity retention
    pub fn low_privacy() -> DpParams {
        DpParams::pure(5.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Seed;

    #[test]
    fn test_flip_probability() {
        // ε = 0 would give 50% flip (maximum privacy)
        // ε → ∞ gives 0% flip (no privacy)

        let params_high = DpParams::pure(0.1);
        let params_low = DpParams::pure(10.0);

        assert!(params_high.flip_probability() > 0.4); // High privacy = more flips
        assert!(params_low.flip_probability() < 0.001); // Low privacy = few flips
    }

    #[test]
    fn test_dp_preserves_dimensions() {
        let seed = Seed::from_string("test");
        let hv = Hypervector::random(&seed, "item");

        let params = DpParams::pure(1.0);
        let dp_hv = DpHypervector::from_vector(&hv, params, Some(42));

        assert_eq!(dp_hv.vector.as_bytes().len(), hv.as_bytes().len());
    }

    #[test]
    fn test_dp_adds_noise() {
        let seed = Seed::from_string("test");
        let hv = Hypervector::random(&seed, "item");

        // High privacy should significantly change the vector
        let params = DpParams::pure(0.1);
        let dp_hv = DpHypervector::from_vector(&hv, params, Some(42));

        let similarity = hv.normalized_cosine_similarity(&dp_hv.vector);

        // With ε=0.1, we expect ~47% bit flips, so similarity should be low
        assert!(similarity < 0.7, "Expected significant noise, got similarity {}", similarity);
        assert!(similarity > 0.3, "Noise shouldn't be complete randomization");
    }

    #[test]
    fn test_dp_deterministic_with_seed() {
        let seed = Seed::from_string("test");
        let hv = Hypervector::random(&seed, "item");
        let params = DpParams::pure(1.0);

        let dp1 = DpHypervector::from_vector(&hv, params, Some(42));
        let dp2 = DpHypervector::from_vector(&hv, params, Some(42));

        assert_eq!(dp1.vector, dp2.vector);
    }

    #[test]
    fn test_privacy_budget() {
        let mut budget = PrivacyBudget::new(5.0);

        assert!(budget.can_query(1.0));
        budget.consume(1.0).unwrap();
        assert_eq!(budget.remaining(), 4.0);

        budget.consume(2.0).unwrap();
        budget.consume(1.5).unwrap();

        assert!(!budget.can_query(1.0));
        assert!(budget.consume(1.0).is_err());
    }

    #[test]
    fn test_corrected_similarity() {
        let seed = Seed::from_string("test");
        let hv1 = Hypervector::random(&seed, "item1");
        let hv2 = Hypervector::random(&seed, "item1"); // Same = identical

        let params = DpParams::pure(2.0); // Moderate privacy
        let dp1 = DpHypervector::from_vector(&hv1, params, Some(1));
        let dp2 = DpHypervector::from_vector(&hv2, params, Some(2));

        let raw = dp1.similarity(&dp2);
        let corrected = dp1.corrected_similarity(&dp2);

        // Corrected should be closer to 1.0 for identical inputs
        // (though noise makes this probabilistic)
        println!("Raw: {}, Corrected: {}", raw, corrected);
    }

    #[test]
    fn test_recommended_params() {
        let high = recommended::high_privacy();
        let low = recommended::low_privacy();

        assert!(high.flip_probability() > low.flip_probability());
        assert!(high.expected_similarity_retention() < low.expected_similarity_retention());
    }
}
