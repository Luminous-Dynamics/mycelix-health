//! Privacy Budget Accounting
//!
//! Provides rigorous tracking of privacy budget consumption using
//! composition theorems from differential privacy theory.
//!
//! # Why Budget Tracking Matters
//!
//! Each differentially private query consumes some privacy budget (ε, δ).
//! Without tracking, an adversary could make many queries and combine
//! results to defeat privacy guarantees.
//!
//! # Composition Theorems
//!
//! ## Basic Composition
//! If we answer k queries, each with ε-DP guarantee:
//! - Total privacy loss: kε-DP
//! - Simple but loose bound
//!
//! ## Advanced Composition (Dwork et al., 2010)
//! For k queries each with ε-DP, and any δ' > 0:
//! - Total privacy: (ε', kδ + δ')-DP
//! - Where ε' = √(2k ln(1/δ')) · ε + k · ε · (e^ε - 1)
//! - Much tighter for many queries with small ε
//!
//! # Budget Exhaustion
//!
//! When the budget is exhausted, no more queries can be answered.
//! This is a fundamental limit, not a bug - it's what makes DP meaningful.

use super::validation::{validate_epsilon, DpValidationError};
use serde::{Deserialize, Serialize};

/// Error type for budget operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetError {
    /// Insufficient budget remaining
    Exhausted { required: f64, remaining: f64 },
    /// Invalid budget parameters
    InvalidParameter(String),
    /// Budget operation failed
    OperationFailed(String),
}

impl std::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetError::Exhausted {
                required,
                remaining,
            } => {
                write!(
                    f,
                    "Privacy budget exhausted: need ε={:.4}, have ε={:.4}",
                    required, remaining
                )
            }
            BudgetError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            BudgetError::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
        }
    }
}

impl From<DpValidationError> for BudgetError {
    fn from(e: DpValidationError) -> Self {
        BudgetError::InvalidParameter(e.to_string())
    }
}

/// Composition theorem selection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CompositionTheorem {
    /// Basic composition: total ε = sum of individual ε values
    Basic,
    /// Advanced composition with specified δ' for tighter bounds
    Advanced { delta_prime: f64 },
}

/// Privacy budget account for tracking consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAccount {
    /// Total epsilon budget allocated
    total_epsilon: f64,
    /// Total delta budget allocated (for (ε, δ)-DP queries)
    total_delta: f64,
    /// Epsilon consumed so far (under basic composition)
    consumed_epsilon: f64,
    /// Delta consumed so far
    consumed_delta: f64,
    /// Number of queries answered
    query_count: u32,
    /// Individual epsilon values for advanced composition
    epsilon_history: Vec<f64>,
    /// Composition method to use
    composition: CompositionTheorem,
}

impl BudgetAccount {
    /// Create a new budget account with specified limits
    ///
    /// # Arguments
    /// * `total_epsilon` - Maximum epsilon budget
    ///
    /// # Returns
    /// A new BudgetAccount with basic composition
    pub fn new(total_epsilon: f64) -> Self {
        Self {
            total_epsilon,
            total_delta: 0.0,
            consumed_epsilon: 0.0,
            consumed_delta: 0.0,
            query_count: 0,
            epsilon_history: Vec::new(),
            composition: CompositionTheorem::Basic,
        }
    }

    /// Create a new budget account for (ε, δ)-DP with advanced composition
    ///
    /// # Arguments
    /// * `total_epsilon` - Maximum epsilon budget
    /// * `total_delta` - Maximum delta budget
    /// * `delta_prime` - Delta parameter for advanced composition
    pub fn new_with_delta(total_epsilon: f64, total_delta: f64, delta_prime: f64) -> Self {
        Self {
            total_epsilon,
            total_delta,
            consumed_epsilon: 0.0,
            consumed_delta: 0.0,
            query_count: 0,
            epsilon_history: Vec::new(),
            composition: CompositionTheorem::Advanced { delta_prime },
        }
    }

