// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Pure-Rust input validation for Mycelix Health.
//!
//! Extracted from `mycelix-health-shared` to enable testing without HDK.
//! These validators are used across all health zomes for data quality
//! and security at the system boundary.
//!
//! # Validators
//! - [`validate_mrn`] — Medical Record Number format
//! - [`validate_did`] — Decentralized Identifier format
//! - [`validate_confidence_score`] — [0,1] bounds with NaN rejection
//! - [`validate_score_range`] — Integer score within instrument range

use serde::{Deserialize, Serialize};

// ============================================================================
// Error types
// ============================================================================

/// Specific validation error codes for programmatic handling.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationErrorCode {
    Required,
    InvalidFormat,
    OutOfRange,
    TooLong,
    TooShort,
    InvalidCharacters,
    DuplicateValue,
    InvalidReference,
}

/// Validation error with field context.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: ValidationErrorCode,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} ({:?})", self.field, self.message, self.code)
    }
}

/// Accumulating validation result.
#[derive(Clone, Debug, Default)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn add_error(&mut self, field: &str, message: &str, code: ValidationErrorCode) {
        self.errors.push(ValidationError {
            field: field.to_string(),
            message: message.to_string(),
            code,
        });
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
    }

    pub fn error_codes(&self) -> Vec<&ValidationErrorCode> {
        self.errors.iter().map(|e| &e.code).collect()
    }
}

// ============================================================================
// Validators
// ============================================================================

/// Validate a Medical Record Number (MRN).
///
/// MRN must be:
/// - 4-20 characters long
/// - Alphanumeric with optional hyphens
/// - Not empty
pub fn validate_mrn(mrn: &str) -> ValidationResult {
    let mut result = ValidationResult::new();

    if mrn.is_empty() {
        result.add_error("mrn", "MRN is required", ValidationErrorCode::Required);
        return result;
    }

    if mrn.len() < 4 {
        result.add_error(
            "mrn",
            "MRN must be at least 4 characters",
            ValidationErrorCode::TooShort,
        );
    }

    if mrn.len() > 20 {
        result.add_error(
            "mrn",
            "MRN cannot exceed 20 characters",
            ValidationErrorCode::TooLong,
        );
    }

    if !mrn.chars().all(|c| c.is_alphanumeric() || c == '-') {
        result.add_error(
            "mrn",
            "MRN can only contain letters, numbers, and hyphens",
            ValidationErrorCode::InvalidCharacters,
        );
    }

    result
}

/// Validate a Decentralized Identifier (DID).
///
/// DID must follow the format: `did:method:specific-id`
/// Supported methods: key, web, pkh, holo, ethr, ion
pub fn validate_did(did: &str) -> ValidationResult {
    let mut result = ValidationResult::new();

    if did.is_empty() {
        result.add_error("did", "DID is required", ValidationErrorCode::Required);
        return result;
    }

    if !did.starts_with("did:") {
        result.add_error(
            "did",
            "DID must start with 'did:'",
            ValidationErrorCode::InvalidFormat,
        );
        return result;
    }

    let parts: Vec<&str> = did.splitn(3, ':').collect();
    if parts.len() < 3 {
        result.add_error(
            "did",
            "DID must have format 'did:method:specific-id'",
            ValidationErrorCode::InvalidFormat,
        );
        return result;
    }

    let method = parts[1];
    let valid_methods = ["key", "web", "pkh", "holo", "ethr", "ion"];
    if !valid_methods.contains(&method) {
        result.add_error(
            "did",
            &format!(
                "Unsupported DID method '{}'. Supported: {:?}",
                method, valid_methods
            ),
            ValidationErrorCode::InvalidFormat,
        );
    }

    let specific_id = parts[2];
    if specific_id.is_empty() {
        result.add_error(
            "did",
            "DID specific identifier is required",
            ValidationErrorCode::Required,
        );
    }

    if specific_id.len() > 256 {
        result.add_error(
            "did",
            "DID specific identifier too long",
            ValidationErrorCode::TooLong,
        );
    }

    result
}

/// Validate a confidence score (0.0 - 1.0).
pub fn validate_confidence_score(score: f64, field_name: &str) -> ValidationResult {
    let mut result = ValidationResult::new();

    if score.is_nan() {
        result.add_error(
            field_name,
            "Confidence score cannot be NaN",
            ValidationErrorCode::InvalidFormat,
        );
        return result;
    }

    if score < 0.0 || score > 1.0 {
        result.add_error(
            field_name,
            "Confidence score must be between 0.0 and 1.0",
            ValidationErrorCode::OutOfRange,
        );
    }

    result
}

