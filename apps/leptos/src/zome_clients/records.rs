// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Records zome client — typed wrappers for health records.

use serde::{Deserialize, Serialize};
use crate::components::mycelial_node::CryptoPathway;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthRecord {
    pub id: String,
    pub category: String,
    pub summary: String,
    pub date: String,
    pub encrypted: bool,
    pub pathways: Vec<CryptoPathway>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessEvent {
    pub who: String,
    pub what: String,
    pub when: String,
    pub event_type: AccessEventType,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum AccessEventType {
    DataAccess,
    FlContribution,
    DividendPayout,
    ConsentChange,
    BreakGlass,
}

/// Mock records for development.
pub fn mock_records() -> Vec<HealthRecord> {
    vec![
        HealthRecord {
            id: "r-001".into(),
            category: "Lab Results".into(),
            summary: "Glucose: 85 mg/dL (normal range)".into(),
            date: "2025-03-15".into(),
            encrypted: true,
            pathways: vec![
                CryptoPathway { holder_name: "Dr. Sarah Chen".into(), active: true },
                CryptoPathway { holder_name: "Valley Medical Lab".into(), active: true },
            ],
        },
        HealthRecord {
            id: "r-002".into(),
            category: "Encounter".into(),
            summary: "Annual physical examination".into(),
            date: "2025-03-10".into(),
            encrypted: true,
            pathways: vec![
                CryptoPathway { holder_name: "Dr. Sarah Chen".into(), active: true },
            ],
        },
        HealthRecord {
            id: "r-003".into(),
            category: "Vital Signs".into(),
            summary: "BP 120/80, HR 72, SpO2 98%".into(),
            date: "2025-03-10".into(),
            encrypted: true,
            pathways: vec![
                CryptoPathway { holder_name: "Dr. Sarah Chen".into(), active: true },
            ],
        },
        HealthRecord {
            id: "r-004".into(),
            category: "Immunization".into(),
            summary: "COVID-19 booster (Moderna)".into(),
            date: "2025-01-20".into(),
            encrypted: true,
            pathways: vec![
                CryptoPathway { holder_name: "CVS Pharmacy".into(), active: false },
            ],
        },
        HealthRecord {
            id: "r-005".into(),
            category: "Medication".into(),
            summary: "Metformin 500mg twice daily".into(),
            date: "2025-02-01".into(),
            encrypted: true,
            pathways: vec![
                CryptoPathway { holder_name: "Dr. Sarah Chen".into(), active: true },
                CryptoPathway { holder_name: "Walgreens Pharmacy".into(), active: true },
            ],
        },
    ]
}

/// Mock access events for the privacy timeline.
pub fn mock_access_events() -> Vec<AccessEvent> {
    vec![
        AccessEvent {
            who: "Dr. Chen".into(),
            what: "viewed Lab Results".into(),
            when: "2h ago".into(),
            event_type: AccessEventType::DataAccess,
        },
        AccessEvent {
            who: "Federated Learning".into(),
            what: "gradient extracted (ε=1.0)".into(),
            when: "1d ago".into(),
            event_type: AccessEventType::FlContribution,
        },
        AccessEvent {
            who: "Diabetes Cohort Study".into(),
            what: "dividend $42.00".into(),
            when: "3d ago".into(),
            event_type: AccessEventType::DividendPayout,
        },
        AccessEvent {
            who: "Valley Urgent Care".into(),
            what: "consent revoked by you".into(),
            when: "5d ago".into(),
            event_type: AccessEventType::ConsentChange,
        },
    ]
}
