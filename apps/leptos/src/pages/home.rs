// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Home — Homeostatic alignment dashboard, fully reactive.

use leptos::prelude::*;
use crate::app::{AppState, HomeostasisState, VaultState};
use crate::zome_clients::consent::ConsentStatus;
use crate::zome_clients::records::AccessEventType;

#[component]
pub fn HomePage() -> impl IntoView {
    let homeostasis = use_context::<RwSignal<HomeostasisState>>()
        .unwrap_or_else(|| RwSignal::new(HomeostasisState::default()));
    let app = use_context::<AppState>().expect("AppState");

    let alignment_pct = move || (homeostasis.get().alignment * 100.0) as u32;

    let active_consent_count = move || {
        app.consents.get().iter()
            .filter(|c| c.status == ConsentStatus::Active)
            .count()
    };

    let encrypted_count = move || {
        app.records.get().iter().filter(|r| r.encrypted).count()
    };

    let yield_display = move || format!("${:.0}", homeostasis.get().metabolic_yield);

    let vault_status = move || match app.vault.get() {
        VaultState::NoVault => "No Vault",
        VaultState::Locked => "Locked",
        VaultState::Unlocked => "Active",
    };

    let recent_events = move || {
        app.access_events.get().iter().take(5).cloned().collect::<Vec<_>>()
    };

    view! {
        <div class="page home-page">
            <header class="page-header">
                <h1 class="bio-title">"Homeostasis"</h1>
                <p class="bio-subtitle">
                    "Vault: " {vault_status}
                </p>
            </header>

            // Vault warning if not set up
            <Show when=move || app.vault.get() == VaultState::NoVault>
                <a href="/welcome" class="vault-warning">
                    "Your health vault is not set up. Tap here to create it."
                </a>
            </Show>

            // Free Energy alignment sphere
            <section class="alignment-card" aria-label="Homeostatic alignment">
                <div class="alignment-sphere-container">
                    <div
                        class="alignment-sphere"
                        role="meter"
                        aria-valuemin="0"
                        aria-valuemax="100"
                        aria-valuenow=alignment_pct
                        aria-valuetext=move || format!("{}% homeostatic alignment", alignment_pct())
                    />
                    <div class="gravity-well" />
                </div>
                <div class="alignment-label">
                    <span class="alignment-value">{alignment_pct}"%"</span>
                    <span class="alignment-text">"Homeostatic Alignment"</span>
                </div>
            </section>

            // Reactive stats
            <section class="stats-row">
                <div class="stat-card">
                    <span class="stat-value">{active_consent_count}</span>
                    <span class="stat-label">"Symbiotic Links"</span>
                </div>
                <div class="stat-card">
                    <span class="stat-value">{encrypted_count}</span>
                    <span class="stat-label">"Encrypted Tissues"</span>
                </div>
                <div class="stat-card metabolic">
                    <span class="stat-value">{yield_display}</span>
                    <span class="stat-label">"Metabolic Yield"</span>
                </div>
            </section>

            // Reactive access timeline
            <section class="recent-events">
                <h2>"Recent Membrane Events"</h2>
                <div class="event-timeline">
                    <For
                        each=recent_events
                        key=|e| format!("{}-{}", e.who, e.when)
                        let:event
                    >
                        <div class="event-item">
                            <span class=move || match event.event_type {
                                AccessEventType::DividendPayout => "event-dot metabolic",
                                AccessEventType::BreakGlass => "event-dot break-glass",
                                _ => "event-dot active",
                            } />
                            <span class="event-text">
                                {event.who.clone()}" "{event.what.clone()}
                            </span>
                            <span class="event-time">{event.when.clone()}</span>
                        </div>
                    </For>
                </div>
            </section>
        </div>
    }
}
