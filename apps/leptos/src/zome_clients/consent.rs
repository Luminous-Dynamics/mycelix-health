// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Consent zome client — typed wrappers for consent operations.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsentSummary {
    pub id: String,
    pub grantee_name: String,
    pub categories: Vec<String>,
    pub purpose: String,
    pub status: ConsentStatus,
    pub granted_at: String,
    pub expires_at: Option<String>,
    pub is_sensitive: bool,
    pub no_further_disclosure: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ConsentStatus {
    Active,
    Revoked,
    Expired,
}

/// Mock consents for development.
pub fn mock_consents() -> Vec<ConsentSummary> {
    vec![
        ConsentSummary {
            id: "c-001".into(),
            grantee_name: "Dr. Sarah Chen".into(),
            categories: vec!["Lab Results".into(), "Medications".into(), "Vital Signs".into()],
            purpose: "Treatment".into(),
            status: ConsentStatus::Active,
            granted_at: "2025-01-15".into(),
            expires_at: Some("2026-12-15".into()),
            is_sensitive: false,
            no_further_disclosure: true,
        },
        ConsentSummary {
            id: "c-002".into(),
            grantee_name: "Dr. James Park".into(),
            categories: vec!["Substance Abuse Treatment".into()],
            purpose: "Treatment".into(),
            status: ConsentStatus::Active,
            granted_at: "2025-03-01".into(),
            expires_at: None,
            is_sensitive: true,
            no_further_disclosure: true,
        },
        ConsentSummary {
            id: "c-003".into(),
            grantee_name: "Valley Urgent Care".into(),
            categories: vec!["Demographics".into(), "Allergies".into()],
            purpose: "Treatment".into(),
            status: ConsentStatus::Revoked,
            granted_at: "2024-11-20".into(),
            expires_at: None,
            is_sensitive: false,
            no_further_disclosure: false,
        },
    ]
}
