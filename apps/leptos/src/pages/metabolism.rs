// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Metabolism — data dividends as metabolic yield.
//!
//! Not a bank balance. Energy flowing back through the research ecosystem
//! to sustain the patient's own care.

use leptos::prelude::*;
use crate::app::{AppState, HomeostasisState};

#[component]
pub fn MetabolismPage() -> impl IntoView {
    let homeostasis = use_context::<RwSignal<HomeostasisState>>()
        .unwrap_or_else(|| RwSignal::new(HomeostasisState::default()));

    let yield_value = move || format!("${:.0}", homeostasis.get().metabolic_yield);

    view! {
        <div class="page metabolism-page">
            <header class="page-header">
                <h1 class="bio-title">"Metabolic Yield"</h1>
                <p class="bio-subtitle">"Energy returning from the research ecosystem"</p>
            </header>

            // Total yield — reactive from AppState
            <section class="yield-hero">
                <div class="yield-value">{yield_value}<span class="yield-unit">" TEND"</span></div>
                <div class="yield-subtitle">"Lifetime metabolic return"</div>
                <div class="yield-flow">
                    "Your data has nourished "
                    <strong>"3 research projects"</strong>
                    " and the energy flows back to sustain your care."
                </div>
            </section>

            // Contribution streams
            <section class="streams">
                <h2>"Active Nutrient Streams"</h2>

                <div class="stream-card">
                    <div class="stream-name">"Type 2 Diabetes Cohort Study"</div>
                    <div class="stream-detail">
                        "7 rounds contributed · 8D glucose gradients · DP protected"
                    </div>
                    <div class="stream-yield">"$342 returned"</div>
                    <div class="stream-bar">
                        <div class="stream-fill" style="width: 58%" />
                    </div>
                </div>

                <div class="stream-card">
                    <div class="stream-name">"Cardiovascular Risk Prediction"</div>
                    <div class="stream-detail">
                        "3 rounds contributed · vital signs + labs"
                    </div>
                    <div class="stream-yield">"$285 returned"</div>
                    <div class="stream-bar">
                        <div class="stream-fill" style="width: 42%" />
                    </div>
                </div>

                <div class="stream-card">
                    <div class="stream-name">"Population Health Atlas"</div>
                    <div class="stream-detail">
                        "12 rounds contributed · demographics + SDOH"
                    </div>
                    <div class="stream-yield">"$220 returned"</div>
                    <div class="stream-bar">
                        <div class="stream-fill" style="width: 35%" />
                    </div>
                </div>
            </section>

            // Preferences
            <section class="yield-preferences">
                <h2>"Yield Allocation"</h2>
                <p>"How should your metabolic return flow?"</p>
                <div class="pref-option active">
                    <span>"Reinvest in care"</span>
                    <span class="pref-pct">"60%"</span>
                </div>
                <div class="pref-option">
                    <span>"Direct payout"</span>
                    <span class="pref-pct">"30%"</span>
                </div>
                <div class="pref-option">
                    <span>"Community health fund"</span>
                    <span class="pref-pct">"10%"</span>
                </div>
            </section>
        </div>
    }
}
