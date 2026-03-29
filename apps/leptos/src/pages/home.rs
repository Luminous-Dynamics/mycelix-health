// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Home — Homeostatic alignment dashboard.
//!
//! Two states:
//! - No Vault: empty except for a glowing "First Breath" seed button
//! - Vault Active: alignment sphere (drifting by allostatic load), stats, events

use leptos::prelude::*;
use crate::app::{AppState, HomeostasisState, VaultState};
use crate::zome_clients::consent::ConsentStatus;
use crate::zome_clients::records::AccessEventType;

#[component]
pub fn HomePage() -> impl IntoView {
    let homeostasis = use_context::<RwSignal<HomeostasisState>>()
        .unwrap_or_else(|| RwSignal::new(HomeostasisState::default()));
    let app = use_context::<AppState>().expect("AppState");

    let no_vault = move || app.vault.get() == VaultState::NoVault;
    let has_vault = move || app.vault.get() != VaultState::NoVault;

    view! {
        <div class="page home-page">
            <header class="page-header">
                <h1 class="bio-title">"Homeostasis"</h1>
            </header>

            // ═══════════════════════════════════════════
            // NO VAULT: The organism hasn't taken its First Breath
            // ═══════════════════════════════════════════
            <Show when=no_vault>
                <div class="first-breath-container">
                    <a href="/welcome" class="first-breath-seed">
                        <div class="seed-pulse" />
                        <div class="seed-core" />
                    </a>
                    <h2 class="first-breath-title">"Take Your First Breath"</h2>
                    <p class="first-breath-text">
                        "Generate your biological signature to bring your health vault to life. "
                        "No data exists on the network until you do."
                    </p>
                </div>
            </Show>

            // ═══════════════════════════════════════════
            // VAULT EXISTS: The living dashboard
            // ═══════════════════════════════════════════
            <Show when=has_vault>
                <AlignmentDashboard homeostasis=homeostasis app=app.clone() />
            </Show>
        </div>
    }
}

/// The full dashboard — only rendered when vault exists.
#[component]
fn AlignmentDashboard(
    homeostasis: RwSignal<HomeostasisState>,
    app: AppState,
) -> impl IntoView {
    let alignment_pct = move || (homeostasis.get().alignment * 100.0) as u32;

    // Allostatic load: the prediction error pushing the sphere off-center.
    // At 100% alignment, drift = 0 (perfect center).
    // At 0% alignment, drift = max offset (touching the boundary).
    // Allostatic load → sphere drift. At 81%, load=0.19, drift ≈ 9px.
    // At 50%, drift ≈ 24px. At 20%, drift ≈ 38px (near boundary).
    // The angle rotates slowly so the drift direction shifts over time.
    let sphere_drift_x = move || {
        let load = 1.0 - homeostasis.get().alignment;
        // Use alignment to set angle — different alignments drift in different directions
        let angle = homeostasis.get().alignment * 4.7 + 0.8;
        (load * 48.0 * angle.cos()) as i32
    };
    let sphere_drift_y = move || {
        let load = 1.0 - homeostasis.get().alignment;
        let angle = homeostasis.get().alignment * 4.7 + 0.8;
        (load * 36.0 * angle.sin()) as i32
    };

    let active_consent_count = move || {
        app.consents.get().iter()
            .filter(|c| c.status == ConsentStatus::Active)
            .count()
    };

    let encrypted_count = move || {
        app.records.get().iter().filter(|r| r.encrypted).count()
    };

    let yield_display = move || format!("${:.0}", homeostasis.get().metabolic_yield);

    let recent_events = move || {
        app.access_events.get().iter().take(5).cloned().collect::<Vec<_>>()
    };

    view! {
        // Alignment sphere — drifts off-center by allostatic load
        <section class="alignment-card" aria-label="Homeostatic alignment">
            <div class="alignment-sphere-container">
                <div class="gravity-well" />
                <div
                    class="alignment-sphere"
                    style=move || format!(
                        "transform: translate({}px, {}px)",
                        sphere_drift_x(),
                        sphere_drift_y(),
                    )
                    role="meter"
                    aria-valuemin="0"
                    aria-valuemax="100"
                    aria-valuenow=alignment_pct
                    aria-valuetext=move || format!("{}% homeostatic alignment", alignment_pct())
                />
            </div>
            <div class="alignment-label">
                <span class="alignment-value">{alignment_pct}"%"</span>
                <span class="alignment-text">"Homeostatic Alignment"</span>
            </div>
        </section>

        // Stats
        <section class="stats-row">
            <div class="stat-card">
                <span class="stat-value">{active_consent_count}</span>
                <span class="stat-label">"Symbiotic Links"</span>
            </div>
            <div class="stat-card">
                <span class="stat-value">{encrypted_count}</span>
                <span class="stat-label">"Encrypted Tissues"</span>
            </div>
            <a href="/metabolism" class="stat-card metabolic stat-link">
                <span class="stat-value">{yield_display}</span>
                <span class="stat-label">"Metabolic Yield"</span>
            </a>
        </section>

        // Timeline
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
    }
}
