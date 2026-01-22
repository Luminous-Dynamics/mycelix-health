//! Patient Notification Tests
//!
//! Tests for the patient notification system that alerts patients
//! when their health data is accessed.

mod test_types {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub enum NotificationPriority {
        Immediate,
        Daily,
        Weekly,
        Silent,
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

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AccessNotification {
        pub notification_id: String,
        pub patient_hash: String,
        pub accessor: String,
        pub accessor_name: String,
        pub data_categories: Vec<DataCategory>,
        pub purpose: String,
        pub accessed_at: i64,
        pub emergency_access: bool,
        pub priority: NotificationPriority,
        pub viewed: bool,
        pub viewed_at: Option<i64>,
        pub summary: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct NotificationPreferences {
        pub patient_hash: String,
        pub default_priority: NotificationPriority,
        pub immediate_categories: Vec<DataCategory>,
        pub silent_agents: Vec<String>,
        pub notify_emergency_access: bool,
        pub notify_new_providers: bool,
        pub daily_digest_hour: Option<u8>,
        pub weekly_summary_day: Option<u8>,
        pub email_enabled: bool,
        pub email_address: Option<String>,
        pub push_enabled: bool,
        pub sms_enabled: bool,
        pub phone_number: Option<String>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub enum DigestType {
        Daily,
        Weekly,
        Monthly,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct NotificationDigest {
        pub digest_id: String,
        pub patient_hash: String,
        pub digest_type: DigestType,
        pub period_start: i64,
        pub period_end: i64,
        pub total_access_events: u32,
        pub unique_accessors: u32,
        pub categories_accessed: Vec<DataCategory>,
        pub emergency_accesses: u32,
        pub viewed: bool,
    }
}

#[cfg(test)]
mod notification_creation_tests {
    use super::test_types::*;

    /// Test notification requires accessor name for patient-friendly display
    #[test]
    fn test_notification_requires_accessor_name() {
        let valid_notification = AccessNotification {
            notification_id: "NOTIF-001".to_string(),
            patient_hash: "patient-123".to_string(),
            accessor: "provider-agent-456".to_string(),
            accessor_name: "Dr. Smith".to_string(),  // Human-readable!
            data_categories: vec![DataCategory::Medications],
            purpose: "Routine follow-up".to_string(),
            accessed_at: 1704153600000000,
            emergency_access: false,
            priority: NotificationPriority::Daily,
            viewed: false,
            viewed_at: None,
            summary: "Dr. Smith viewed your medications".to_string(),
        };

        assert!(!valid_notification.accessor_name.is_empty());
        assert!(!valid_notification.summary.is_empty());
    }

    /// Test notification summary is plain language
    #[test]
    fn test_notification_plain_language_summary() {
        fn generate_summary(
            accessor_name: &str,
            categories: &[DataCategory],
            emergency: bool,
        ) -> String {
            let categories_text: Vec<&str> = categories.iter().map(|c| match c {
                DataCategory::Demographics => "basic information",
                DataCategory::Allergies => "allergy information",
                DataCategory::Medications => "medications",
                DataCategory::Diagnoses => "diagnoses",
                DataCategory::Procedures => "procedures",
                DataCategory::LabResults => "lab results",
                DataCategory::ImagingStudies => "imaging studies",
                DataCategory::VitalSigns => "vital signs",
                DataCategory::Immunizations => "immunizations",
                DataCategory::MentalHealth => "mental health records",
                DataCategory::SubstanceAbuse => "substance abuse records",
                DataCategory::SexualHealth => "sexual health records",
                DataCategory::GeneticData => "genetic data",
                DataCategory::FinancialData => "billing information",
                DataCategory::All => "all records",
            }).collect();

            let joined = if categories_text.len() == 1 {
                categories_text[0].to_string()
            } else {
                format!("{} and {}",
                    categories_text[..categories_text.len()-1].join(", "),
                    categories_text.last().unwrap()
                )
            };

            if emergency {
                format!("{} accessed your {} in an emergency", accessor_name, joined)
            } else {
                format!("{} viewed your {}", accessor_name, joined)
            }
        }

        let summary1 = generate_summary("Dr. Smith", &[DataCategory::Medications], false);
        assert_eq!(summary1, "Dr. Smith viewed your medications");

        let summary2 = generate_summary(
            "City Hospital ER",
            &[DataCategory::Allergies, DataCategory::Medications],
            true
        );
        assert_eq!(summary2, "City Hospital ER accessed your allergy information and medications in an emergency");
    }

    /// Test emergency access always gets immediate priority
    #[test]
    fn test_emergency_access_immediate_priority() {
        fn determine_priority(
            prefs: &NotificationPreferences,
            categories: &[DataCategory],
            emergency: bool,
            new_provider: bool,
        ) -> NotificationPriority {
            // Emergency access is always immediate if user wants to know
            if emergency && prefs.notify_emergency_access {
                return NotificationPriority::Immediate;
            }

            // New providers get immediate notification if enabled
            if new_provider && prefs.notify_new_providers {
                return NotificationPriority::Immediate;
            }

            // Check if any category is in immediate list
            for cat in categories {
                if prefs.immediate_categories.contains(cat) {
                    return NotificationPriority::Immediate;
                }
            }

            // Default to user's preference
            prefs.default_priority.clone()
        }

        let prefs = NotificationPreferences {
            patient_hash: "patient-123".to_string(),
            default_priority: NotificationPriority::Daily,
            immediate_categories: vec![DataCategory::MentalHealth],
            silent_agents: vec![],
            notify_emergency_access: true,
            notify_new_providers: true,
            daily_digest_hour: Some(18),
            weekly_summary_day: Some(0),
            email_enabled: false,
            email_address: None,
            push_enabled: true,
            sms_enabled: false,
            phone_number: None,
        };

        // Emergency access → Immediate
        assert_eq!(
            determine_priority(&prefs, &[DataCategory::Medications], true, false),
            NotificationPriority::Immediate
        );

        // New provider → Immediate
        assert_eq!(
            determine_priority(&prefs, &[DataCategory::Demographics], false, true),
            NotificationPriority::Immediate
        );

        // Mental health (in immediate list) → Immediate
        assert_eq!(
            determine_priority(&prefs, &[DataCategory::MentalHealth], false, false),
            NotificationPriority::Immediate
        );

        // Regular access → Daily (default)
        assert_eq!(
            determine_priority(&prefs, &[DataCategory::Medications], false, false),
            NotificationPriority::Daily
        );
    }
}

#[cfg(test)]
mod notification_preferences_tests {
    use super::test_types::*;

    /// Test daily digest hour validation
    #[test]
    fn test_daily_digest_hour_validation() {
        fn is_valid_hour(hour: Option<u8>) -> bool {
            match hour {
                None => true,
                Some(h) => h <= 23,
            }
        }

        assert!(is_valid_hour(Some(0)));    // Midnight
        assert!(is_valid_hour(Some(12)));   // Noon
        assert!(is_valid_hour(Some(23)));   // 11 PM
        assert!(!is_valid_hour(Some(24)));  // Invalid
        assert!(!is_valid_hour(Some(25)));  // Invalid
        assert!(is_valid_hour(None));       // No digest
    }

    /// Test weekly summary day validation
    #[test]
    fn test_weekly_summary_day_validation() {
        fn is_valid_day(day: Option<u8>) -> bool {
            match day {
                None => true,
                Some(d) => d <= 6,  // 0=Sunday, 6=Saturday
            }
        }

        assert!(is_valid_day(Some(0)));   // Sunday
        assert!(is_valid_day(Some(6)));   // Saturday
        assert!(!is_valid_day(Some(7)));  // Invalid
        assert!(is_valid_day(None));      // No summary
    }

    /// Test email requires address when enabled
    #[test]
    fn test_email_requires_address() {
        fn is_valid_email_config(enabled: bool, address: &Option<String>) -> bool {
            if enabled {
                address.is_some() && !address.as_ref().unwrap().is_empty()
            } else {
                true
            }
        }

        assert!(is_valid_email_config(true, &Some("patient@example.com".to_string())));
        assert!(!is_valid_email_config(true, &None));
        assert!(!is_valid_email_config(true, &Some(String::new())));
        assert!(is_valid_email_config(false, &None));
    }

    /// Test SMS requires phone number when enabled
    #[test]
    fn test_sms_requires_phone() {
        fn is_valid_sms_config(enabled: bool, phone: &Option<String>) -> bool {
            if enabled {
                phone.is_some() && !phone.as_ref().unwrap().is_empty()
            } else {
                true
            }
        }

        assert!(is_valid_sms_config(true, &Some("555-0123".to_string())));
        assert!(!is_valid_sms_config(true, &None));
        assert!(is_valid_sms_config(false, &None));
    }

    /// Test silent agents exclude from notifications
    #[test]
    fn test_silent_agents() {
        let prefs = NotificationPreferences {
            patient_hash: "patient-123".to_string(),
            default_priority: NotificationPriority::Daily,
            immediate_categories: vec![],
            silent_agents: vec!["trusted-pcp-agent".to_string()],
            notify_emergency_access: true,
            notify_new_providers: true,
            daily_digest_hour: None,
            weekly_summary_day: None,
            email_enabled: false,
            email_address: None,
            push_enabled: false,
            sms_enabled: false,
            phone_number: None,
        };

        fn should_notify(prefs: &NotificationPreferences, accessor: &str) -> bool {
            !prefs.silent_agents.contains(&accessor.to_string())
        }

        assert!(!should_notify(&prefs, "trusted-pcp-agent"));  // Silent
        assert!(should_notify(&prefs, "new-specialist"));       // Notify
    }
}

#[cfg(test)]
mod notification_digest_tests {
    use super::test_types::*;

    /// Test digest period validation
    #[test]
    fn test_digest_period_validation() {
        fn is_valid_period(start: i64, end: i64) -> bool {
            end > start
        }

        assert!(is_valid_period(1704067200000000, 1704153600000000));  // Valid: end after start
        assert!(!is_valid_period(1704153600000000, 1704067200000000)); // Invalid: end before start
        assert!(!is_valid_period(1704067200000000, 1704067200000000)); // Invalid: same time
    }

    /// Test daily digest structure
    #[test]
    fn test_daily_digest_structure() {
        let digest = NotificationDigest {
            digest_id: "DIGEST-DAILY-001".to_string(),
            patient_hash: "patient-123".to_string(),
            digest_type: DigestType::Daily,
            period_start: 1704067200000000,  // Start of day
            period_end: 1704153600000000,    // End of day
            total_access_events: 5,
            unique_accessors: 2,
            categories_accessed: vec![
                DataCategory::Demographics,
                DataCategory::Medications,
                DataCategory::LabResults,
            ],
            emergency_accesses: 0,
            viewed: false,
        };

        assert!(matches!(digest.digest_type, DigestType::Daily));
        assert_eq!(digest.total_access_events, 5);
        assert_eq!(digest.unique_accessors, 2);
        assert!(!digest.viewed);
    }

    /// Test weekly digest summary
    #[test]
    fn test_weekly_digest_summary() {
        let digest = NotificationDigest {
            digest_id: "DIGEST-WEEKLY-001".to_string(),
            patient_hash: "patient-456".to_string(),
            digest_type: DigestType::Weekly,
            period_start: 1703462400000000,  // Week start
            period_end: 1704067200000000,    // Week end
            total_access_events: 25,
            unique_accessors: 8,
            categories_accessed: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::Diagnoses,
                DataCategory::LabResults,
            ],
            emergency_accesses: 1,
            viewed: false,
        };

        assert!(matches!(digest.digest_type, DigestType::Weekly));
        // Weekly digest covers 7 days
        let period_days = (digest.period_end - digest.period_start) / (24 * 60 * 60 * 1000000);
        assert!(period_days >= 6 && period_days <= 7);
    }
}

#[cfg(test)]
mod notification_scenario_tests {
    use super::test_types::*;

    /// Scenario: Patient gets immediate notification for emergency access
    #[test]
    fn scenario_emergency_immediate_notification() {
        let notification = AccessNotification {
            notification_id: "NOTIF-EMG-001".to_string(),
            patient_hash: "patient-123".to_string(),
            accessor: "er-doctor-agent".to_string(),
            accessor_name: "Dr. Emergency at City Hospital ER".to_string(),
            data_categories: vec![
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::Diagnoses,
            ],
            purpose: "Emergency treatment".to_string(),
            accessed_at: 1704153600000000,
            emergency_access: true,
            priority: NotificationPriority::Immediate,
            viewed: false,
            viewed_at: None,
            summary: "Dr. Emergency at City Hospital ER accessed your allergy information, medications, and diagnoses in an emergency situation".to_string(),
        };

        assert!(notification.emergency_access);
        assert!(matches!(notification.priority, NotificationPriority::Immediate));
        assert!(notification.summary.contains("emergency"));
    }

    /// Scenario: Patient receives daily digest of routine access
    #[test]
    fn scenario_daily_digest_routine_access() {
        let digest = NotificationDigest {
            digest_id: "DIGEST-001".to_string(),
            patient_hash: "patient-789".to_string(),
            digest_type: DigestType::Daily,
            period_start: 1704067200000000,
            period_end: 1704153600000000,
            total_access_events: 3,
            unique_accessors: 2,
            categories_accessed: vec![
                DataCategory::Demographics,
                DataCategory::Medications,
            ],
            emergency_accesses: 0,
            viewed: false,
        };

        // Patient can see summary without being overwhelmed
        assert_eq!(digest.total_access_events, 3);
        assert_eq!(digest.unique_accessors, 2);
        assert_eq!(digest.emergency_accesses, 0);
    }

    /// Scenario: Patient marks notification as viewed
    #[test]
    fn scenario_mark_notification_viewed() {
        let mut notification = AccessNotification {
            notification_id: "NOTIF-VIEW-001".to_string(),
            patient_hash: "patient-abc".to_string(),
            accessor: "pcp-agent".to_string(),
            accessor_name: "Dr. Primary".to_string(),
            data_categories: vec![DataCategory::LabResults],
            purpose: "Review test results".to_string(),
            accessed_at: 1704153600000000,
            emergency_access: false,
            priority: NotificationPriority::Daily,
            viewed: false,
            viewed_at: None,
            summary: "Dr. Primary viewed your lab results".to_string(),
        };

        // Initially unviewed
        assert!(!notification.viewed);
        assert!(notification.viewed_at.is_none());

        // Patient views notification
        notification.viewed = true;
        notification.viewed_at = Some(1704157200000000);

        assert!(notification.viewed);
        assert!(notification.viewed_at.is_some());
    }

    /// Scenario: Sensitive data access triggers immediate notification
    #[test]
    fn scenario_sensitive_data_immediate_notification() {
        let notification = AccessNotification {
            notification_id: "NOTIF-SENS-001".to_string(),
            patient_hash: "patient-sensitive".to_string(),
            accessor: "psychiatrist-agent".to_string(),
            accessor_name: "Dr. Therapist".to_string(),
            data_categories: vec![DataCategory::MentalHealth],
            purpose: "Therapy session notes".to_string(),
            accessed_at: 1704153600000000,
            emergency_access: false,
            priority: NotificationPriority::Immediate,  // Sensitive = Immediate
            viewed: false,
            viewed_at: None,
            summary: "Dr. Therapist viewed your mental health records".to_string(),
        };

        // Mental health access should be immediate priority
        assert!(notification.data_categories.contains(&DataCategory::MentalHealth));
        assert!(matches!(notification.priority, NotificationPriority::Immediate));
    }
}
