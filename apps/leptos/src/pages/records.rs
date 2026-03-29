// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Records — encrypted health tissue, each with a mycelial node.

use leptos::prelude::*;
use crate::components::mycelial_node::MycelialNode;
use crate::zome_clients::records::{mock_records, HealthRecord};

#[component]
pub fn RecordsPage() -> impl IntoView {
    let records = mock_records();

    view! {
        <div class="page records-page">
            <header class="page-header">
                <h1 class="bio-title">"Tissue"</h1>
                <p class="bio-subtitle">"Your encrypted health records"</p>
            </header>

            <div class="records-list">
                {records.into_iter().map(|record| {
                    let (encrypted, _) = signal(record.encrypted);
                    let (pathways, _) = signal(record.pathways.clone());
                    let category = record.category.clone();
                    view! {
                        <div class="record-card">
                            <div class="record-header">
                                <span class="record-category">{record.category.clone()}</span>
                                <MycelialNode
                                    encrypted=encrypted
                                    pathways=pathways
                                    category=category
                                />
                            </div>
                            <div class="record-summary">{record.summary}</div>
                            <div class="record-date">{record.date}</div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}
