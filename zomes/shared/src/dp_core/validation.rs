//! Input Validation for Differential Privacy Parameters
//!
//! Provides rigorous validation of DP parameters to ensure
//! meaningful privacy guarantees.
//!
//! # Parameter Constraints
//!
//! ## Epsilon (ε)
//! - Must be positive (> 0)
//! - Smaller = more private, but more noise
//! - Typical values: 0.01 (very private) to 1.0 (less private)
//! - Values > 10 provide minimal privacy protection
//!
//! ## Delta (δ)
//! - Must be in [0, 1)
//! - Should be cryptographically small (< 1/dataset_size)
//! - Typical values: 10^-6 to 10^-9
//! - δ = 0 gives pure ε-DP (Laplace mechanism)
//!
//! ## Sensitivity (Δf)
//! - Must be positive (> 0)
//! - Depends on the query type:
//!   - Count query: Δf = 1
//!   - Sum query: Δf = max_value
//!   - Average query: Δf = range / n
//!   - Histogram: Δf = 1 per bucket

use serde::{Deserialize, Serialize};

/// Error type for DP parameter validation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DpValidationError {
    /// Epsilon is invalid
    InvalidEpsilon { value: f64, reason: String },
    /// Delta is invalid
    InvalidDelta { value: f64, reason: String },
    /// Sensitivity is invalid
    InvalidSensitivity { value: f64, reason: String },
    /// Query parameters are invalid
    InvalidQuery(String),
}

impl std::fmt::Display for DpValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DpValidationError::InvalidEpsilon { value, reason } => {
                write!(f, "Invalid epsilon {}: {}", value, reason)
            }
            DpValidationError::InvalidDelta { value, reason } => {
                write!(f, "Invalid delta {}: {}", value, reason)
            }
            DpValidationError::InvalidSensitivity { value, reason } => {
                write!(f, "Invalid sensitivity {}: {}", value, reason)
            }
            DpValidationError::InvalidQuery(msg) => write!(f, "Invalid query: {}", msg),
        }
    }
}

/// Maximum allowed epsilon (beyond this, privacy is negligible)
pub const MAX_EPSILON: f64 = 10.0;

/// Maximum allowed delta (should be cryptographically small)
pub const MAX_DELTA: f64 = 0.01;

/// Minimum allowed epsilon (too small = too much noise)
pub const MIN_EPSILON: f64 = 1e-10;

/// Minimum allowed sensitivity
pub const MIN_SENSITIVITY: f64 = 1e-15;

/// Validate epsilon parameter
///
/// # Arguments
/// * `epsilon` - Privacy parameter to validate
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(DpValidationError)` if invalid
///
/// # Constraints
/// - Must be positive (> 0)
/// - Must be finite
/// - Should be ≤ MAX_EPSILON for meaningful privacy
pub fn validate_epsilon(epsilon: f64) -> Result<(), DpValidationError> {
    if !epsilon.is_finite() {
        return Err(DpValidationError::InvalidEpsilon {
            value: epsilon,
            reason: "Epsilon must be a finite number".to_string(),
        });
    }

    if epsilon <= 0.0 {
        return Err(DpValidationError::InvalidEpsilon {
            value: epsilon,
            reason: "Epsilon must be positive".to_string(),
        });
    }

    if epsilon < MIN_EPSILON {
        return Err(DpValidationError::InvalidEpsilon {
            value: epsilon,
            reason: format!(
                "Epsilon too small (< {}): would add infinite noise",
                MIN_EPSILON
            ),
        });
    }

    if epsilon > MAX_EPSILON {
        // Warning but not error - some use cases may need larger epsilon
        // In practice, you might want to log this
    }

    Ok(())
}

/// Validate delta parameter
///
/// # Arguments
/// * `delta` - Failure probability to validate
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(DpValidationError)` if invalid
///
/// # Constraints
/// - Must be in [0, 1)
/// - Must be finite
/// - Should be < 1/n (where n is dataset size) for meaningful privacy
pub fn validate_delta(delta: f64) -> Result<(), DpValidationError> {
    if !delta.is_finite() {
        return Err(DpValidationError::InvalidDelta {
            value: delta,
            reason: "Delta must be a finite number".to_string(),
        });
    }

    if delta < 0.0 {
        return Err(DpValidationError::InvalidDelta {
            value: delta,
            reason: "Delta must be non-negative".to_string(),
        });
    }

    if delta >= 1.0 {
        return Err(DpValidationError::InvalidDelta {
            value: delta,
            reason: "Delta must be less than 1".to_string(),
        });
    }

    if delta > MAX_DELTA {
        return Err(DpValidationError::InvalidDelta {
            value: delta,
            reason: format!(
                "Delta too large (> {}): privacy guarantee too weak",
                MAX_DELTA
            ),
        });
    }

    Ok(())
}

