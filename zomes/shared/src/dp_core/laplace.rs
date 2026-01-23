//! Laplace Mechanism for Differential Privacy
//!
//! Implements the classic Laplace mechanism for achieving (ε, 0)-differential privacy.
//!
//! # Mathematical Foundation
//!
//! For a numeric query f with sensitivity Δf (the maximum change in f when
//! one record is added/removed), the Laplace mechanism adds noise drawn from
//! the Laplace distribution:
//!
//! ```text
//! M(D) = f(D) + Lap(0, Δf/ε)
//! ```
//!
//! The Laplace distribution with scale b = Δf/ε has PDF:
//!
//! ```text
//! p(x) = (1/2b) * e^(-|x|/b)
//! ```
//!
//! # Privacy Guarantee
//!
//! The Laplace mechanism provides (ε, 0)-differential privacy, meaning
//! for any two neighboring datasets D and D':
//!
//! ```text
//! P[M(D) ∈ S] ≤ e^ε · P[M(D') ∈ S]
//! ```
//!
//! # Inverse CDF Sampling
//!
//! We sample from Laplace(0, b) using the inverse CDF method:
//!
//! ```text
//! F(x) = 0.5 + 0.5 * sign(x) * (1 - e^(-|x|/b))
//! F^(-1)(u) = -b * sign(u - 0.5) * ln(1 - 2|u - 0.5|)
//! ```
//!
//! For u ~ Uniform(0, 1), F^(-1)(u) ~ Laplace(0, b)

use super::rng::{RngError, SecureRng};
use super::validation::{validate_epsilon, validate_sensitivity, DpValidationError};
use serde::{Deserialize, Serialize};

/// Error type for Laplace mechanism operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LaplaceError {
    /// RNG failure
    Rng(String),
    /// Invalid parameters
    Validation(String),
}

impl std::fmt::Display for LaplaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaplaceError::Rng(msg) => write!(f, "RNG error: {}", msg),
            LaplaceError::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl From<RngError> for LaplaceError {
    fn from(e: RngError) -> Self {
        LaplaceError::Rng(e.to_string())
    }
}

impl From<DpValidationError> for LaplaceError {
    fn from(e: DpValidationError) -> Self {
        LaplaceError::Validation(e.to_string())
    }
}

/// Laplace mechanism for (ε, 0)-differential privacy
pub struct LaplaceMechanism;

impl LaplaceMechanism {
    /// Sample from Laplace(0, scale) distribution
    ///
    /// Uses the inverse CDF method with cryptographic randomness.
    ///
    /// # Arguments
    /// * `scale` - The scale parameter b of the Laplace distribution
    ///
    /// # Returns
    /// A sample from Laplace(0, scale)
    ///
    /// # Mathematical Details
    ///
    /// For U ~ Uniform(-0.5, 0.5), we compute:
    /// X = -scale * sign(U) * ln(1 - 2|U|)
    ///
    /// This is equivalent to the standard inverse CDF with U' = U + 0.5 ~ Uniform(0, 1).
    pub fn sample(scale: f64) -> Result<f64, LaplaceError> {
        if scale <= 0.0 {
            return Err(LaplaceError::Validation(
                "Scale must be positive".to_string(),
            ));
        }

        let u = SecureRng::random_f64_centered()?;

        // Inverse CDF: -scale * sign(u) * ln(1 - 2|u|)
        let abs_u = u.abs();
        let noise = -scale * u.signum() * (1.0 - 2.0 * abs_u).ln();

        Ok(noise)
    }

    /// Add Laplace noise to a value for (ε, 0)-differential privacy
    ///
    /// # Arguments
    /// * `value` - The true value to protect
    /// * `sensitivity` - The L1 sensitivity (Δf) of the query
    /// * `epsilon` - Privacy parameter (lower = more private)
    ///
    /// # Returns
    /// The noisy value: value + Lap(0, sensitivity/epsilon)
    ///
    /// # Example
    /// ```ignore
    /// // Count query has sensitivity 1 (adding one record changes count by 1)
    /// let true_count = 100.0;
    /// let noisy_count = LaplaceMechanism::add_noise(true_count, 1.0, 0.1)?;
    /// // noisy_count is ε-differentially private with ε = 0.1
    /// ```
    pub fn add_noise(value: f64, sensitivity: f64, epsilon: f64) -> Result<f64, LaplaceError> {
        validate_sensitivity(sensitivity)?;
        validate_epsilon(epsilon)?;

        let scale = sensitivity / epsilon;
        let noise = Self::sample(scale)?;

        Ok(value + noise)
    }

