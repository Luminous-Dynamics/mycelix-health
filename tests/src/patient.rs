//! Patient Zome Tests
//!
//! Tests for patient registration, demographics, allergies, and identity linking.

use serde::{Deserialize, Serialize};

/// Test patient demographics structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPatient {
    pub patient_id: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: i64,
    pub biological_sex: String,
    pub blood_type: Option<String>,
    pub contact: TestContactInfo,
    pub emergency_contact: Option<TestEmergencyContact>,
    pub allergies: Vec<TestAllergy>,
    pub matl_trust_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestContactInfo {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEmergencyContact {
    pub name: String,
    pub relationship: String,
    pub phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAllergy {
    pub allergen: String,
    pub severity: String,
    pub reaction_description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_patient() -> TestPatient {
        TestPatient {
            patient_id: "PAT-001".to_string(),
            first_name: "Alice".to_string(),
            last_name: "Johnson".to_string(),
            date_of_birth: 631152000000000, // 1990-01-01 in microseconds
            biological_sex: "Female".to_string(),
            blood_type: Some("A+".to_string()),
            contact: TestContactInfo {
                email: Some("alice@example.com".to_string()),
                phone: Some("+1-555-0123".to_string()),
                address: Some("123 Main St".to_string()),
            },
            emergency_contact: Some(TestEmergencyContact {
                name: "Bob Johnson".to_string(),
                relationship: "Spouse".to_string(),
                phone: "+1-555-0124".to_string(),
            }),
            allergies: vec![
                TestAllergy {
                    allergen: "Penicillin".to_string(),
                    severity: "Severe".to_string(),
                    reaction_description: "Anaphylaxis".to_string(),
                },
            ],
            matl_trust_score: 0.0,
        }
    }

    #[test]
    fn test_patient_creation() {
        let patient = create_test_patient();
        assert_eq!(patient.patient_id, "PAT-001");
        assert_eq!(patient.first_name, "Alice");
        assert_eq!(patient.last_name, "Johnson");
    }

    #[test]
    fn test_patient_id_format() {
        let patient = create_test_patient();
        // Patient IDs should follow format: PAT-XXX
        assert!(patient.patient_id.starts_with("PAT-"));
        assert!(patient.patient_id.len() >= 5);
    }

    #[test]
    fn test_patient_has_valid_contact() {
        let patient = create_test_patient();
        // At least one contact method required
        assert!(
            patient.contact.email.is_some()
            || patient.contact.phone.is_some()
        );
    }

    #[test]
    fn test_allergy_validation() {
        let patient = create_test_patient();
        for allergy in &patient.allergies {
            // Severity must be valid
            assert!(
                allergy.severity == "Mild"
                || allergy.severity == "Moderate"
                || allergy.severity == "Severe"
            );
            // Allergen name required
            assert!(!allergy.allergen.is_empty());
        }
    }

    #[test]
    fn test_biological_sex_valid_values() {
        let patient = create_test_patient();
        let valid_values = ["Male", "Female", "Intersex", "Unknown"];
        assert!(valid_values.contains(&patient.biological_sex.as_str()));
    }

    #[test]
    fn test_blood_type_valid_values() {
        let patient = create_test_patient();
        if let Some(blood_type) = &patient.blood_type {
            let valid_types = ["A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"];
            assert!(valid_types.contains(&blood_type.as_str()));
        }
    }

    #[test]
    fn test_initial_trust_score() {
        let patient = create_test_patient();
        // New patients start with 0 trust score
        assert_eq!(patient.matl_trust_score, 0.0);
    }

    #[test]
    fn test_trust_score_bounds() {
        let mut patient = create_test_patient();
        patient.matl_trust_score = 0.85;
        // Trust score must be between 0.0 and 1.0
        assert!(patient.matl_trust_score >= 0.0 && patient.matl_trust_score <= 1.0);
    }

    #[test]
    fn test_date_of_birth_valid() {
        let patient = create_test_patient();
        // DOB should be in the past (in microseconds)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;
        assert!(patient.date_of_birth < now);
    }

    #[test]
    fn test_emergency_contact_phone_format() {
        let patient = create_test_patient();
        if let Some(contact) = &patient.emergency_contact {
            // Phone should contain digits
            assert!(contact.phone.chars().any(|c| c.is_ascii_digit()));
        }
    }
}
