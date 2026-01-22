//! Provider Zome Tests
//!
//! Tests for provider credentials, license verification, and epistemic classification.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestProvider {
    pub provider_id: String,
    pub npi_number: String,
    pub name: String,
    pub credentials: Vec<String>,
    pub provider_type: String,
    pub epistemic_level: u8,
    pub specialties: Vec<String>,
    pub licenses: Vec<TestLicense>,
    pub matl_trust_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestLicense {
    pub license_type: String,
    pub license_number: String,
    pub issuing_state: String,
    pub issued_date: i64,
    pub expiration_date: i64,
    pub status: String,
    pub verified_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_provider() -> TestProvider {
        TestProvider {
            provider_id: "PROV-001".to_string(),
            npi_number: "1234567890".to_string(),
            name: "Dr. Sarah Chen, MD, PhD".to_string(),
            credentials: vec!["MD".to_string(), "PhD".to_string()],
            provider_type: "Physician".to_string(),
            epistemic_level: 2,
            specialties: vec!["Internal Medicine".to_string(), "Oncology".to_string()],
            licenses: vec![
                TestLicense {
                    license_type: "MD".to_string(),
                    license_number: "MD-12345".to_string(),
                    issuing_state: "CA".to_string(),
                    issued_date: 1420070400000000,
                    expiration_date: 1767225600000000,
                    status: "Active".to_string(),
                    verified_at: Some(1704067200000000),
                },
            ],
            matl_trust_score: 0.85,
        }
    }

    #[test]
    fn test_npi_format() {
        let provider = create_test_provider();
        // NPI must be exactly 10 digits
        assert_eq!(provider.npi_number.len(), 10);
        assert!(provider.npi_number.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_provider_type_valid() {
        let provider = create_test_provider();
        let valid_types = [
            "Physician", "Nurse", "Pharmacist", "Dentist",
            "Psychologist", "PhysicalTherapist", "SocialWorker",
            "PhysicianAssistant", "NursePractitioner", "Specialist"
        ];
        assert!(valid_types.contains(&provider.provider_type.as_str()));
    }

    #[test]
    fn test_provider_has_credentials() {
        let provider = create_test_provider();
        assert!(!provider.credentials.is_empty());
    }

    #[test]
    fn test_provider_has_at_least_one_license() {
        let provider = create_test_provider();
        assert!(!provider.licenses.is_empty());
    }

    #[test]
    fn test_license_status_valid() {
        let provider = create_test_provider();
        let valid_statuses = ["Active", "Inactive", "Suspended", "Revoked", "Expired", "Pending"];
        for license in &provider.licenses {
            assert!(valid_statuses.contains(&license.status.as_str()));
        }
    }

    #[test]
    fn test_license_dates_valid() {
        let provider = create_test_provider();
        for license in &provider.licenses {
            assert!(license.expiration_date > license.issued_date);
        }
    }

    #[test]
    fn test_epistemic_level_valid() {
        let provider = create_test_provider();
        // E0-E4 scale
        assert!(provider.epistemic_level <= 4);
    }

    #[test]
    fn test_trust_score_bounds() {
        let provider = create_test_provider();
        assert!(provider.matl_trust_score >= 0.0 && provider.matl_trust_score <= 1.0);
    }

    #[test]
    fn test_verified_provider_has_verification_timestamp() {
        let provider = create_test_provider();
        for license in &provider.licenses {
            if license.status == "Active" {
                // Active licenses should have verification
                assert!(license.verified_at.is_some());
            }
        }
    }

    #[test]
    fn test_state_code_format() {
        let provider = create_test_provider();
        for license in &provider.licenses {
            // US state codes are 2 uppercase letters
            assert_eq!(license.issuing_state.len(), 2);
            assert!(license.issuing_state.chars().all(|c| c.is_ascii_uppercase()));
        }
    }
}
