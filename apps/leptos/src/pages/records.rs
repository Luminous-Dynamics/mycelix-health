// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Records — encrypted health tissue, each with a mycelial node.

use leptos::prelude::*;
use crate::components::mycelial_node::{MycelialNode, CryptoPathway};

#[component]
pub fn RecordsPage() -> impl IntoView {
    // Mock records — will be replaced with real zome calls
    let records = vec![
        ("Lab Results", "Glucose: within range", "2025-03-15", vec![
            CryptoPathway { holder_name: "Dr. Sarah Chen".into(), active: true },
            CryptoPathway { holder_name: "Valley Medical Lab".into(), active: true },
        ]),
        ("Encounter", "Annual physical exam", "2025-03-10", vec![
            CryptoPathway { holder_name: "Dr. Sarah Chen".into(), active: true },
        ]),
        ("Vital Signs", "BP 120/80, HR 72", "2025-03-10", vec![
            CryptoPathway { holder_name: "Dr. Sarah Chen".into(), active: true },
        ]),
        ("Immunization", "COVID-19 booster (Moderna)", "2025-01-20", vec![
            CryptoPathway { holder_name: "CVS Pharmacy".into(), active: false },
        ]),
    ];

    view! {
        <div class="page records-page">
            <header class="page-header">
                <h1 class="bio-title">"Tissue"</h1>
                <p class="bio-subtitle">"Your encrypted health records"</p>
            </header>

            <div class="records-list">
                {records.into_iter().map(|(category, summary, date, pathways)| {
                    let (encrypted, _) = signal(true);
                    let (pathways_signal, _) = signal(pathways.clone());
                    view! {
                        <div class="record-card">
                            <div class="record-header">
                                <span class="record-category">{category}</span>
                                <MycelialNode
                                    encrypted=encrypted
                                    pathways=pathways_signal
                                    category=category.to_string()
                                />
                            </div>
                            <div class="record-summary">{summary}</div>
                            <div class="record-date">{date}</div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}
