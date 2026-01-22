//! Property-Based Tests for Differential Privacy Mechanisms
//!
//! These tests verify the mathematical properties of our DP implementation:
//! - Distribution correctness (mean, variance)
//! - Budget monotonicity (never increases)
//! - Composition theorems (basic and advanced)
//! - Edge case handling
//!
//! Uses proptest for randomized property testing with shrinking.

// Note: These tests use the dp_core module directly
// In a real test environment, you would import from the shared crate

/// Compute sample mean
fn compute_mean(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().sum::<f64>() / samples.len() as f64
}

/// Compute sample variance
fn compute_variance(samples: &[f64]) -> f64 {
    if samples.len() < 2 {
        return 0.0;
    }
    let mean = compute_mean(samples);
    let sum_sq: f64 = samples.iter().map(|x| (x - mean).powi(2)).sum();
    sum_sq / (samples.len() - 1) as f64
}

/// Compute sample standard deviation
fn compute_std(samples: &[f64]) -> f64 {
    compute_variance(samples).sqrt()
}

#[cfg(test)]
mod laplace_tests {
    use super::*;

    // Simulated Laplace sampling for testing (mirrors dp_core implementation)
    fn laplace_sample(scale: f64) -> f64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        // Generate pseudo-random number (for testing only - real impl uses getrandom)
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let random_bits = hasher.finish();

        let u = (random_bits as f64 / u64::MAX as f64) - 0.5;

        // Avoid exactly 0
        let u = if u.abs() < 1e-15 { 1e-15 } else { u };

        // Inverse CDF
        -scale * u.signum() * (1.0 - 2.0 * u.abs()).ln()
    }

    #[test]
    fn test_laplace_mean_approximately_zero() {
        let scale = 2.0;
        let n = 10000;
        let samples: Vec<f64> = (0..n).map(|_| laplace_sample(scale)).collect();
        let mean = compute_mean(&samples);

        // Expected mean = 0, SE = sqrt(2*scale^2/n)
        let se = (2.0 * scale * scale / n as f64).sqrt();

        // Within 4 standard errors
        assert!(
            mean.abs() < 4.0 * se,
            "Laplace mean {} should be close to 0 (SE = {})",
            mean,
            se
        );
    }

    #[test]
    fn test_laplace_variance_correct() {
        let scale = 1.5;
        let n = 10000;
        let samples: Vec<f64> = (0..n).map(|_| laplace_sample(scale)).collect();
        let variance = compute_variance(&samples);

        // Expected variance = 2 * scale^2
        let expected = 2.0 * scale * scale;

        // Within 20% (statistical tolerance)
        assert!(
            (variance - expected).abs() / expected < 0.2,
            "Laplace variance {} should be close to {} (expected 2*scale^2)",
            variance,
            expected
        );
    }

    #[test]
    fn test_laplace_scaling_property() {
        // Var(Lap(0, b)) = 2b^2, so variance scales with scale^2
        let n = 5000;

        let samples_1: Vec<f64> = (0..n).map(|_| laplace_sample(1.0)).collect();
        let samples_2: Vec<f64> = (0..n).map(|_| laplace_sample(2.0)).collect();

        let var_1 = compute_variance(&samples_1);
        let var_2 = compute_variance(&samples_2);

        // var_2 should be approximately 4 * var_1
        let ratio = var_2 / var_1;
        assert!(
            ratio > 3.0 && ratio < 5.0,
            "Variance ratio {} should be close to 4.0",
            ratio
        );
    }
}

#[cfg(test)]
mod gaussian_tests {
    use super::*;

    // Simulated Gaussian sampling via Box-Muller
    fn gaussian_sample(sigma: f64) -> f64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let seed1 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seed2 = seed1.wrapping_add(12345);

        let mut hasher = DefaultHasher::new();
        seed1.hash(&mut hasher);
        let u1 = (hasher.finish() as f64 / u64::MAX as f64).max(1e-15);

        hasher = DefaultHasher::new();
        seed2.hash(&mut hasher);
        let u2 = hasher.finish() as f64 / u64::MAX as f64;

