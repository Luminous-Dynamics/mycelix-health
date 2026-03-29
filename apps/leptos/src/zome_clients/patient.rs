// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Patient zome client — typed wrappers for patient operations.

use serde::{Deserialize, Serialize};

/// Patient profile (mirrors patient_integrity::Patient).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientProfile {
    pub given_name: String,
    pub family_name: String,
    pub date_of_birth: String,
    pub gender: String,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub emergency_contact_name: Option<String>,
    pub emergency_contact_phone: Option<String>,
}

/// Mock patient profile for development.
pub fn mock_patient() -> PatientProfile {
    PatientProfile {
        given_name: "Alex".into(),
        family_name: "Rivera".into(),
        date_of_birth: "1990-06-15".into(),
        gender: "Non-binary".into(),
        contact_email: Some("alex@example.com".into()),
        contact_phone: Some("+1-555-0142".into()),
        emergency_contact_name: Some("Jordan Rivera".into()),
        emergency_contact_phone: Some("+1-555-0198".into()),
    }
}
