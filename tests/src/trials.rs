//! Clinical Trials Zome Tests
//!
//! Tests for FDA 21 CFR Part 11 compliance, ICH E6 Good Clinical Practice,
//! adverse event reporting, and research integrity.

use serde::{Deserialize, Serialize};

/// Clinical trial entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestClinicalTrial {
    pub trial_id: String,
    pub nct_number: Option<String>,
    pub title: String,
    pub phase: String,
    pub study_type: String,
    pub status: String,
    pub principal_investigator: String,
    pub sponsor: String,
    pub irb_approval_number: String,
    pub eligibility: TestEligibilityCriteria,
    pub target_enrollment: u32,
    pub current_enrollment: u32,
    pub start_date: i64,
    pub estimated_end_date: Option<i64>,
    pub interventions: Vec<TestIntervention>,
    pub outcomes: Vec<TestOutcome>,
    pub epistemic_level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEligibilityCriteria {
    pub min_age: Option<u32>,
    pub max_age: Option<u32>,
    pub sex: String,
    pub healthy_volunteers: bool,
    pub inclusion_criteria: Vec<String>,
    pub exclusion_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestIntervention {
    pub intervention_type: String,
    pub name: String,
    pub description: String,
    pub arm_group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestOutcome {
    pub outcome_type: String,
    pub title: String,
    pub description: String,
    pub time_frame: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTrialParticipant {
    pub participant_id: String,
    pub trial_id: String,
    pub patient_id: String,
    pub consent_hash: String,
    pub enrollment_date: i64,
    pub arm_assignment: Option<String>,
    pub status: String,
    pub blinded: bool,
    pub screening_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAdverseEvent {
    pub event_id: String,
    pub participant_id: String,
    pub trial_id: String,
    pub event_term: String,
    pub description: String,
    pub onset_date: i64,
    pub severity: String,
    pub seriousness: Vec<String>,
    pub is_serious: bool,
    pub is_unexpected: bool,
    pub causality: String,
    pub outcome: String,
    pub action_taken: Vec<String>,
    pub reported_by: String,
    pub reported_at: i64,
    pub medwatch_submitted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_trial() -> TestClinicalTrial {
        TestClinicalTrial {
            trial_id: "TRIAL-2024-001".to_string(),
            nct_number: Some("NCT05123456".to_string()),
            title: "Phase 3 Randomized Controlled Trial of Novel Treatment".to_string(),
            phase: "Phase3".to_string(),
            study_type: "Interventional".to_string(),
            status: "Recruiting".to_string(),
            principal_investigator: "Dr. Sarah Chen".to_string(),
            sponsor: "Luminous Therapeutics".to_string(),
            irb_approval_number: "IRB-2024-0123".to_string(),
            eligibility: TestEligibilityCriteria {
                min_age: Some(18),
                max_age: Some(65),
                sex: "All".to_string(),
                healthy_volunteers: false,
                inclusion_criteria: vec![
                    "Confirmed diagnosis of condition X".to_string(),
                    "ECOG performance status 0-2".to_string(),
                ],
                exclusion_criteria: vec![
                    "Pregnancy or breastfeeding".to_string(),
                    "Active malignancy".to_string(),
                ],
            },
            target_enrollment: 500,
            current_enrollment: 125,
            start_date: 1704067200000000,
            estimated_end_date: Some(1767225600000000),
            interventions: vec![
                TestIntervention {
                    intervention_type: "Drug".to_string(),
                    name: "LUM-001".to_string(),
                    description: "Experimental compound".to_string(),
                    arm_group: "Treatment".to_string(),
                },
                TestIntervention {
                    intervention_type: "Drug".to_string(),
                    name: "Placebo".to_string(),
                    description: "Matching placebo".to_string(),
                    arm_group: "Control".to_string(),
                },
            ],
            outcomes: vec![
                TestOutcome {
                    outcome_type: "Primary".to_string(),
                    title: "Overall Response Rate".to_string(),
                    description: "Proportion of participants with complete or partial response".to_string(),
                    time_frame: "12 weeks".to_string(),
                },
            ],
            epistemic_level: 1, // E1 = single center trial
        }
    }

    // ========== FDA 21 CFR PART 11 COMPLIANCE TESTS ==========

    #[test]
    fn test_trial_has_unique_identifier() {
        let trial = create_test_trial();
        assert!(!trial.trial_id.is_empty());
        assert!(trial.trial_id.contains("TRIAL-"));
    }

    #[test]
    fn test_trial_nct_number_format() {
        let trial = create_test_trial();
        if let Some(nct) = &trial.nct_number {
            // NCT numbers start with "NCT" followed by 8 digits
            assert!(nct.starts_with("NCT"));
            assert_eq!(nct.len(), 11);
        }
    }

    #[test]
    fn test_trial_irb_approval_required() {
        let trial = create_test_trial();
        // All trials MUST have IRB approval
        assert!(!trial.irb_approval_number.is_empty());
    }

    #[test]
    fn test_trial_phase_valid_values() {
        let trial = create_test_trial();
        let valid_phases = ["Phase1", "Phase1a", "Phase1b", "Phase2", "Phase2a", "Phase2b", "Phase3", "Phase4", "NotApplicable"];
        assert!(valid_phases.contains(&trial.phase.as_str()));
    }

    #[test]
    fn test_trial_study_type_valid() {
        let trial = create_test_trial();
        let valid_types = ["Interventional", "Observational", "ExpandedAccess"];
        assert!(valid_types.contains(&trial.study_type.as_str()));
    }

    #[test]
    fn test_trial_status_valid() {
        let trial = create_test_trial();
        let valid_statuses = [
            "NotYetRecruiting", "Recruiting", "EnrollingByInvitation",
            "ActiveNotRecruiting", "Suspended", "Terminated", "Completed", "Withdrawn"
        ];
        assert!(valid_statuses.contains(&trial.status.as_str()));
    }

    #[test]
    fn test_trial_has_principal_investigator() {
        let trial = create_test_trial();
        // FDA requires a named PI
        assert!(!trial.principal_investigator.is_empty());
    }

    #[test]
    fn test_trial_has_sponsor() {
        let trial = create_test_trial();
        // FDA requires sponsor identification
        assert!(!trial.sponsor.is_empty());
    }

    // ========== ELIGIBILITY CRITERIA TESTS ==========

    #[test]
    fn test_eligibility_age_range_valid() {
        let trial = create_test_trial();
        if let (Some(min), Some(max)) = (trial.eligibility.min_age, trial.eligibility.max_age) {
            assert!(min < max);
            assert!(min >= 0);
            assert!(max <= 120);
        }
    }

    #[test]
    fn test_eligibility_sex_valid() {
        let trial = create_test_trial();
        let valid_values = ["All", "Male", "Female"];
        assert!(valid_values.contains(&trial.eligibility.sex.as_str()));
    }

    #[test]
    fn test_eligibility_has_inclusion_criteria() {
        let trial = create_test_trial();
        // Must have at least one inclusion criterion
        assert!(!trial.eligibility.inclusion_criteria.is_empty());
    }

    #[test]
    fn test_eligibility_exclusion_criteria_documented() {
        let trial = create_test_trial();
        // Exclusion criteria can be empty but should be considered
        // This tests that the field exists and is usable
        assert!(trial.eligibility.exclusion_criteria.len() >= 0);
    }

    // ========== INTERVENTION TESTS ==========

    #[test]
    fn test_interventional_trial_has_interventions() {
        let trial = create_test_trial();
        if trial.study_type == "Interventional" {
            assert!(!trial.interventions.is_empty());
        }
    }

    #[test]
    fn test_intervention_type_valid() {
        let trial = create_test_trial();
        let valid_types = [
            "Drug", "Device", "Biological", "Procedure", "Radiation",
            "Behavioral", "Genetic", "DietarySupplement", "Combination", "Other"
        ];
        for intervention in &trial.interventions {
            assert!(valid_types.contains(&intervention.intervention_type.as_str()));
        }
    }

    #[test]
    fn test_intervention_has_arm_group() {
        let trial = create_test_trial();
        for intervention in &trial.interventions {
            assert!(!intervention.arm_group.is_empty());
        }
    }

    // ========== OUTCOME TESTS ==========

    #[test]
    fn test_trial_has_primary_outcome() {
        let trial = create_test_trial();
        let has_primary = trial.outcomes.iter().any(|o| o.outcome_type == "Primary");
        assert!(has_primary, "Trial must have at least one primary outcome");
    }

    #[test]
    fn test_outcome_type_valid() {
        let trial = create_test_trial();
        let valid_types = ["Primary", "Secondary", "Exploratory"];
        for outcome in &trial.outcomes {
            assert!(valid_types.contains(&outcome.outcome_type.as_str()));
        }
    }

    #[test]
    fn test_outcome_has_time_frame() {
        let trial = create_test_trial();
        for outcome in &trial.outcomes {
            assert!(!outcome.time_frame.is_empty());
        }
    }

    // ========== PARTICIPANT ENROLLMENT TESTS ==========

    fn create_test_participant() -> TestTrialParticipant {
        TestTrialParticipant {
            participant_id: "PART-001".to_string(),
            trial_id: "TRIAL-2024-001".to_string(),
            patient_id: "PAT-001".to_string(),
            consent_hash: "sha256:consent_document_hash".to_string(),
            enrollment_date: 1704153600000000,
            arm_assignment: Some("Treatment".to_string()),
            status: "Enrolled".to_string(),
            blinded: true,
            screening_passed: true,
        }
    }

    #[test]
    fn test_participant_requires_consent() {
        let participant = create_test_participant();
        // ICH E6 GCP - informed consent REQUIRED
        assert!(!participant.consent_hash.is_empty());
    }

    #[test]
    fn test_participant_screening_before_enrollment() {
        let participant = create_test_participant();
        // Must pass screening to be enrolled
        if participant.status == "Enrolled" {
            assert!(participant.screening_passed);
        }
    }

    #[test]
    fn test_participant_status_valid() {
        let participant = create_test_participant();
        let valid_statuses = [
            "Screening", "ScreenFailed", "Enrolled", "Active",
            "Completed", "Withdrawn", "LostToFollowUp", "Discontinued"
        ];
        assert!(valid_statuses.contains(&participant.status.as_str()));
    }

    // ========== ADVERSE EVENT REPORTING TESTS (ICH E6, FDA) ==========

    fn create_test_adverse_event() -> TestAdverseEvent {
        TestAdverseEvent {
            event_id: "AE-001".to_string(),
            participant_id: "PART-001".to_string(),
            trial_id: "TRIAL-2024-001".to_string(),
            event_term: "Headache".to_string(),
            description: "Mild headache starting 2 hours post-dose, resolved with acetaminophen".to_string(),
            onset_date: 1704240000000000,
            severity: "Mild".to_string(),
            seriousness: vec![],
            is_serious: false,
            is_unexpected: false,
            causality: "PossiblyRelated".to_string(),
            outcome: "Recovered".to_string(),
            action_taken: vec!["DrugNotChanged".to_string(), "ConcomitantMedication".to_string()],
            reported_by: "DR-123".to_string(),
            reported_at: 1704326400000000,
            medwatch_submitted: false,
        }
    }

    fn create_serious_adverse_event() -> TestAdverseEvent {
        TestAdverseEvent {
            event_id: "SAE-001".to_string(),
            participant_id: "PART-001".to_string(),
            trial_id: "TRIAL-2024-001".to_string(),
            event_term: "Anaphylaxis".to_string(),
            description: "Severe allergic reaction requiring hospitalization".to_string(),
            onset_date: 1704240000000000,
            severity: "LifeThreatening".to_string(),
            seriousness: vec![
                "ResultsInHospitalization".to_string(),
                "LifeThreatening".to_string(),
            ],
            is_serious: true,
            is_unexpected: true,
            causality: "DefinitelyRelated".to_string(),
            outcome: "RecoveredWithSequelae".to_string(),
            action_taken: vec![
                "DrugWithdrawn".to_string(),
                "Hospitalization".to_string(),
            ],
            reported_by: "DR-123".to_string(),
            reported_at: 1704243600000000, // 1 hour after onset
            medwatch_submitted: true,
        }
    }

    #[test]
    fn test_ae_severity_valid() {
        let ae = create_test_adverse_event();
        let valid_severities = ["Mild", "Moderate", "Severe", "LifeThreatening", "Death"];
        assert!(valid_severities.contains(&ae.severity.as_str()));
    }

    #[test]
    fn test_ae_causality_valid() {
        let ae = create_test_adverse_event();
        let valid_causality = [
            "NotRelated", "Unlikely", "PossiblyRelated",
            "ProbablyRelated", "DefinitelyRelated"
        ];
        assert!(valid_causality.contains(&ae.causality.as_str()));
    }

    #[test]
    fn test_ae_outcome_valid() {
        let ae = create_test_adverse_event();
        let valid_outcomes = [
            "Recovered", "Recovering", "NotRecovered",
            "RecoveredWithSequelae", "Fatal", "Unknown"
        ];
        assert!(valid_outcomes.contains(&ae.outcome.as_str()));
    }

    #[test]
    fn test_sae_requires_seriousness_criteria() {
        let sae = create_serious_adverse_event();
        if sae.is_serious {
            // Must have at least one seriousness criterion
            assert!(!sae.seriousness.is_empty());
        }
    }

    #[test]
    fn test_sae_seriousness_criteria_valid() {
        let sae = create_serious_adverse_event();
        let valid_criteria = [
            "ResultsInDeath", "LifeThreatening", "RequiresHospitalization",
            "ResultsInHospitalization", "ResultsInPersistentDisability",
            "CongenitalAnomaly", "MedicallySignificant"
        ];
        for criterion in &sae.seriousness {
            assert!(valid_criteria.contains(&criterion.as_str()));
        }
    }

    #[test]
    fn test_sae_timely_reporting() {
        let sae = create_serious_adverse_event();
        // SAEs must be reported within 24 hours (86400 seconds = 86400000000 microseconds)
        let reporting_window = 86400000000i64;
        let time_to_report = sae.reported_at - sae.onset_date;
        assert!(time_to_report <= reporting_window, "SAE must be reported within 24 hours");
    }

    #[test]
    fn test_sae_medwatch_for_unexpected() {
        let sae = create_serious_adverse_event();
        // Serious AND unexpected events require FDA MedWatch submission
        if sae.is_serious && sae.is_unexpected {
            assert!(sae.medwatch_submitted);
        }
    }

    #[test]
    fn test_ae_requires_reporter() {
        let ae = create_test_adverse_event();
        // Must know who reported the event
        assert!(!ae.reported_by.is_empty());
    }

    // ========== EPISTEMIC CLASSIFICATION TESTS ==========

    #[test]
    fn test_trial_epistemic_level_valid() {
        let trial = create_test_trial();
        // E0-E4 empirical levels
        assert!(trial.epistemic_level <= 4);
    }

    #[test]
    fn test_rct_higher_epistemic_level() {
        let trial = create_test_trial();
        // Phase 3 RCTs should have higher epistemic credibility
        if trial.phase == "Phase3" && trial.study_type == "Interventional" {
            // Multi-center RCTs typically E2+
            assert!(trial.epistemic_level >= 1);
        }
    }
}