/// Validate sensitivity parameter
///
/// # Arguments
/// * `sensitivity` - Query sensitivity to validate
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(DpValidationError)` if invalid
///
/// # Constraints
/// - Must be positive (> 0)
/// - Must be finite
pub fn validate_sensitivity(sensitivity: f64) -> Result<(), DpValidationError> {
    if !sensitivity.is_finite() {
        return Err(DpValidationError::InvalidSensitivity {
            value: sensitivity,
            reason: "Sensitivity must be a finite number".to_string(),
        });
    }

    if sensitivity <= 0.0 {
        return Err(DpValidationError::InvalidSensitivity {
            value: sensitivity,
            reason: "Sensitivity must be positive".to_string(),
        });
    }

    if sensitivity < MIN_SENSITIVITY {
        return Err(DpValidationError::InvalidSensitivity {
            value: sensitivity,
            reason: "Sensitivity too small: likely a computation error".to_string(),
        });
    }

    Ok(())
}

/// Validate all DP parameters together
///
/// # Arguments
/// * `epsilon` - Privacy parameter
/// * `delta` - Failure probability (0 for pure ε-DP)
/// * `sensitivity` - Query sensitivity
pub fn validate_dp_parameters(
    epsilon: f64,
    delta: f64,
    sensitivity: f64,
) -> Result<(), DpValidationError> {
    validate_epsilon(epsilon)?;
    validate_delta(delta)?;
    validate_sensitivity(sensitivity)?;
    Ok(())
}

/// Validate that dataset size is sufficient for the delta parameter
///
/// For meaningful (ε, δ)-DP, we need δ < 1/n
/// Otherwise, the δ term dominates and privacy is lost.
///
/// # Arguments
/// * `delta` - Privacy parameter
/// * `dataset_size` - Number of records
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(DpValidationError)` if delta is too large for dataset
pub fn validate_delta_for_dataset(
    delta: f64,
    dataset_size: usize,
) -> Result<(), DpValidationError> {
    if dataset_size == 0 {
        return Err(DpValidationError::InvalidQuery(
            "Dataset size must be positive".to_string(),
        ));
    }

    let max_delta = 1.0 / (dataset_size as f64);

    if delta > max_delta {
        return Err(DpValidationError::InvalidDelta {
            value: delta,
            reason: format!(
                "Delta {} too large for dataset size {}: should be < {} (1/n)",
                delta, dataset_size, max_delta
            ),
        });
    }

    Ok(())
}

/// Validate minimum contributor count for aggregate queries
///
/// # Arguments
/// * `contributor_count` - Number of contributors
/// * `minimum_required` - Minimum for k-anonymity
pub fn validate_minimum_contributors(
    contributor_count: u32,
    minimum_required: u32,
) -> Result<(), DpValidationError> {
    if contributor_count < minimum_required {
        return Err(DpValidationError::InvalidQuery(format!(
            "Insufficient contributors: have {}, need {} for k-anonymity",
            contributor_count, minimum_required
        )));
    }
    Ok(())
}

