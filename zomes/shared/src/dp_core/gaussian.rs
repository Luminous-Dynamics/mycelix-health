//! Gaussian Mechanism for Differential Privacy
//!
//! Implements the Gaussian mechanism for achieving (ε, δ)-differential privacy.
//!
//! # Mathematical Foundation
//!
//! For a numeric query f with L2 sensitivity Δ₂f, the Gaussian mechanism adds
//! noise drawn from a normal distribution:
//!
//! ```text
//! M(D) = f(D) + N(0, σ²)
//! ```
//!
//! Where the standard deviation σ is calibrated to achieve (ε, δ)-DP:
//!
//! ```text
//! σ = Δ₂f · √(2 ln(1.25/δ)) / ε
//! ```
//!
//! # Privacy Guarantee
//!
//! The Gaussian mechanism provides (ε, δ)-differential privacy, meaning
//! for any two neighboring datasets D and D':
//!
//! ```text
//! P[M(D) ∈ S] ≤ e^ε · P[M(D') ∈ S] + δ
//! ```
//!
//! The δ parameter represents a small probability of "catastrophic" privacy failure.
//! It should typically be cryptographically small (e.g., 10^-6 or smaller).
//!
//! # Box-Muller Transform
//!
//! We generate Gaussian samples using the Box-Muller transform:
//!
//! ```text
//! Given U₁, U₂ ~ Uniform(0, 1):
//! Z₁ = √(-2 ln U₁) · cos(2π U₂)
//! Z₂ = √(-2 ln U₁) · sin(2π U₂)
//! ```
//!
//! Both Z₁ and Z₂ are independent standard normal N(0, 1) samples.

use super::rng::{RngError, SecureRng};
use super::validation::{
    validate_delta, validate_epsilon, validate_sensitivity, DpValidationError,
};
use serde::{Deserialize, Serialize};

/// Error type for Gaussian mechanism operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GaussianError {
    /// RNG failure
    Rng(String),
    /// Invalid parameters
    Validation(String),
}

impl std::fmt::Display for GaussianError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GaussianError::Rng(msg) => write!(f, "RNG error: {}", msg),
            GaussianError::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl From<RngError> for GaussianError {
    fn from(e: RngError) -> Self {
        GaussianError::Rng(e.to_string())
    }
}

impl From<DpValidationError> for GaussianError {
    fn from(e: DpValidationError) -> Self {
        GaussianError::Validation(e.to_string())
    }
}

/// Gaussian mechanism for (ε, δ)-differential privacy
pub struct GaussianMechanism;

impl GaussianMechanism {
    /// Sample from standard normal N(0, 1) using Box-Muller transform
    ///
    /// # Returns
    /// A sample from the standard normal distribution
    pub fn sample_standard_normal() -> Result<f64, GaussianError> {
        let (u1, u2) = SecureRng::random_pair_for_box_muller()?;

        // Box-Muller transform
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();

        Ok(z)
    }

    /// Sample from N(0, σ²)
    ///
    /// # Arguments
    /// * `sigma` - Standard deviation of the Gaussian
    ///
    /// # Returns
    /// A sample from N(0, σ²)
    pub fn sample(sigma: f64) -> Result<f64, GaussianError> {
        if sigma <= 0.0 {
            return Err(GaussianError::Validation(
                "Sigma must be positive".to_string(),
            ));
        }

        let z = Self::sample_standard_normal()?;
        Ok(z * sigma)
    }

    /// Compute the required σ for (ε, δ)-differential privacy
    ///
    /// Uses the analytic Gaussian mechanism formula:
    /// σ = Δ₂f · √(2 ln(1.25/δ)) / ε
    ///
    /// # Arguments
    /// * `sensitivity` - L2 sensitivity of the query
    /// * `epsilon` - Privacy parameter
    /// * `delta` - Probability of privacy failure
    ///
    /// # Returns
    /// The required standard deviation σ
    pub fn compute_sigma(sensitivity: f64, epsilon: f64, delta: f64) -> Result<f64, GaussianError> {
        validate_sensitivity(sensitivity)?;
        validate_epsilon(epsilon)?;
        validate_delta(delta)?;

        // σ = Δf · √(2 ln(1.25/δ)) / ε
        let sigma = sensitivity * (2.0 * (1.25 / delta).ln()).sqrt() / epsilon;

        Ok(sigma)
    }

