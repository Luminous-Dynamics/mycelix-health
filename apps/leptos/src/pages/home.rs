// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Home — Homeostatic alignment dashboard.
//!
//! Shows the patient's biological sovereignty state:
//! - Free Energy health alignment (sphere in gravity well)
//! - Recent access events (who touched the membrane)
//! - Active symbiotic connections (consents)
//! - Metabolic yield summary (data dividends)

use leptos::prelude::*;
use crate::app::HomeostasisState;

#[component]
pub fn HomePage() -> impl IntoView {
    let homeostasis = use_context::<RwSignal<HomeostasisState>>()
        .unwrap_or_else(|| RwSignal::new(HomeostasisState::default()));

    let alignment_pct = move || (homeostasis.get().alignment * 100.0) as u32;

    view! {
        <div class="page home-page">
            <header class="page-header">
                <h1 class="bio-title">"Homeostasis"</h1>
                <p class="bio-subtitle">"Your biological sovereignty at a glance"</p>
            </header>

            // Free Energy alignment indicator
            <section class="alignment-card" aria-label="Homeostatic alignment">
                <div class="alignment-sphere-container">
                    <div
                        class="alignment-sphere"
                        style=move || format!(
                            "transform: translate({}px, {}px)",
                            ((1.0 - homeostasis.get().alignment) * 20.0 * (js_sys::Math::random() - 0.5)) as i32,
                            ((1.0 - homeostasis.get().alignment) * 15.0 * (js_sys::Math::random() - 0.5)) as i32,
                        )
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

            // Quick stats
            <section class="stats-row">
                <div class="stat-card">
                    <span class="stat-value">"3"</span>
                    <span class="stat-label">"Symbiotic Links"</span>
                </div>
                <div class="stat-card">
                    <span class="stat-value">"12"</span>
                    <span class="stat-label">"Encrypted Tissues"</span>
                </div>
                <div class="stat-card metabolic">
                    <span class="stat-value">"$847"</span>
                    <span class="stat-label">"Metabolic Yield"</span>
                </div>
            </section>

            // Recent membrane events (access log)
            <section class="recent-events">
                <h2>"Recent Membrane Events"</h2>
                <div class="event-timeline">
                    <div class="event-item">
                        <span class="event-dot active" />
                        <span class="event-text">"Dr. Chen accessed Lab Results"</span>
                        <span class="event-time">"2h ago"</span>
                    </div>
                    <div class="event-item">
                        <span class="event-dot active" />
                        <span class="event-text">"FL Round 7 — gradient contributed"</span>
                        <span class="event-time">"1d ago"</span>
                    </div>
                    <div class="event-item">
                        <span class="event-dot metabolic" />
                        <span class="event-text">"Dividend: $42.00 from diabetes study"</span>
                        <span class="event-time">"3d ago"</span>
                    </div>
                </div>
            </section>
        </div>
    }
}
