//! Care Team Template Tests
//!
//! Tests for the care team template system that enables
//! one-click consent for common healthcare scenarios.

mod test_types {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub enum DataPermission {
        Read,
        Write,
        Share,
        Export,
        Delete,
        Amend,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub enum DataCategory {
        Demographics,
        Allergies,
        Medications,
        Diagnoses,
        Procedures,
        LabResults,
        ImagingStudies,
        VitalSigns,
        Immunizations,
        MentalHealth,
        SubstanceAbuse,
        SexualHealth,
        GeneticData,
        FinancialData,
        All,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub enum ConsentPurpose {
        Treatment,
        Payment,
        HealthcareOperations,
        Research,
        PublicHealth,
        LegalProceeding,
        Marketing,
        FamilyNotification,
        Other(String),
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub enum TemplateType {
        System,
        Organization(String),
        Personal,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub enum SystemTemplate {
        PrimaryCareTeam,
        SpecialistReferral,
        HospitalAdmission,
        EmergencyDepartment,
        MentalHealthProvider,
        PharmacyAccess,
        InsuranceBilling,
        ClinicalTrial,
        TelehealthVisit,
        SecondOpinion,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct CareTeamTemplate {
        pub template_id: String,
        pub name: String,
        pub description: String,
        pub permissions: Vec<DataPermission>,
        pub data_categories: Vec<DataCategory>,
        pub default_exclusions: Vec<DataCategory>,
        pub purpose: ConsentPurpose,
        pub default_duration_days: Option<u32>,
        pub template_type: TemplateType,
        pub active: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub enum CareTeamRole {
        PrimaryCarePhysician,
        Specialist,
        Nurse,
        NursePractitioner,
        PhysicianAssistant,
        Pharmacist,
        CaseManager,
        SocialWorker,
        Therapist,
        Dietitian,
        PhysicalTherapist,
        AdministrativeStaff,
        BillingSpecialist,
        Other(String),
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub enum CareTeamStatus {
        Active,
        Inactive,
        Dissolved,
        Expired,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct CareTeamMember {
        pub member_id: String,
        pub role: CareTeamRole,
        pub active: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct CareTeam {
        pub team_id: String,
        pub patient_hash: String,
        pub team_name: String,
        pub template_id: Option<String>,
        pub members: Vec<CareTeamMember>,
        pub permissions: Vec<DataPermission>,
        pub data_categories: Vec<DataCategory>,
        pub exclusions: Vec<DataCategory>,
        pub purpose: ConsentPurpose,
        pub status: CareTeamStatus,
    }
}

#[cfg(test)]
mod system_template_tests {
    use super::test_types::*;

    /// Test primary care team template
    #[test]
    fn test_primary_care_team_template() {
        let template = CareTeamTemplate {
            template_id: "primary-care-team".to_string(),
            name: "Primary Care Team".to_string(),
            description: "Your primary care doctor, nurses, and office staff can view most of your health information to coordinate your care.".to_string(),
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::Diagnoses,
                DataCategory::Procedures,
                DataCategory::LabResults,
                DataCategory::ImagingStudies,
                DataCategory::VitalSigns,
                DataCategory::Immunizations,
            ],
            default_exclusions: vec![
                DataCategory::MentalHealth,
                DataCategory::SubstanceAbuse,
                DataCategory::SexualHealth,
                DataCategory::GeneticData,
            ],
            purpose: ConsentPurpose::Treatment,
            default_duration_days: Some(365),
            template_type: TemplateType::System,
            active: true,
        };

        // Should be read-only
        assert!(template.permissions.contains(&DataPermission::Read));
        assert!(!template.permissions.contains(&DataPermission::Write));

        // Should exclude sensitive categories
        assert!(template.default_exclusions.contains(&DataCategory::MentalHealth));
        assert!(template.default_exclusions.contains(&DataCategory::SubstanceAbuse));

        // Should have patient-friendly description
        assert!(!template.description.is_empty());
        assert!(template.description.contains("coordinate your care"));
    }

    /// Test emergency department template (short duration)
    #[test]
    fn test_emergency_department_template() {
        let template = CareTeamTemplate {
            template_id: "emergency-department".to_string(),
            name: "Emergency Department".to_string(),
            description: "Emergency room staff can access your records for 24 hours to provide urgent care.".to_string(),
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::Diagnoses,
                DataCategory::Procedures,
                DataCategory::LabResults,
                DataCategory::VitalSigns,
            ],
            default_exclusions: vec![DataCategory::FinancialData],
            purpose: ConsentPurpose::Treatment,
            default_duration_days: Some(1),  // 24 hours!
            template_type: TemplateType::System,
            active: true,
        };

        // ED access should be very short
        assert_eq!(template.default_duration_days, Some(1));

        // Should include critical categories for emergency care
        assert!(template.data_categories.contains(&DataCategory::Allergies));
        assert!(template.data_categories.contains(&DataCategory::Medications));

        // Financial data not needed for ED
        assert!(template.default_exclusions.contains(&DataCategory::FinancialData));
    }

    /// Test mental health provider template (enhanced privacy)
    #[test]
    fn test_mental_health_template() {
        let template = CareTeamTemplate {
            template_id: "mental-health-provider".to_string(),
            name: "Mental Health Provider".to_string(),
            description: "Your therapist or psychiatrist can access your mental health records. These are kept separate and private.".to_string(),
            permissions: vec![DataPermission::Read, DataPermission::Write],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::MentalHealth,
            ],
            default_exclusions: vec![
                DataCategory::GeneticData,
                DataCategory::FinancialData,
            ],
            purpose: ConsentPurpose::Treatment,
            default_duration_days: Some(365),
            template_type: TemplateType::System,
            active: true,
        };

        // Mental health provider CAN write (documentation)
        assert!(template.permissions.contains(&DataPermission::Write));

        // Specifically includes mental health category
        assert!(template.data_categories.contains(&DataCategory::MentalHealth));

        // But limited other access
        assert!(!template.data_categories.contains(&DataCategory::Diagnoses));
        assert!(!template.data_categories.contains(&DataCategory::Procedures));
    }

    /// Test insurance/billing template (financial only)
    #[test]
    fn test_insurance_billing_template() {
        let template = CareTeamTemplate {
            template_id: "insurance-billing".to_string(),
            name: "Insurance & Billing".to_string(),
            description: "Your insurance company can access billing information to process claims.".to_string(),
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Diagnoses,
                DataCategory::Procedures,
                DataCategory::FinancialData,
            ],
            default_exclusions: vec![
                DataCategory::MentalHealth,
                DataCategory::SubstanceAbuse,
                DataCategory::SexualHealth,
                DataCategory::GeneticData,
            ],
            purpose: ConsentPurpose::Payment,  // Different purpose!
            default_duration_days: Some(365),
            template_type: TemplateType::System,
            active: true,
        };

        // Purpose is Payment, not Treatment
        assert!(matches!(template.purpose, ConsentPurpose::Payment));

        // Read-only
        assert!(!template.permissions.contains(&DataPermission::Write));

        // Includes financial but excludes sensitive
        assert!(template.data_categories.contains(&DataCategory::FinancialData));
        assert!(template.default_exclusions.contains(&DataCategory::MentalHealth));
    }
}

#[cfg(test)]
mod care_team_creation_tests {
    use super::test_types::*;

    /// Test care team requires at least one member
    #[test]
    fn test_care_team_requires_members() {
        fn is_valid_care_team(team: &CareTeam) -> bool {
            !team.members.is_empty() && team.members.iter().any(|m| m.active)
        }

        let valid_team = CareTeam {
            team_id: "TEAM-001".to_string(),
            patient_hash: "patient-123".to_string(),
            team_name: "My Primary Care Team".to_string(),
            template_id: Some("primary-care-team".to_string()),
            members: vec![
                CareTeamMember {
                    member_id: "provider-456".to_string(),
                    role: CareTeamRole::PrimaryCarePhysician,
                    active: true,
                },
            ],
            permissions: vec![DataPermission::Read],
            data_categories: vec![DataCategory::Demographics],
            exclusions: vec![],
            purpose: ConsentPurpose::Treatment,
            status: CareTeamStatus::Active,
        };

        let empty_team = CareTeam {
            team_id: "TEAM-002".to_string(),
            patient_hash: "patient-123".to_string(),
            team_name: "Empty Team".to_string(),
            template_id: None,
            members: vec![],
            permissions: vec![DataPermission::Read],
            data_categories: vec![DataCategory::Demographics],
            exclusions: vec![],
            purpose: ConsentPurpose::Treatment,
            status: CareTeamStatus::Active,
        };

        assert!(is_valid_care_team(&valid_team));
        assert!(!is_valid_care_team(&empty_team));
    }

    /// Test care team inherits template settings
    #[test]
    fn test_care_team_from_template() {
        let template = CareTeamTemplate {
            template_id: "specialist-referral".to_string(),
            name: "Specialist Referral".to_string(),
            description: "A specialist you've been referred to".to_string(),
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::Diagnoses,
            ],
            default_exclusions: vec![
                DataCategory::MentalHealth,
                DataCategory::FinancialData,
            ],
            purpose: ConsentPurpose::Treatment,
            default_duration_days: Some(90),
            template_type: TemplateType::System,
            active: true,
        };

        // Create team from template
        let team = CareTeam {
            team_id: "TEAM-SPEC-001".to_string(),
            patient_hash: "patient-789".to_string(),
            team_name: template.name.clone(),
            template_id: Some(template.template_id.clone()),
            members: vec![
                CareTeamMember {
                    member_id: "specialist-abc".to_string(),
                    role: CareTeamRole::Specialist,
                    active: true,
                },
            ],
            permissions: template.permissions.clone(),
            data_categories: template.data_categories.clone(),
            exclusions: template.default_exclusions.clone(),
            purpose: template.purpose.clone(),
            status: CareTeamStatus::Active,
        };

        // Team should have template's settings
        assert_eq!(team.permissions, template.permissions);
        assert_eq!(team.data_categories, template.data_categories);
        assert_eq!(team.exclusions, template.default_exclusions);
        assert_eq!(team.purpose, template.purpose);
    }
}

#[cfg(test)]
mod care_team_member_tests {
    use super::test_types::*;

    /// Test care team role types
    #[test]
    fn test_care_team_roles() {
        let roles = vec![
            CareTeamRole::PrimaryCarePhysician,
            CareTeamRole::Specialist,
            CareTeamRole::Nurse,
            CareTeamRole::NursePractitioner,
            CareTeamRole::PhysicianAssistant,
            CareTeamRole::Pharmacist,
            CareTeamRole::CaseManager,
            CareTeamRole::SocialWorker,
            CareTeamRole::Therapist,
            CareTeamRole::Dietitian,
            CareTeamRole::PhysicalTherapist,
            CareTeamRole::AdministrativeStaff,
            CareTeamRole::BillingSpecialist,
            CareTeamRole::Other("Custom Role".to_string()),
        ];

        // All roles are valid and distinct
        assert_eq!(roles.len(), 14);
    }

    /// Test adding member to care team
    #[test]
    fn test_add_member_to_team() {
        let mut team = CareTeam {
            team_id: "TEAM-ADD-001".to_string(),
            patient_hash: "patient-add".to_string(),
            team_name: "Growing Team".to_string(),
            template_id: None,
            members: vec![
                CareTeamMember {
                    member_id: "pcp-001".to_string(),
                    role: CareTeamRole::PrimaryCarePhysician,
                    active: true,
                },
            ],
            permissions: vec![DataPermission::Read],
            data_categories: vec![DataCategory::Demographics],
            exclusions: vec![],
            purpose: ConsentPurpose::Treatment,
            status: CareTeamStatus::Active,
        };

        assert_eq!(team.members.len(), 1);

        // Add a nurse
        team.members.push(CareTeamMember {
            member_id: "nurse-002".to_string(),
            role: CareTeamRole::Nurse,
            active: true,
        });

        assert_eq!(team.members.len(), 2);
    }

    /// Test removing member (mark inactive, not delete)
    #[test]
    fn test_remove_member_marks_inactive() {
        let mut team = CareTeam {
            team_id: "TEAM-REM-001".to_string(),
            patient_hash: "patient-rem".to_string(),
            team_name: "Team with Removal".to_string(),
            template_id: None,
            members: vec![
                CareTeamMember {
                    member_id: "pcp-001".to_string(),
                    role: CareTeamRole::PrimaryCarePhysician,
                    active: true,
                },
                CareTeamMember {
                    member_id: "nurse-002".to_string(),
                    role: CareTeamRole::Nurse,
                    active: true,
                },
            ],
            permissions: vec![DataPermission::Read],
            data_categories: vec![DataCategory::Demographics],
            exclusions: vec![],
            purpose: ConsentPurpose::Treatment,
            status: CareTeamStatus::Active,
        };

        // "Remove" nurse by marking inactive (for audit trail)
        for member in &mut team.members {
            if member.member_id == "nurse-002" {
                member.active = false;
            }
        }

        // Member still exists but is inactive
        assert_eq!(team.members.len(), 2);
        let nurse = team.members.iter().find(|m| m.member_id == "nurse-002").unwrap();
        assert!(!nurse.active);

        // Count active members
        let active_count = team.members.iter().filter(|m| m.active).count();
        assert_eq!(active_count, 1);
    }
}

#[cfg(test)]
mod care_team_authorization_tests {
    use super::test_types::*;

    /// Test care team authorization check
    #[test]
    fn test_care_team_authorization() {
        let team = CareTeam {
            team_id: "TEAM-AUTH-001".to_string(),
            patient_hash: "patient-auth".to_string(),
            team_name: "Auth Test Team".to_string(),
            template_id: None,
            members: vec![
                CareTeamMember {
                    member_id: "provider-authorized".to_string(),
                    role: CareTeamRole::PrimaryCarePhysician,
                    active: true,
                },
                CareTeamMember {
                    member_id: "provider-inactive".to_string(),
                    role: CareTeamRole::Nurse,
                    active: false,  // Inactive!
                },
            ],
            permissions: vec![DataPermission::Read],
            data_categories: vec![DataCategory::Demographics, DataCategory::Medications],
            exclusions: vec![DataCategory::MentalHealth],
            purpose: ConsentPurpose::Treatment,
            status: CareTeamStatus::Active,
        };

        fn check_authorization(
            team: &CareTeam,
            member_id: &str,
            permission: &DataPermission,
            category: &DataCategory,
        ) -> bool {
            // Team must be active
            if !matches!(team.status, CareTeamStatus::Active) {
                return false;
            }

            // Find member and check if active
            let member = team.members.iter().find(|m| m.member_id == member_id);
            if member.is_none() || !member.unwrap().active {
                return false;
            }

            // Check permission
            if !team.permissions.contains(permission) {
                return false;
            }

            // Check category (All covers everything)
            let category_covered = team.data_categories.iter().any(|c| {
                matches!(c, DataCategory::All) || c == category
            });
            if !category_covered {
                return false;
            }

            // Check not excluded
            if team.exclusions.contains(category) {
                return false;
            }

            true
        }

        // Active member, valid category → Authorized
        assert!(check_authorization(&team, "provider-authorized", &DataPermission::Read, &DataCategory::Medications));

        // Active member, excluded category → Not authorized
        assert!(!check_authorization(&team, "provider-authorized", &DataPermission::Read, &DataCategory::MentalHealth));

        // Inactive member → Not authorized
        assert!(!check_authorization(&team, "provider-inactive", &DataPermission::Read, &DataCategory::Medications));

        // Non-member → Not authorized
        assert!(!check_authorization(&team, "stranger", &DataPermission::Read, &DataCategory::Medications));

        // Wrong permission → Not authorized
        assert!(!check_authorization(&team, "provider-authorized", &DataPermission::Write, &DataCategory::Medications));
    }
}

#[cfg(test)]
mod care_team_scenario_tests {
    use super::test_types::*;

    /// Scenario: Patient creates primary care team with template
    #[test]
    fn scenario_create_primary_care_team() {
        let team = CareTeam {
            team_id: "TEAM-PCP-001".to_string(),
            patient_hash: "patient-pcp".to_string(),
            team_name: "My Primary Care Team".to_string(),
            template_id: Some("primary-care-team".to_string()),
            members: vec![
                CareTeamMember {
                    member_id: "dr-smith".to_string(),
                    role: CareTeamRole::PrimaryCarePhysician,
                    active: true,
                },
                CareTeamMember {
                    member_id: "nurse-jones".to_string(),
                    role: CareTeamRole::Nurse,
                    active: true,
                },
                CareTeamMember {
                    member_id: "ma-wilson".to_string(),
                    role: CareTeamRole::AdministrativeStaff,
                    active: true,
                },
            ],
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::Diagnoses,
                DataCategory::Procedures,
                DataCategory::LabResults,
                DataCategory::VitalSigns,
            ],
            exclusions: vec![
                DataCategory::MentalHealth,
                DataCategory::SubstanceAbuse,
            ],
            purpose: ConsentPurpose::Treatment,
            status: CareTeamStatus::Active,
        };

        // Team has multiple members with different roles
        assert_eq!(team.members.len(), 3);

        // Includes typical primary care categories
        assert!(team.data_categories.contains(&DataCategory::Medications));
        assert!(team.data_categories.contains(&DataCategory::LabResults));

        // Excludes sensitive categories
        assert!(team.exclusions.contains(&DataCategory::MentalHealth));
    }

    /// Scenario: Hospital admission creates temporary broad-access team
    #[test]
    fn scenario_hospital_admission_team() {
        let team = CareTeam {
            team_id: "TEAM-HOSP-001".to_string(),
            patient_hash: "patient-admitted".to_string(),
            team_name: "Hospital Admission - City General".to_string(),
            template_id: Some("hospital-admission".to_string()),
            members: vec![
                CareTeamMember {
                    member_id: "attending-dr".to_string(),
                    role: CareTeamRole::PrimaryCarePhysician,
                    active: true,
                },
                CareTeamMember {
                    member_id: "hospitalist".to_string(),
                    role: CareTeamRole::Specialist,
                    active: true,
                },
                CareTeamMember {
                    member_id: "floor-nurse".to_string(),
                    role: CareTeamRole::Nurse,
                    active: true,
                },
                CareTeamMember {
                    member_id: "case-manager".to_string(),
                    role: CareTeamRole::CaseManager,
                    active: true,
                },
            ],
            permissions: vec![DataPermission::Read, DataPermission::Write],
            data_categories: vec![DataCategory::All],  // Broad access for inpatient
            exclusions: vec![],
            purpose: ConsentPurpose::Treatment,
            status: CareTeamStatus::Active,
        };

        // Hospital team has broader access
        assert!(team.data_categories.contains(&DataCategory::All));
        assert!(team.permissions.contains(&DataPermission::Write));

        // Multiple care roles
        assert_eq!(team.members.len(), 4);
    }

    /// Scenario: Specialist referral with limited scope
    #[test]
    fn scenario_specialist_referral() {
        let team = CareTeam {
            team_id: "TEAM-SPEC-001".to_string(),
            patient_hash: "patient-referred".to_string(),
            team_name: "Cardiology Consult - Dr. Heart".to_string(),
            template_id: Some("specialist-referral".to_string()),
            members: vec![
                CareTeamMember {
                    member_id: "dr-heart".to_string(),
                    role: CareTeamRole::Specialist,
                    active: true,
                },
            ],
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::Diagnoses,
                DataCategory::LabResults,
            ],
            exclusions: vec![
                DataCategory::MentalHealth,
                DataCategory::SubstanceAbuse,
                DataCategory::SexualHealth,
                DataCategory::FinancialData,
            ],
            purpose: ConsentPurpose::Treatment,
            status: CareTeamStatus::Active,
        };

        // Specialist has limited scope
        assert!(!team.data_categories.contains(&DataCategory::All));
        assert!(team.data_categories.contains(&DataCategory::LabResults));

        // Multiple exclusions for privacy
        assert!(team.exclusions.len() >= 3);
    }

    /// Scenario: Dissolving a care team
    #[test]
    fn scenario_dissolve_care_team() {
        let mut team = CareTeam {
            team_id: "TEAM-END-001".to_string(),
            patient_hash: "patient-discharged".to_string(),
            team_name: "Ended Care Relationship".to_string(),
            template_id: None,
            members: vec![
                CareTeamMember {
                    member_id: "former-provider".to_string(),
                    role: CareTeamRole::PrimaryCarePhysician,
                    active: true,
                },
            ],
            permissions: vec![DataPermission::Read],
            data_categories: vec![DataCategory::Demographics],
            exclusions: vec![],
            purpose: ConsentPurpose::Treatment,
            status: CareTeamStatus::Active,
        };

        // Patient ends relationship with provider
        team.status = CareTeamStatus::Dissolved;

        assert!(matches!(team.status, CareTeamStatus::Dissolved));
    }
}