    /// Compute the scale parameter for given sensitivity and epsilon
    ///
    /// scale = Δf / ε
    pub fn compute_scale(sensitivity: f64, epsilon: f64) -> Result<f64, LaplaceError> {
        validate_sensitivity(sensitivity)?;
        validate_epsilon(epsilon)?;
        Ok(sensitivity / epsilon)
    }

    /// Compute the variance of Laplace noise
    ///
    /// Var(Lap(0, b)) = 2b²
    ///
    /// For scale = Δf/ε:
    /// Var = 2(Δf/ε)² = 2Δf²/ε²
    pub fn variance(sensitivity: f64, epsilon: f64) -> Result<f64, LaplaceError> {
        let scale = Self::compute_scale(sensitivity, epsilon)?;
        Ok(2.0 * scale * scale)
    }

    /// Compute the standard deviation of Laplace noise
    ///
    /// SD = √(2) * scale = √(2) * Δf/ε
    pub fn std_dev(sensitivity: f64, epsilon: f64) -> Result<f64, LaplaceError> {
        let variance = Self::variance(sensitivity, epsilon)?;
        Ok(variance.sqrt())
    }

    /// Compute the 95% confidence interval width for Laplace noise
    ///
    /// For Laplace(0, b), 95% of values fall within [-b*ln(20), b*ln(20)]
    /// ≈ [-3b, 3b]
    ///
    /// Returns the half-width of the interval.
    pub fn confidence_interval_95(sensitivity: f64, epsilon: f64) -> Result<f64, LaplaceError> {
        let scale = Self::compute_scale(sensitivity, epsilon)?;
        // P(|X| < x) = 1 - e^(-x/b) for x > 0
        // 0.95 = 1 - e^(-x/b)
        // e^(-x/b) = 0.05
        // x = -b * ln(0.05) ≈ 3b
        Ok(-scale * 0.05_f64.ln())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_produces_values() {
        let scale = 1.0;
        let sample = LaplaceMechanism::sample(scale).unwrap();
        // Sample should be finite
        assert!(sample.is_finite());
    }

    #[test]
    fn test_sample_invalid_scale() {
        assert!(LaplaceMechanism::sample(0.0).is_err());
        assert!(LaplaceMechanism::sample(-1.0).is_err());
    }

    #[test]
    fn test_add_noise_changes_value() {
        let value = 100.0;
        let noisy = LaplaceMechanism::add_noise(value, 1.0, 0.1).unwrap();
        // Very unlikely to be exactly the same
        // (but theoretically possible, so we just check it's finite)
        assert!(noisy.is_finite());
    }

    #[test]
    fn test_variance_calculation() {
        let sensitivity = 1.0;
        let epsilon = 0.1;
        let variance = LaplaceMechanism::variance(sensitivity, epsilon).unwrap();
        // Var = 2 * (1/0.1)² = 2 * 100 = 200
        assert!((variance - 200.0).abs() < 1e-10);
    }

    #[test]
    fn test_sample_mean_approximately_zero() {
        // Law of large numbers: mean should be close to 0
        let scale = 1.0;
        let n = 10000;
        let sum: f64 = (0..n)
            .map(|_| LaplaceMechanism::sample(scale).unwrap())
            .sum();
        let mean = sum / n as f64;

        // Mean should be within 3 standard errors of 0
        // SE = sqrt(variance/n) = sqrt(2)/sqrt(n)
        let se = (2.0_f64).sqrt() / (n as f64).sqrt();
        assert!(mean.abs() < 3.0 * se, "Mean {} too far from 0", mean);
    }

    #[test]
    fn test_sample_variance_approximately_correct() {
        let scale = 2.0;
        let n = 10000;
        let samples: Vec<f64> = (0..n)
            .map(|_| LaplaceMechanism::sample(scale).unwrap())
            .collect();

        let mean: f64 = samples.iter().sum::<f64>() / n as f64;
        let variance: f64 =
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64;

        // Expected variance = 2 * scale² = 2 * 4 = 8
        let expected = 2.0 * scale * scale;

        // Should be within 20% (statistical test)
        assert!(
            (variance - expected).abs() / expected < 0.2,
            "Variance {} too far from expected {}",
            variance,
            expected
        );
    }
}