/// Recommended privacy parameters for common use cases
#[derive(Debug, Clone, Copy)]
pub struct RecommendedParameters {
    pub epsilon: f64,
    pub delta: f64,
    pub description: &'static str,
}

/// Get recommended parameters for different privacy levels
pub fn recommended_parameters(level: PrivacyLevel) -> RecommendedParameters {
    match level {
        PrivacyLevel::VeryHigh => RecommendedParameters {
            epsilon: 0.1,
            delta: 1e-9,
            description: "Very high privacy: suitable for sensitive medical data",
        },
        PrivacyLevel::High => RecommendedParameters {
            epsilon: 0.5,
            delta: 1e-7,
            description: "High privacy: suitable for most health analytics",
        },
        PrivacyLevel::Medium => RecommendedParameters {
            epsilon: 1.0,
            delta: 1e-6,
            description: "Medium privacy: balance of utility and privacy",
        },
        PrivacyLevel::Low => RecommendedParameters {
            epsilon: 3.0,
            delta: 1e-5,
            description: "Lower privacy: higher utility, less noise",
        },
    }
}

/// Privacy levels for parameter recommendations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PrivacyLevel {
    VeryHigh,
    High,
    Medium,
    Low,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_epsilon_valid() {
        assert!(validate_epsilon(0.1).is_ok());
        assert!(validate_epsilon(1.0).is_ok());
        assert!(validate_epsilon(5.0).is_ok());
    }

    #[test]
    fn test_validate_epsilon_invalid() {
        assert!(validate_epsilon(0.0).is_err());
        assert!(validate_epsilon(-1.0).is_err());
        assert!(validate_epsilon(f64::INFINITY).is_err());
        assert!(validate_epsilon(f64::NAN).is_err());
    }

    #[test]
    fn test_validate_delta_valid() {
        assert!(validate_delta(0.0).is_ok()); // Pure ε-DP
        assert!(validate_delta(1e-6).is_ok());
        assert!(validate_delta(1e-9).is_ok());
    }

    #[test]
    fn test_validate_delta_invalid() {
        assert!(validate_delta(-0.001).is_err());
        assert!(validate_delta(1.0).is_err());
        assert!(validate_delta(0.5).is_err()); // Too large
        assert!(validate_delta(f64::NAN).is_err());
    }

    #[test]
    fn test_validate_sensitivity_valid() {
        assert!(validate_sensitivity(1.0).is_ok());
        assert!(validate_sensitivity(0.001).is_ok());
        assert!(validate_sensitivity(100.0).is_ok());
    }

    #[test]
    fn test_validate_sensitivity_invalid() {
        assert!(validate_sensitivity(0.0).is_err());
        assert!(validate_sensitivity(-1.0).is_err());
        assert!(validate_sensitivity(f64::NAN).is_err());
    }

    #[test]
    fn test_validate_dp_parameters_all_valid() {
        assert!(validate_dp_parameters(0.1, 1e-6, 1.0).is_ok());
    }

    #[test]
    fn test_validate_dp_parameters_any_invalid() {
        assert!(validate_dp_parameters(-0.1, 1e-6, 1.0).is_err());
        assert!(validate_dp_parameters(0.1, 0.5, 1.0).is_err());
        assert!(validate_dp_parameters(0.1, 1e-6, -1.0).is_err());
    }

    #[test]
    fn test_validate_delta_for_dataset() {
        // For n=1000, delta should be < 0.001
        assert!(validate_delta_for_dataset(1e-6, 1000).is_ok());
        assert!(validate_delta_for_dataset(0.01, 1000).is_err());
    }

    #[test]
    fn test_validate_minimum_contributors() {
        assert!(validate_minimum_contributors(100, 10).is_ok());
        assert!(validate_minimum_contributors(5, 10).is_err());
    }

    #[test]
    fn test_recommended_parameters() {
        let params = recommended_parameters(PrivacyLevel::High);
        assert!(validate_epsilon(params.epsilon).is_ok());
        assert!(validate_delta(params.delta).is_ok());
    }
}