/// Validate a score within a specified range.
pub fn validate_score_range(
    score: u32,
    min: u32,
    max: u32,
    field_name: &str,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    if score < min || score > max {
        result.add_error(
            field_name,
            &format!("Score must be between {} and {}, got {}", min, max, score),
            ValidationErrorCode::OutOfRange,
        );
    }

    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── MRN ──────────────────────────────────────────────────────────

    #[test]
    fn test_mrn_valid() {
        assert!(validate_mrn("MRN-1234").is_valid());
        assert!(validate_mrn("ABCD").is_valid());
        assert!(validate_mrn("12345678901234567890").is_valid()); // 20 chars
    }

    #[test]
    fn test_mrn_empty() {
        let r = validate_mrn("");
        assert!(!r.is_valid());
        assert!(r.error_codes().contains(&&ValidationErrorCode::Required));
    }

    #[test]
    fn test_mrn_too_short() {
        let r = validate_mrn("AB");
        assert!(!r.is_valid());
        assert!(r.error_codes().contains(&&ValidationErrorCode::TooShort));
    }

    #[test]
    fn test_mrn_too_long() {
        let r = validate_mrn("A".repeat(21).as_str());
        assert!(!r.is_valid());
        assert!(r.error_codes().contains(&&ValidationErrorCode::TooLong));
    }

    #[test]
    fn test_mrn_invalid_chars() {
        let r = validate_mrn("MRN@#$!");
        assert!(!r.is_valid());
        assert!(r
            .error_codes()
            .contains(&&ValidationErrorCode::InvalidCharacters));
    }

    // ── DID ──────────────────────────────────────────────────────────

    #[test]
    fn test_did_valid() {
        assert!(validate_did("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK").is_valid());
        assert!(validate_did("did:web:example.com").is_valid());
        assert!(validate_did("did:holo:uhCAkX5678").is_valid());
        assert!(validate_did("did:ethr:0x1234abcd").is_valid());
        assert!(validate_did("did:ion:long-identifier").is_valid());
    }

    #[test]
    fn test_did_empty() {
        let r = validate_did("");
        assert!(!r.is_valid());
        assert!(r.error_codes().contains(&&ValidationErrorCode::Required));
    }

    #[test]
    fn test_did_missing_prefix() {
        let r = validate_did("key:z6MkhaXgBZDvotDkL5257");
        assert!(!r.is_valid());
        assert!(r
            .error_codes()
            .contains(&&ValidationErrorCode::InvalidFormat));
    }

    #[test]
    fn test_did_unsupported_method() {
        let r = validate_did("did:unsupported:identifier");
        assert!(!r.is_valid());
        assert!(r
            .error_codes()
            .contains(&&ValidationErrorCode::InvalidFormat));
    }

    #[test]
    fn test_did_missing_specific_id() {
        let r = validate_did("did:key:");
        assert!(!r.is_valid());
        assert!(r.error_codes().contains(&&ValidationErrorCode::Required));
    }

    #[test]
    fn test_did_too_long_specific_id() {
        let long_id = "x".repeat(257);
        let r = validate_did(&format!("did:key:{}", long_id));
        assert!(!r.is_valid());
        assert!(r.error_codes().contains(&&ValidationErrorCode::TooLong));
    }

    // ── Confidence Score ─────────────────────────────────────────────

    #[test]
    fn test_confidence_valid() {
        assert!(validate_confidence_score(0.0, "score").is_valid());
        assert!(validate_confidence_score(0.5, "score").is_valid());
        assert!(validate_confidence_score(1.0, "score").is_valid());
    }

    #[test]
    fn test_confidence_out_of_range() {
        assert!(!validate_confidence_score(-0.1, "score").is_valid());
        assert!(!validate_confidence_score(1.1, "score").is_valid());
    }

    #[test]
    fn test_confidence_nan() {
        let r = validate_confidence_score(f64::NAN, "score");
        assert!(!r.is_valid());
        assert!(r
            .error_codes()
            .contains(&&ValidationErrorCode::InvalidFormat));
    }

    #[test]
    fn test_confidence_infinity() {
        assert!(!validate_confidence_score(f64::INFINITY, "score").is_valid());
        assert!(!validate_confidence_score(f64::NEG_INFINITY, "score").is_valid());
    }

    // ── Score Range ──────────────────────────────────────────────────

    #[test]
    fn test_score_range_valid() {
        assert!(validate_score_range(5, 0, 27, "phq9").is_valid());
        assert!(validate_score_range(0, 0, 27, "phq9").is_valid());
        assert!(validate_score_range(27, 0, 27, "phq9").is_valid());
    }

    #[test]
    fn test_score_range_out_of_bounds() {
        assert!(!validate_score_range(28, 0, 27, "phq9").is_valid());
    }

    // ── Merge ────────────────────────────────────────────────────────

    #[test]
    fn test_validation_result_merge() {
        let mut r1 = validate_mrn(""); // Has error
        let r2 = validate_did("");     // Has error
        r1.merge(r2);
        assert_eq!(r1.errors.len(), 2);
    }

    // ── Proptest ─────────────────────────────────────────────────────

    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(1000))]

            #[test]
            fn mrn_valid_alphanum_always_passes(
                mrn in "[a-zA-Z0-9\\-]{4,20}"
            ) {
                prop_assert!(validate_mrn(&mrn).is_valid(),
                    "Valid MRN should pass: {mrn}");
            }

            #[test]
            fn did_valid_format_always_passes(
                method in prop::sample::select(vec!["key", "web", "pkh", "holo", "ethr", "ion"]),
                id in "[a-zA-Z0-9]{1,50}"
            ) {
                let did = format!("did:{method}:{id}");
                prop_assert!(validate_did(&did).is_valid(),
                    "Valid DID should pass: {did}");
            }

            #[test]
            fn confidence_in_range_passes(score in 0.0..=1.0f64) {
                prop_assert!(validate_confidence_score(score, "test").is_valid(),
                    "Score {score} should be valid");
            }

            #[test]
            fn confidence_out_of_range_fails(score in prop::num::f64::ANY) {
                prop_assume!(!score.is_nan());
                if score < 0.0 || score > 1.0 {
                    prop_assert!(!validate_confidence_score(score, "test").is_valid(),
                        "Score {score} should be invalid");
                }
            }

            #[test]
            fn score_range_within_bounds_passes(
                score in 0u32..=100,
                min in 0u32..=50,
                max in 51u32..=100,
            ) {
                if score >= min && score <= max {
                    prop_assert!(validate_score_range(score, min, max, "test").is_valid());
                }
            }
        }
    }
}