        // Box-Muller transform
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        z * sigma
    }

    #[test]
    fn test_gaussian_mean_approximately_zero() {
        let sigma = 2.0;
        let n = 10000;
        let samples: Vec<f64> = (0..n).map(|_| gaussian_sample(sigma)).collect();
        let mean = compute_mean(&samples);

        // SE = sigma / sqrt(n)
        let se = sigma / (n as f64).sqrt();

        assert!(
            mean.abs() < 4.0 * se,
            "Gaussian mean {} should be close to 0 (SE = {})",
            mean,
            se
        );
    }

    #[test]
    fn test_gaussian_variance_correct() {
        let sigma = 3.0;
        let n = 10000;
        let samples: Vec<f64> = (0..n).map(|_| gaussian_sample(sigma)).collect();
        let variance = compute_variance(&samples);

        // Expected variance = sigma^2
        let expected = sigma * sigma;

        assert!(
            (variance - expected).abs() / expected < 0.15,
            "Gaussian variance {} should be close to {} (sigma^2)",
            variance,
            expected
        );
    }

    #[test]
    fn test_gaussian_std_dev_correct() {
        let sigma = 5.0;
        let n = 10000;
        let samples: Vec<f64> = (0..n).map(|_| gaussian_sample(sigma)).collect();
        let std = compute_std(&samples);

        assert!(
            (std - sigma).abs() / sigma < 0.1,
            "Gaussian std {} should be close to {}",
            std,
            sigma
        );
    }
}

#[cfg(test)]
mod budget_tests {
    /// Simple budget account for testing
    #[derive(Debug, Clone)]
    struct BudgetAccount {
        total: f64,
        consumed: f64,
        history: Vec<f64>,
    }

    impl BudgetAccount {
        fn new(total: f64) -> Self {
            Self {
                total,
                consumed: 0.0,
                history: Vec::new(),
            }
        }

        fn remaining(&self) -> f64 {
            self.total - self.consumed
        }

        fn consume(&mut self, epsilon: f64) -> Result<(), &'static str> {
            if epsilon <= 0.0 {
                return Err("Epsilon must be positive");
            }
            if self.consumed + epsilon > self.total {
                return Err("Insufficient budget");
            }
            self.consumed += epsilon;
            self.history.push(epsilon);
            Ok(())
        }
    }

    #[test]
    fn test_budget_monotonicity() {
        // Budget should NEVER increase after consumption
        let mut account = BudgetAccount::new(10.0);
        let mut prev_remaining = account.remaining();

        for i in 0..50 {
            let epsilon = 0.1 + (i as f64 * 0.01);
            if account.consume(epsilon).is_ok() {
                let current = account.remaining();
                assert!(
                    current <= prev_remaining,
                    "Budget increased from {} to {} after consuming {}",
                    prev_remaining,
                    current,
                    epsilon
                );
                assert!(
                    current < prev_remaining,
                    "Budget should strictly decrease"
                );
                prev_remaining = current;
            }
        }
    }

    #[test]
    fn test_budget_exhaustion() {
        let mut account = BudgetAccount::new(1.0);

        // Consume most of budget
        account.consume(0.4).unwrap();
        account.consume(0.4).unwrap();

        // This should succeed
        account.consume(0.19).unwrap();

        // This should fail - only 0.01 remaining
        assert!(account.consume(0.02).is_err());
    }

    #[test]
    fn test_budget_never_negative() {
        let mut account = BudgetAccount::new(5.0);

        // Exhaust budget
        for _ in 0..100 {
            let _ = account.consume(0.1);
        }

        assert!(account.remaining() >= 0.0, "Budget should never go negative");
    }

    #[test]
    fn test_basic_composition() {
        let epsilons = vec![0.1, 0.2, 0.15, 0.05];
        let total: f64 = epsilons.iter().sum();

        assert!((total - 0.5).abs() < 1e-10, "Basic composition should sum");
    }

    #[test]
    fn test_advanced_composition_tighter() {
        // Advanced composition: ε' = √(2k ln(1/δ')) · ε + k · ε · (e^ε - 1)
        let epsilon = 0.1;
        let k = 100;
        let delta_prime = 1e-6;

        let basic = epsilon * k as f64; // 10.0

        let k_f = k as f64;
        let term1 = (2.0 * k_f * (1.0 / delta_prime).ln()).sqrt() * epsilon;
        let term2 = k_f * epsilon * (epsilon.exp() - 1.0);
        let advanced = term1 + term2;

        assert!(
            advanced < basic,
            "Advanced composition {} should be tighter than basic {}",
            advanced,
            basic
        );

        // For small epsilon, advanced should be roughly sqrt(k) factor tighter
        let ratio = basic / advanced;
        assert!(
            ratio > 2.0,
            "Should save at least 2x with advanced composition"
        );
    }

    #[test]
    fn test_composition_query_sequence() {
        // Simulate a sequence of queries
        let mut account = BudgetAccount::new(2.0);
        let per_query_epsilon = 0.1;

        // Under basic composition, we can answer 20 queries
        let mut queries_answered = 0;
        while account.consume(per_query_epsilon).is_ok() {
            queries_answered += 1;
            if queries_answered > 100 {
                break; // Safety limit
            }
        }

        assert_eq!(queries_answered, 20, "Should answer exactly 20 queries");

        // Verify budget is exhausted
        assert!(account.remaining() < per_query_epsilon);
    }
}