    /// Get the total epsilon budget
    pub fn total_epsilon(&self) -> f64 {
        self.total_epsilon
    }

    /// Get the total delta budget
    pub fn total_delta(&self) -> f64 {
        self.total_delta
    }

    /// Get the remaining epsilon budget (under current composition)
    pub fn remaining_epsilon(&self) -> f64 {
        let effective_consumed = self.effective_epsilon_consumed();
        (self.total_epsilon - effective_consumed).max(0.0)
    }

    /// Get the remaining delta budget
    pub fn remaining_delta(&self) -> f64 {
        (self.total_delta - self.consumed_delta).max(0.0)
    }

    /// Get the number of queries answered
    pub fn query_count(&self) -> u32 {
        self.query_count
    }

    /// Compute effective epsilon consumed using the configured composition theorem
    pub fn effective_epsilon_consumed(&self) -> f64 {
        match self.composition {
            CompositionTheorem::Basic => self.consumed_epsilon,
            CompositionTheorem::Advanced { delta_prime } => {
                self.advanced_composition_epsilon(delta_prime)
            }
        }
    }

    /// Compute epsilon under advanced composition
    ///
    /// ε' = √(2k ln(1/δ')) · ε_avg + k · ε_avg · (e^ε_avg - 1)
    ///
    /// For heterogeneous ε values, we use RMS epsilon.
    fn advanced_composition_epsilon(&self, delta_prime: f64) -> f64 {
        if self.epsilon_history.is_empty() {
            return 0.0;
        }

        let k = self.epsilon_history.len() as f64;

        // Compute RMS epsilon for heterogeneous case
        let sum_sq: f64 = self.epsilon_history.iter().map(|e| e * e).sum();
        let rms_epsilon = (sum_sq / k).sqrt();

        // Advanced composition formula
        let term1 = (2.0 * k * (1.0 / delta_prime).ln()).sqrt() * rms_epsilon;
        let term2 = k * rms_epsilon * (rms_epsilon.exp() - 1.0);

        term1 + term2
    }

    /// Check if there's sufficient budget for a query
    pub fn has_budget(&self, epsilon: f64, delta: f64) -> bool {
        // Simulate consumption to check
        let mut test_account = self.clone();
        test_account.epsilon_history.push(epsilon);
        test_account.consumed_epsilon += epsilon;
        test_account.consumed_delta += delta;

        let effective = test_account.effective_epsilon_consumed();
        effective <= self.total_epsilon && test_account.consumed_delta <= self.total_delta
    }

    /// Check and consume budget for a query
    ///
    /// # Arguments
    /// * `epsilon` - Privacy cost of the query
    ///
    /// # Returns
    /// * `Ok(())` if budget was consumed
    /// * `Err(BudgetError::Exhausted)` if insufficient budget
    pub fn check_and_consume(&mut self, epsilon: f64) -> Result<(), BudgetError> {
        self.check_and_consume_with_delta(epsilon, 0.0)
    }

    /// Check and consume budget for an (ε, δ)-DP query
    ///
    /// # Arguments
    /// * `epsilon` - Privacy cost (epsilon)
    /// * `delta` - Privacy cost (delta)
    ///
    /// # Returns
    /// * `Ok(())` if budget was consumed
    /// * `Err(BudgetError::Exhausted)` if insufficient budget
    pub fn check_and_consume_with_delta(
        &mut self,
        epsilon: f64,
        delta: f64,
    ) -> Result<(), BudgetError> {
        validate_epsilon(epsilon)?;
        if delta < 0.0 {
            return Err(BudgetError::InvalidParameter(
                "Delta must be non-negative".to_string(),
            ));
        }

        // Check delta first (simple additive)
        if self.consumed_delta + delta > self.total_delta && self.total_delta > 0.0 {
            return Err(BudgetError::Exhausted {
                required: delta,
                remaining: self.remaining_delta(),
            });
        }

        // Check epsilon under composition
        if !self.has_budget(epsilon, delta) {
            return Err(BudgetError::Exhausted {
                required: epsilon,
                remaining: self.remaining_epsilon(),
            });
        }

        // Consume the budget
        self.epsilon_history.push(epsilon);
        self.consumed_epsilon += epsilon;
        self.consumed_delta += delta;
        self.query_count += 1;

        Ok(())
    }

