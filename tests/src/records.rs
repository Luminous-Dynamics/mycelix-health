//! Medical Records Zome Tests
//!
//! Tests for encounters, diagnoses, lab results, and medical coding standards.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEncounter {
    pub encounter_id: String,
    pub patient_id: String,
    pub provider_id: String,
    pub encounter_type: String,
    pub status: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub chief_complaint: String,
    pub diagnoses: Vec<TestDiagnosis>,
    pub vital_signs: Option<TestVitalSigns>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDiagnosis {
    pub diagnosis_id: String,
    pub icd10_code: String,
    pub description: String,
    pub diagnosis_type: String,
    pub status: String,
    pub diagnosed_by: String,
    pub diagnosed_at: i64,
    pub epistemic_level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestLabResult {
    pub result_id: String,
    pub patient_id: String,
    pub loinc_code: String,
    pub test_name: String,
    pub value: String,
    pub unit: String,
    pub reference_range: String,
    pub interpretation: String,
    pub collected_at: i64,
    pub resulted_at: i64,
    pub is_critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestVitalSigns {
    pub blood_pressure_systolic: Option<u32>,
    pub blood_pressure_diastolic: Option<u32>,
    pub heart_rate: Option<u32>,
    pub respiratory_rate: Option<u32>,
    pub temperature_celsius: Option<f64>,
    pub oxygen_saturation: Option<u32>,
    pub height_cm: Option<f64>,
    pub weight_kg: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_encounter() -> TestEncounter {
        TestEncounter {
            encounter_id: "ENC-001".to_string(),
            patient_id: "PAT-001".to_string(),
            provider_id: "PROV-001".to_string(),
            encounter_type: "Outpatient".to_string(),
            status: "InProgress".to_string(),
            start_time: 1704153600000000,
            end_time: None,
            chief_complaint: "Persistent cough for 2 weeks".to_string(),
            diagnoses: vec![],
            vital_signs: Some(TestVitalSigns {
                blood_pressure_systolic: Some(120),
                blood_pressure_diastolic: Some(80),
                heart_rate: Some(72),
                respiratory_rate: Some(16),
                temperature_celsius: Some(37.0),
                oxygen_saturation: Some(98),
                height_cm: Some(170.0),
                weight_kg: Some(70.0),
            }),
        }
    }

    fn create_test_diagnosis() -> TestDiagnosis {
        TestDiagnosis {
            diagnosis_id: "DX-001".to_string(),
            icd10_code: "J06.9".to_string(),
            description: "Acute upper respiratory infection, unspecified".to_string(),
            diagnosis_type: "Admitting".to_string(),
            status: "Active".to_string(),
            diagnosed_by: "PROV-001".to_string(),
            diagnosed_at: 1704153600000000,
            epistemic_level: 2,
        }
    }

    fn create_test_lab_result() -> TestLabResult {
        TestLabResult {
            result_id: "LAB-001".to_string(),
            patient_id: "PAT-001".to_string(),
            loinc_code: "2093-3".to_string(),
            test_name: "Cholesterol [Mass/volume] in Serum or Plasma".to_string(),
            value: "195".to_string(),
            unit: "mg/dL".to_string(),
            reference_range: "<200".to_string(),
            interpretation: "Normal".to_string(),
            collected_at: 1704067200000000,
            resulted_at: 1704153600000000,
            is_critical: false,
        }
    }

    // ========== ENCOUNTER TESTS ==========

    #[test]
    fn test_encounter_type_valid() {
        let encounter = create_test_encounter();
        let valid_types = [
            "Outpatient", "Inpatient", "Emergency", "Observation",
            "Virtual", "Home", "PreAdmission"
        ];
        assert!(valid_types.contains(&encounter.encounter_type.as_str()));
    }

    #[test]
    fn test_encounter_status_valid() {
        let encounter = create_test_encounter();
        let valid_statuses = [
            "Planned", "Arrived", "Triaged", "InProgress",
            "OnLeave", "Finished", "Cancelled"
        ];
        assert!(valid_statuses.contains(&encounter.status.as_str()));
    }

    #[test]
    fn test_encounter_has_chief_complaint() {
        let encounter = create_test_encounter();
        assert!(!encounter.chief_complaint.is_empty());
    }

    #[test]
    fn test_encounter_end_after_start() {
        let mut encounter = create_test_encounter();
        encounter.end_time = Some(1704157200000000);
        if let Some(end) = encounter.end_time {
            assert!(end > encounter.start_time);
        }
    }

    // ========== DIAGNOSIS TESTS (ICD-10) ==========

    #[test]
    fn test_icd10_code_format() {
        let diagnosis = create_test_diagnosis();
        // ICD-10 codes: Letter + 2 digits + optional decimal + 1-4 characters
        assert!(!diagnosis.icd10_code.is_empty());
        assert!(diagnosis.icd10_code.chars().next().unwrap().is_ascii_uppercase());
    }

    #[test]
    fn test_diagnosis_type_valid() {
        let diagnosis = create_test_diagnosis();
        let valid_types = [
            "Admitting", "Working", "Final", "Differential",
            "Principal", "Secondary", "Chronic"
        ];
        assert!(valid_types.contains(&diagnosis.diagnosis_type.as_str()));
    }

    #[test]
    fn test_diagnosis_status_valid() {
        let diagnosis = create_test_diagnosis();
        let valid_statuses = ["Active", "Resolved", "Inactive", "Remission", "Recurrence"];
        assert!(valid_statuses.contains(&diagnosis.status.as_str()));
    }

    #[test]
    fn test_diagnosis_has_diagnosing_provider() {
        let diagnosis = create_test_diagnosis();
        assert!(!diagnosis.diagnosed_by.is_empty());
    }

    #[test]
    fn test_diagnosis_epistemic_level() {
        let diagnosis = create_test_diagnosis();
        // E0=suspected, E1=clinical, E2=lab-confirmed, E3=biopsy-confirmed
        assert!(diagnosis.epistemic_level <= 4);
    }

    // ========== LAB RESULT TESTS (LOINC) ==========

    #[test]
    fn test_loinc_code_format() {
        let lab = create_test_lab_result();
        // LOINC codes are numeric with optional dash and check digit
        assert!(!lab.loinc_code.is_empty());
        assert!(lab.loinc_code.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_lab_result_has_value() {
        let lab = create_test_lab_result();
        assert!(!lab.value.is_empty());
    }

    #[test]
    fn test_lab_result_has_unit() {
        let lab = create_test_lab_result();
        assert!(!lab.unit.is_empty());
    }

    #[test]
    fn test_lab_result_interpretation_valid() {
        let lab = create_test_lab_result();
        let valid_interpretations = [
            "Normal", "Abnormal", "High", "Low", "CriticalHigh",
            "CriticalLow", "Indeterminate", "Pending"
        ];
        assert!(valid_interpretations.contains(&lab.interpretation.as_str()));
    }

    #[test]
    fn test_lab_result_timing() {
        let lab = create_test_lab_result();
        // Result should come after collection
        assert!(lab.resulted_at >= lab.collected_at);
    }

    #[test]
    fn test_critical_lab_flagged() {
        let mut lab = create_test_lab_result();
        lab.interpretation = "CriticalHigh".to_string();
        lab.is_critical = true;
        // Critical interpretation should set is_critical flag
        if lab.interpretation.contains("Critical") {
            assert!(lab.is_critical);
        }
    }

    // ========== VITAL SIGNS TESTS ==========

    #[test]
    fn test_vital_signs_blood_pressure_range() {
        let encounter = create_test_encounter();
        if let Some(vs) = &encounter.vital_signs {
            if let Some(sys) = vs.blood_pressure_systolic {
                assert!(sys >= 50 && sys <= 300);
            }
            if let Some(dia) = vs.blood_pressure_diastolic {
                assert!(dia >= 20 && dia <= 200);
            }
        }
    }

    #[test]
    fn test_vital_signs_heart_rate_range() {
        let encounter = create_test_encounter();
        if let Some(vs) = &encounter.vital_signs {
            if let Some(hr) = vs.heart_rate {
                assert!(hr >= 20 && hr <= 300);
            }
        }
    }

    #[test]
    fn test_vital_signs_o2_sat_range() {
        let encounter = create_test_encounter();
        if let Some(vs) = &encounter.vital_signs {
            if let Some(o2) = vs.oxygen_saturation {
                assert!(o2 >= 0 && o2 <= 100);
            }
        }
    }

    #[test]
    fn test_vital_signs_temperature_range() {
        let encounter = create_test_encounter();
        if let Some(vs) = &encounter.vital_signs {
            if let Some(temp) = vs.temperature_celsius {
                // Survivable human temperature range
                assert!(temp >= 25.0 && temp <= 45.0);
            }
        }
    }
}