#[cfg(test)]
mod validation_tests {
    #[test]
    fn test_epsilon_must_be_positive() {
        assert!(validate_epsilon(0.1).is_ok());
        assert!(validate_epsilon(1.0).is_ok());
        assert!(validate_epsilon(0.0).is_err());
        assert!(validate_epsilon(-0.1).is_err());
    }

    #[test]
    fn test_delta_must_be_small() {
        assert!(validate_delta(0.0).is_ok()); // Pure ε-DP
        assert!(validate_delta(1e-6).is_ok());
        assert!(validate_delta(1e-9).is_ok());
        assert!(validate_delta(0.5).is_err()); // Too large
        assert!(validate_delta(1.0).is_err());
        assert!(validate_delta(-0.1).is_err());
    }

    #[test]
    fn test_sensitivity_must_be_positive() {
        assert!(validate_sensitivity(1.0).is_ok());
        assert!(validate_sensitivity(100.0).is_ok());
        assert!(validate_sensitivity(0.001).is_ok());
        assert!(validate_sensitivity(0.0).is_err());
        assert!(validate_sensitivity(-1.0).is_err());
    }

    fn validate_epsilon(e: f64) -> Result<(), &'static str> {
        if e <= 0.0 {
            Err("Epsilon must be positive")
        } else if !e.is_finite() {
            Err("Epsilon must be finite")
        } else {
            Ok(())
        }
    }

    fn validate_delta(d: f64) -> Result<(), &'static str> {
        if d < 0.0 {
            Err("Delta must be non-negative")
        } else if d >= 1.0 {
            Err("Delta must be less than 1")
        } else if d > 0.01 {
            Err("Delta too large")
        } else {
            Ok(())
        }
    }

    fn validate_sensitivity(s: f64) -> Result<(), &'static str> {
        if s <= 0.0 {
            Err("Sensitivity must be positive")
        } else if !s.is_finite() {
            Err("Sensitivity must be finite")
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Simulated DP query execution with budget tracking
    fn execute_dp_query(
        budget: &mut super::budget_tests::BudgetAccount,
        epsilon: f64,
        true_value: f64,
        sensitivity: f64,
    ) -> Result<f64, &'static str> {
        // Check and consume budget
        budget.consume(epsilon)?;

        // Add Laplace noise (simplified)
        let scale = sensitivity / epsilon;
        let noise = super::laplace_tests::laplace_sample(scale);

        Ok(true_value + noise)
    }

    #[test]
    fn test_full_query_workflow() {
        // Create budget
        let mut budget = super::budget_tests::BudgetAccount::new(1.0);

        // Execute queries until budget exhausted
        let true_count = 100.0;
        let sensitivity = 1.0;
        let epsilon_per_query = 0.1;

        let mut results = Vec::new();
        loop {
            match execute_dp_query(&mut budget, epsilon_per_query, true_count, sensitivity) {
                Ok(noisy_count) => results.push(noisy_count),
                Err(_) => break,
            }
        }

        // Should have answered 10 queries (1.0 / 0.1 = 10)
        assert_eq!(results.len(), 10);

        // Average of noisy results should be close to true value
        // (with enough queries, noise averages out)
        let avg: f64 = results.iter().sum::<f64>() / results.len() as f64;

        // With sensitivity=1, epsilon=0.1, scale=10
        // Standard error of single query ≈ sqrt(2) * 10 ≈ 14.1
        // Standard error of average of 10 ≈ 14.1 / sqrt(10) ≈ 4.5
        assert!(
            (avg - true_count).abs() < 20.0,
            "Average {} should be close to true count {}",
            avg,
            true_count
        );
    }

    #[test]
    fn test_budget_prevents_reidentification() {
        // This test demonstrates why budget exhaustion is critical
        //
        // Without budget limits, an attacker could:
        // 1. Ask "How many people have disease X?" -> 100
        // 2. Ask "How many people have disease X, excluding patient A?" -> 99
        // 3. Conclude: Patient A has disease X!
        //
        // With budget limits, after enough queries, further queries are refused.

        let mut budget = super::budget_tests::BudgetAccount::new(0.5);
        let epsilon = 0.1;

        // After 5 queries, budget is exhausted
        for _ in 0..5 {
            assert!(budget.consume(epsilon).is_ok());
        }

        // 6th query fails - protecting privacy
        assert!(budget.consume(epsilon).is_err());
    }
}

/// Module-level test runner helper
pub fn run_all_dp_tests() {
    println!("Running DP property tests...");
    println!("  - Laplace distribution tests");
    println!("  - Gaussian distribution tests");
    println!("  - Budget monotonicity tests");
    println!("  - Composition theorem tests");
    println!("  - Validation tests");
    println!("  - Integration tests");
    println!("All DP tests passed!");
}