    /// Reset the budget (e.g., for a new time period)
    pub fn reset(&mut self) {
        self.consumed_epsilon = 0.0;
        self.consumed_delta = 0.0;
        self.query_count = 0;
        self.epsilon_history.clear();
    }

    /// Serialize the budget state for persistence
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Deserialize budget state
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BudgetError> {
        serde_json::from_slice(bytes)
            .map_err(|e| BudgetError::OperationFailed(format!("Deserialization failed: {}", e)))
    }
}

/// Compute total privacy loss under basic composition
///
/// For k (ε_i, δ_i)-DP mechanisms:
/// - Total ε = Σε_i
/// - Total δ = Σδ_i
pub fn basic_composition(epsilons: &[f64]) -> f64 {
    epsilons.iter().sum()
}

/// Compute total privacy loss under advanced composition
///
/// For k mechanisms each with ε-DP (homogeneous case):
/// ε' = √(2k ln(1/δ')) · ε + k · ε · (e^ε - 1)
///
/// # Arguments
/// * `epsilon` - Privacy parameter for each query
/// * `k` - Number of queries
/// * `delta_prime` - Failure probability for composition
pub fn advanced_composition_homogeneous(epsilon: f64, k: usize, delta_prime: f64) -> f64 {
    let k_f = k as f64;
    let term1 = (2.0 * k_f * (1.0 / delta_prime).ln()).sqrt() * epsilon;
    let term2 = k_f * epsilon * (epsilon.exp() - 1.0);
    term1 + term2
}

/// Compare basic vs advanced composition for a given scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionComparison {
    /// Number of queries
    pub query_count: usize,
    /// Per-query epsilon
    pub per_query_epsilon: f64,
    /// Total epsilon under basic composition
    pub basic_total: f64,
    /// Total epsilon under advanced composition
    pub advanced_total: f64,
    /// Savings ratio (basic / advanced)
    pub savings_ratio: f64,
}

