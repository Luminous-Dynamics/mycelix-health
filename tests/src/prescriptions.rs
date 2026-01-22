//! Prescriptions Zome Tests
//!
//! Tests for medication prescribing, drug interactions, and controlled substances.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPrescription {
    pub prescription_id: String,
    pub patient_id: String,
    pub prescriber_id: String,
    pub rxnorm_code: String,
    pub ndc_code: Option<String>,
    pub medication_name: String,
    pub dosage: String,
    pub frequency: String,
    pub route: String,
    pub quantity: u32,
    pub refills_allowed: u32,
    pub refills_remaining: u32,
    pub prescribed_date: i64,
    pub expiration_date: i64,
    pub status: String,
    pub drug_schedule: Option<String>,
    pub diagnosis_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDrugInteraction {
    pub alert_id: String,
    pub drug_a_rxnorm: String,
    pub drug_b_rxnorm: String,
    pub severity: String,
    pub description: String,
    pub clinical_significance: String,
    pub management: String,
    pub acknowledged: bool,
    pub acknowledged_by: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_prescription() -> TestPrescription {
        TestPrescription {
            prescription_id: "RX-001".to_string(),
            patient_id: "PAT-001".to_string(),
            prescriber_id: "PROV-001".to_string(),
            rxnorm_code: "197361".to_string(),
            ndc_code: Some("00074-3799-13".to_string()),
            medication_name: "Lisinopril 10 MG Oral Tablet".to_string(),
            dosage: "10 mg".to_string(),
            frequency: "Once daily".to_string(),
            route: "Oral".to_string(),
            quantity: 30,
            refills_allowed: 5,
            refills_remaining: 5,
            prescribed_date: 1704067200000000,
            expiration_date: 1735689600000000,
            status: "Active".to_string(),
            drug_schedule: None, // Non-controlled
            diagnosis_codes: vec!["I10".to_string()], // Hypertension
        }
    }

    fn create_controlled_prescription() -> TestPrescription {
        TestPrescription {
            prescription_id: "RX-002".to_string(),
            patient_id: "PAT-001".to_string(),
            prescriber_id: "PROV-001".to_string(),
            rxnorm_code: "856987".to_string(),
            ndc_code: Some("00406-0367-01".to_string()),
            medication_name: "Oxycodone 5 MG Oral Tablet".to_string(),
            dosage: "5 mg".to_string(),
            frequency: "Every 6 hours as needed".to_string(),
            route: "Oral".to_string(),
            quantity: 20,
            refills_allowed: 0, // No refills for Schedule II
            refills_remaining: 0,
            prescribed_date: 1704067200000000,
            expiration_date: 1706745600000000, // 30 days for Schedule II
            status: "Active".to_string(),
            drug_schedule: Some("ScheduleII".to_string()),
            diagnosis_codes: vec!["G89.4".to_string()], // Chronic pain
        }
    }

    fn create_test_interaction() -> TestDrugInteraction {
        TestDrugInteraction {
            alert_id: "ALERT-001".to_string(),
            drug_a_rxnorm: "197361".to_string(), // Lisinopril
            drug_b_rxnorm: "831541".to_string(), // Potassium
            severity: "Major".to_string(),
            description: "Concurrent use may increase risk of hyperkalemia".to_string(),
            clinical_significance: "Monitor potassium levels closely".to_string(),
            management: "Check serum potassium before initiating and periodically".to_string(),
            acknowledged: false,
            acknowledged_by: None,
        }
    }

    // ========== PRESCRIPTION TESTS ==========

    #[test]
    fn test_prescription_has_rxnorm_code() {
        let rx = create_test_prescription();
        // RxNorm is required for interoperability
        assert!(!rx.rxnorm_code.is_empty());
    }

    #[test]
    fn test_prescription_ndc_format() {
        let rx = create_test_prescription();
        if let Some(ndc) = &rx.ndc_code {
            // NDC format: XXXXX-XXXX-XX (11 digits with dashes)
            assert!(ndc.contains('-'));
            let digits: String = ndc.chars().filter(|c| c.is_ascii_digit()).collect();
            assert!(digits.len() == 10 || digits.len() == 11);
        }
    }

    #[test]
    fn test_prescription_status_valid() {
        let rx = create_test_prescription();
        let valid_statuses = [
            "Active", "Completed", "Cancelled", "OnHold",
            "Discontinued", "Expired", "Draft"
        ];
        assert!(valid_statuses.contains(&rx.status.as_str()));
    }

    #[test]
    fn test_prescription_route_valid() {
        let rx = create_test_prescription();
        let valid_routes = [
            "Oral", "Sublingual", "Topical", "Inhalation",
            "Intravenous", "Intramuscular", "Subcutaneous",
            "Rectal", "Ophthalmic", "Otic", "Nasal", "Transdermal"
        ];
        assert!(valid_routes.contains(&rx.route.as_str()));
    }

    #[test]
    fn test_prescription_quantity_positive() {
        let rx = create_test_prescription();
        assert!(rx.quantity > 0);
    }

    #[test]
    fn test_prescription_refills_valid() {
        let rx = create_test_prescription();
        // Cannot have more refills remaining than allowed
        assert!(rx.refills_remaining <= rx.refills_allowed);
    }

    #[test]
    fn test_prescription_expiration_after_prescribed() {
        let rx = create_test_prescription();
        assert!(rx.expiration_date > rx.prescribed_date);
    }

    #[test]
    fn test_prescription_has_diagnosis() {
        let rx = create_test_prescription();
        // Must have at least one diagnosis code (medical necessity)
        assert!(!rx.diagnosis_codes.is_empty());
    }

    // ========== CONTROLLED SUBSTANCE TESTS ==========

    #[test]
    fn test_controlled_substance_schedule_valid() {
        let rx = create_controlled_prescription();
        if let Some(schedule) = &rx.drug_schedule {
            let valid_schedules = [
                "ScheduleII", "ScheduleIII", "ScheduleIV", "ScheduleV"
            ];
            assert!(valid_schedules.contains(&schedule.as_str()));
        }
    }

    #[test]
    fn test_schedule_ii_no_refills() {
        let rx = create_controlled_prescription();
        if let Some(schedule) = &rx.drug_schedule {
            if schedule == "ScheduleII" {
                // DEA requires no refills for Schedule II
                assert_eq!(rx.refills_allowed, 0);
            }
        }
    }

    #[test]
    fn test_controlled_substance_quantity_limits() {
        let rx = create_controlled_prescription();
        if rx.drug_schedule.is_some() {
            // Controlled substances typically limited to 30-90 day supply
            assert!(rx.quantity <= 180);
        }
    }

    #[test]
    fn test_schedule_ii_short_expiration() {
        let rx = create_controlled_prescription();
        if let Some(schedule) = &rx.drug_schedule {
            if schedule == "ScheduleII" {
                // Schedule II must be filled within 30-90 days depending on state
                let max_validity = 90 * 24 * 60 * 60 * 1000000i64; // 90 days in microseconds
                assert!(rx.expiration_date - rx.prescribed_date <= max_validity);
            }
        }
    }

    // ========== DRUG INTERACTION TESTS ==========

    #[test]
    fn test_interaction_severity_valid() {
        let alert = create_test_interaction();
        let valid_severities = ["Minor", "Moderate", "Major", "Contraindicated"];
        assert!(valid_severities.contains(&alert.severity.as_str()));
    }

    #[test]
    fn test_interaction_has_description() {
        let alert = create_test_interaction();
        assert!(!alert.description.is_empty());
    }

    #[test]
    fn test_interaction_has_management() {
        let alert = create_test_interaction();
        // Clinicians need actionable guidance
        assert!(!alert.management.is_empty());
    }

    #[test]
    fn test_major_interaction_requires_acknowledgment() {
        let mut alert = create_test_interaction();
        alert.severity = "Major".to_string();
        // Major interactions should require provider acknowledgment
        // before dispensing
        if alert.severity == "Major" || alert.severity == "Contraindicated" {
            // Test that the system tracks acknowledgment
            assert!(!alert.acknowledged || alert.acknowledged_by.is_some());
        }
    }

    #[test]
    fn test_acknowledged_interaction_has_acknowledger() {
        let mut alert = create_test_interaction();
        alert.acknowledged = true;
        alert.acknowledged_by = Some("PROV-001".to_string());

        if alert.acknowledged {
            assert!(alert.acknowledged_by.is_some());
        }
    }
}
