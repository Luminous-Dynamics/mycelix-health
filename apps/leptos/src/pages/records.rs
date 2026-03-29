// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Records — encrypted health tissue with vault-aware display.

use leptos::prelude::*;
use crate::app::{AppState, VaultState};
use crate::components::mycelial_node::{MycelialNode, CryptoPathway};

#[component]
pub fn RecordsPage() -> impl IntoView {
    let app = use_context::<AppState>().expect("AppState");
    let selected = RwSignal::new(Option::<String>::None);

    let vault_unlocked = move || app.vault.get() == VaultState::Unlocked;
    let records = move || app.records.get();

    view! {
        <div class="page records-page">
            <header class="page-header">
                <h1 class="bio-title">"Tissue"</h1>
                <p class="bio-subtitle">
                    {move || format!("{} encrypted records", records().len())}
                </p>
            </header>

            <Show when=move || !vault_unlocked()>
                <div class="records-locked-banner">
                    "Records are encrypted. "
                    <a href="/settings">"Unlock your vault"</a>
                    " to view contents."
                </div>
            </Show>

            <div class="records-list">
                {move || records().into_iter().map(|r| {
                    let id = r.id.clone();
                    let id2 = id.clone();
                    let id3 = id.clone();
                    let (enc, _) = signal(r.encrypted);
                    let (pw, _) = signal(r.pathways.clone());
                    let cat = r.category.clone();
                    let id_for_sel = id.clone();
                    let id_for_sel2 = id.clone();
                    let is_sel = move || selected.get().as_deref() == Some(&id_for_sel);
                    let is_sel2 = move || selected.get().as_deref() == Some(&id_for_sel2);
                    let is_enc = r.encrypted;
                    let can_view = move || vault_unlocked() || !is_enc;
                    let can_view2 = move || vault_unlocked() || !is_enc;
                    let holders = r.pathways.iter()
                        .filter(|p| p.active)
                        .map(|p| p.holder_name.clone())
                        .collect::<Vec<_>>()
                        .join(", ");

                    view! {
                        <div
                            class=move || if is_sel() { "record-card selected" } else { "record-card" }
                            on:click=move |_| {
                                if selected.get().as_deref() == Some(&id2) {
                                    selected.set(None);
                                } else {
                                    selected.set(Some(id2.clone()));
                                }
                            }
                        >
                            <div class="record-header">
                                <span class="record-category">{r.category.clone()}</span>
                                <MycelialNode encrypted=enc pathways=pw category=cat.clone() />
                            </div>

                            {move || if can_view() {
                                view! { <div class="record-summary">{r.summary.clone()}</div> }.into_any()
                            } else {
                                view! { <div class="record-encrypted">"██████ ████ ██████████"</div> }.into_any()
                            }}

                            <div class="record-date">{r.date.clone()}</div>

                            {move || if is_sel2() && can_view2() {
                                view! {
                                    <div class="record-detail">
                                        <div class="detail-section">
                                            <span class="detail-label">"Encryption"</span>
                                            <span class="detail-value">"XChaCha20-Poly1305"</span>
                                        </div>
                                        <div class="detail-section">
                                            <span class="detail-label">"Key Holders"</span>
                                            <span class="detail-value">{holders.clone()}</span>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div /> }.into_any()
                            }}
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}