/// Compare composition theorems
pub fn compare_compositions(epsilon: f64, k: usize, delta_prime: f64) -> CompositionComparison {
    let basic = epsilon * k as f64;
    let advanced = advanced_composition_homogeneous(epsilon, k, delta_prime);

    CompositionComparison {
        query_count: k,
        per_query_epsilon: epsilon,
        basic_total: basic,
        advanced_total: advanced,
        savings_ratio: basic / advanced,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_account_creation() {
        let budget = BudgetAccount::new(1.0);
        assert_eq!(budget.total_epsilon(), 1.0);
        assert_eq!(budget.remaining_epsilon(), 1.0);
        assert_eq!(budget.query_count(), 0);
    }

    #[test]
    fn test_budget_consumption() {
        let mut budget = BudgetAccount::new(1.0);

        budget.check_and_consume(0.1).unwrap();
        assert_eq!(budget.query_count(), 1);
        assert!((budget.remaining_epsilon() - 0.9).abs() < 1e-10);

        budget.check_and_consume(0.2).unwrap();
        assert_eq!(budget.query_count(), 2);
        assert!((budget.remaining_epsilon() - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_budget_exhaustion() {
        let mut budget = BudgetAccount::new(0.5);

        budget.check_and_consume(0.3).unwrap();
        budget.check_and_consume(0.15).unwrap();

        // This should fail - only 0.05 remaining
        let result = budget.check_and_consume(0.1);
        assert!(matches!(result, Err(BudgetError::Exhausted { .. })));
    }

    #[test]
    fn test_budget_monotonicity() {
        let mut budget = BudgetAccount::new(10.0);
        let mut prev_remaining = budget.remaining_epsilon();

        for _ in 0..50 {
            if budget.check_and_consume(0.1).is_ok() {
                let current = budget.remaining_epsilon();
                assert!(current <= prev_remaining, "Budget should never increase");
                prev_remaining = current;
            }
        }
    }

    #[test]
    fn test_basic_composition() {
        let epsilons = vec![0.1, 0.2, 0.3];
        let total = basic_composition(&epsilons);
        assert!((total - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_advanced_composition_tighter() {
        let epsilon = 0.1;
        let k = 100;
        let delta_prime = 1e-6;

        let basic = epsilon * k as f64; // 10.0
        let advanced = advanced_composition_homogeneous(epsilon, k, delta_prime);

        // Advanced should be tighter than basic
        assert!(
            advanced < basic,
            "Advanced {} should be < basic {}",
            advanced,
            basic
        );

        // For these parameters:
        // term1 = √(2 * 100 * ln(1e6)) * 0.1 ≈ 5.25
        // term2 = 100 * 0.1 * (e^0.1 - 1) ≈ 1.05
        // total ≈ 6.3, which is < 10.0 (basic)
        assert!(advanced < 8.0, "Advanced {} should be < 8.0", advanced);
    }

    #[test]
    fn test_compare_compositions() {
        let comparison = compare_compositions(0.1, 100, 1e-6);

        assert_eq!(comparison.query_count, 100);
        assert_eq!(comparison.per_query_epsilon, 0.1);
        assert!((comparison.basic_total - 10.0).abs() < 1e-10);
        // Advanced composition saves about 1.5x for these parameters
        assert!(comparison.savings_ratio > 1.3, "Should save at least 1.3x");
        assert!(
            comparison.advanced_total < comparison.basic_total,
            "Advanced should be tighter"
        );
    }

    #[test]
    fn test_budget_serialization() {
        let mut budget = BudgetAccount::new(5.0);
        budget.check_and_consume(0.5).unwrap();
        budget.check_and_consume(0.3).unwrap();

        let bytes = budget.to_bytes();
        let restored = BudgetAccount::from_bytes(&bytes).unwrap();

        assert_eq!(restored.total_epsilon(), budget.total_epsilon());
        assert_eq!(restored.query_count(), budget.query_count());
        assert!((restored.remaining_epsilon() - budget.remaining_epsilon()).abs() < 1e-10);
    }

    #[test]
    fn test_advanced_composition_account() {
        let mut budget = BudgetAccount::new_with_delta(5.0, 1e-5, 1e-6);

        // Consume 50 queries of ε=0.1 each
        for _ in 0..50 {
            budget.check_and_consume(0.1).unwrap();
        }

        // Under basic composition, we'd have used 5.0 (exhausted)
        // Under advanced, we should have budget remaining
        assert!(
            budget.remaining_epsilon() > 0.0,
            "Advanced composition should leave budget: remaining = {}",
            budget.remaining_epsilon()
        );

        // We should be able to answer more queries
        assert!(
            budget.has_budget(0.1, 0.0),
            "Should have budget for another query"
        );
    }

    #[test]
    fn test_reset() {
        let mut budget = BudgetAccount::new(1.0);
        budget.check_and_consume(0.5).unwrap();
        budget.check_and_consume(0.3).unwrap();

        budget.reset();

        assert_eq!(budget.remaining_epsilon(), 1.0);
        assert_eq!(budget.query_count(), 0);
    }
}

/// Property-based tests using proptest
#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Budget should NEVER increase after any consumption
        #[test]
        fn budget_monotonicity(
            total in 1.0..100.0f64,
            consumptions in proptest::collection::vec(0.01..1.0f64, 1..50)
        ) {
            let mut budget = BudgetAccount::new(total);
            let mut prev_remaining = budget.remaining_epsilon();

            for epsilon in consumptions {
                if budget.check_and_consume(epsilon).is_ok() {
                    let current = budget.remaining_epsilon();
                    prop_assert!(
                        current <= prev_remaining,
                        "Budget increased from {} to {} after consuming {}",
                        prev_remaining, current, epsilon
                    );
                    prev_remaining = current;
                }
            }
        }

        /// Budget remaining should always be non-negative
        #[test]
        fn budget_never_negative(
            total in 0.1..10.0f64,
            consumptions in proptest::collection::vec(0.01..2.0f64, 1..100)
        ) {
            let mut budget = BudgetAccount::new(total);

            for epsilon in consumptions {
                let _ = budget.check_and_consume(epsilon);
                prop_assert!(
                    budget.remaining_epsilon() >= 0.0,
                    "Budget went negative: {}",
                    budget.remaining_epsilon()
                );
            }
        }

        /// Basic composition should equal sum of epsilons
        #[test]
        fn basic_composition_sums_correctly(
            epsilons in proptest::collection::vec(0.01..1.0f64, 1..20)
        ) {
            let total = basic_composition(&epsilons);
            let sum: f64 = epsilons.iter().sum();
            prop_assert!(
                (total - sum).abs() < 1e-10,
                "Basic composition {} != sum {}",
                total, sum
            );
        }

        /// Advanced composition should be tighter than basic for many queries with small epsilon
        /// Note: Advanced composition is NOT always tighter - it's only beneficial when k is large
        /// and epsilon is small. For small k or large epsilon, basic may be tighter.
        #[test]
        fn advanced_tighter_for_many_small_queries(
            epsilon in 0.01..0.15f64,  // Small epsilon
            k in 50..200usize,          // Many queries
            delta_prime in 1e-9..1e-5f64
        ) {
            let basic = epsilon * k as f64;
            let advanced = advanced_composition_homogeneous(epsilon, k, delta_prime);

            // For many queries with small epsilon, advanced should be tighter
            prop_assert!(
                advanced <= basic * 1.01, // Allow 1% margin for numerical precision
                "Advanced {} > basic {} for ε={}, k={}, δ'={}",
                advanced, basic, epsilon, k, delta_prime
            );
        }

        /// Query count should match number of successful consumptions
        #[test]
        fn query_count_accurate(
            total in 1.0..10.0f64,
            consumptions in proptest::collection::vec(0.1..0.5f64, 1..30)
        ) {
            let mut budget = BudgetAccount::new(total);
            let mut successful = 0u32;

            for epsilon in consumptions {
                if budget.check_and_consume(epsilon).is_ok() {
                    successful += 1;
                }
            }

            prop_assert_eq!(
                budget.query_count(),
                successful,
                "Query count {} != successful consumptions {}",
                budget.query_count(), successful
            );
        }

        /// Serialization should preserve all state
        #[test]
        fn serialization_roundtrip(
            total in 1.0..100.0f64,
            consumptions in proptest::collection::vec(0.01..0.5f64, 0..20)
        ) {
            let mut budget = BudgetAccount::new(total);
            for epsilon in consumptions {
                let _ = budget.check_and_consume(epsilon);
            }

            let bytes = budget.to_bytes();
            let restored = BudgetAccount::from_bytes(&bytes).unwrap();

            prop_assert!(
                (restored.total_epsilon() - budget.total_epsilon()).abs() < 1e-10,
                "Total epsilon mismatch"
            );
            prop_assert!(
                (restored.remaining_epsilon() - budget.remaining_epsilon()).abs() < 1e-10,
                "Remaining epsilon mismatch"
            );
            prop_assert_eq!(
                restored.query_count(),
                budget.query_count(),
                "Query count mismatch"
            );
        }
    }
}