    /// Add Gaussian noise to a value for (ε, δ)-differential privacy
    ///
    /// # Arguments
    /// * `value` - The true value to protect
    /// * `sensitivity` - The L2 sensitivity of the query
    /// * `epsilon` - Privacy parameter (lower = more private)
    /// * `delta` - Probability of privacy failure (should be very small)
    ///
    /// # Returns
    /// The noisy value: value + N(0, σ²)
    ///
    /// # Example
    /// ```ignore
    /// // Average query has sensitivity = range / n
    /// let true_avg = 50.0;
    /// let sensitivity = 100.0 / 1000.0; // range 100, 1000 records
    /// let noisy_avg = GaussianMechanism::add_noise(true_avg, sensitivity, 0.1, 1e-6)?;
    /// // noisy_avg is (ε, δ)-differentially private
    /// ```
    pub fn add_noise(
        value: f64,
        sensitivity: f64,
        epsilon: f64,
        delta: f64,
    ) -> Result<f64, GaussianError> {
        let sigma = Self::compute_sigma(sensitivity, epsilon, delta)?;
        let noise = Self::sample(sigma)?;

        Ok(value + noise)
    }

    /// Compute the variance of Gaussian noise for given parameters
    ///
    /// Var = σ² = (Δf · √(2 ln(1.25/δ)) / ε)²
    pub fn variance(sensitivity: f64, epsilon: f64, delta: f64) -> Result<f64, GaussianError> {
        let sigma = Self::compute_sigma(sensitivity, epsilon, delta)?;
        Ok(sigma * sigma)
    }

    /// Compute the 95% confidence interval half-width for Gaussian noise
    ///
    /// For N(0, σ²), 95% of values fall within [-1.96σ, 1.96σ]
    pub fn confidence_interval_95(
        sensitivity: f64,
        epsilon: f64,
        delta: f64,
    ) -> Result<f64, GaussianError> {
        let sigma = Self::compute_sigma(sensitivity, epsilon, delta)?;
        Ok(1.96 * sigma)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_standard_normal_produces_values() {
        let sample = GaussianMechanism::sample_standard_normal().unwrap();
        assert!(sample.is_finite());
    }

    #[test]
    fn test_sample_with_sigma() {
        let sigma = 2.0;
        let sample = GaussianMechanism::sample(sigma).unwrap();
        assert!(sample.is_finite());
    }

    #[test]
    fn test_sample_invalid_sigma() {
        assert!(GaussianMechanism::sample(0.0).is_err());
        assert!(GaussianMechanism::sample(-1.0).is_err());
    }

    #[test]
    fn test_compute_sigma() {
        let sensitivity = 1.0;
        let epsilon = 0.1;
        let delta = 1e-6;

        let sigma = GaussianMechanism::compute_sigma(sensitivity, epsilon, delta).unwrap();

        // σ = 1.0 * √(2 ln(1.25/1e-6)) / 0.1
        // ln(1.25e6) ≈ 14.04
        // √(2 * 14.04) ≈ 5.30
        // σ ≈ 53.0
        assert!(
            sigma > 50.0 && sigma < 60.0,
            "Sigma {} out of expected range",
            sigma
        );
    }

    #[test]
    fn test_add_noise_changes_value() {
        let value = 100.0;
        let noisy = GaussianMechanism::add_noise(value, 1.0, 0.1, 1e-6).unwrap();
        assert!(noisy.is_finite());
    }

    #[test]
    fn test_standard_normal_mean_approximately_zero() {
        let n = 10000;
        let sum: f64 = (0..n)
            .map(|_| GaussianMechanism::sample_standard_normal().unwrap())
            .sum();
        let mean = sum / n as f64;

        // Standard error = 1/√n
        let se = 1.0 / (n as f64).sqrt();
        assert!(mean.abs() < 3.0 * se, "Mean {} too far from 0", mean);
    }

    #[test]
    fn test_standard_normal_variance_approximately_one() {
        let n = 10000;
        let samples: Vec<f64> = (0..n)
            .map(|_| GaussianMechanism::sample_standard_normal().unwrap())
            .collect();

        let mean: f64 = samples.iter().sum::<f64>() / n as f64;
        let variance: f64 =
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64;

        // Expected variance = 1.0
        assert!(
            (variance - 1.0).abs() < 0.1,
            "Variance {} too far from 1.0",
            variance
        );
    }

    #[test]
    fn test_scaled_normal_variance() {
        let sigma = 3.0;
        let n = 10000;
        let samples: Vec<f64> = (0..n)
            .map(|_| GaussianMechanism::sample(sigma).unwrap())
            .collect();

        let mean: f64 = samples.iter().sum::<f64>() / n as f64;
        let variance: f64 =
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64;

        // Expected variance = σ² = 9.0
        assert!(
            (variance - 9.0).abs() / 9.0 < 0.15,
            "Variance {} too far from expected 9.0",
            variance
        );
    }
}
