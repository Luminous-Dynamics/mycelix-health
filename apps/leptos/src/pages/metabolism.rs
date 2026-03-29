// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Metabolism — data dividends as metabolic yield. Fully reactive.

use leptos::prelude::*;
use crate::app::{AppState, HomeostasisState};
use crate::zome_clients::records::AccessEventType;

#[component]
pub fn MetabolismPage() -> impl IntoView {
    let homeostasis = use_context::<RwSignal<HomeostasisState>>()
        .unwrap_or_else(|| RwSignal::new(HomeostasisState::default()));
    let app = use_context::<AppState>().expect("AppState");

    let yield_value = move || format!("${:.0}", homeostasis.get().metabolic_yield);

    // Yield allocation preferences (reactive)
    let care_pct = RwSignal::new(60u32);
    let payout_pct = RwSignal::new(30u32);
    let community_pct = move || 100 - care_pct.get() - payout_pct.get();

    // Contribution streams (mock — would come from dividends zome)
    let streams = vec![
        ("Type 2 Diabetes Cohort Study", "7 rounds · glucose gradients · DP protected", 342.0, 58),
        ("Cardiovascular Risk Prediction", "3 rounds · vital signs + labs", 285.0, 42),
        ("Population Health Atlas", "12 rounds · demographics + SDOH", 220.0, 35),
    ];

    view! {
        <div class="page metabolism-page">
            <header class="page-header">
                <h1 class="bio-title">"Metabolic Yield"</h1>
                <p class="bio-subtitle">"Energy returning from the research ecosystem"</p>
            </header>

            // Total yield — reactive
            <section class="yield-hero">
                <div class="yield-value">{yield_value}<span class="yield-unit">" TEND"</span></div>
                <div class="yield-subtitle">"Lifetime metabolic return"</div>
                <div class="yield-flow">
                    "Your data has nourished "
                    <strong>{streams.len().to_string()}" research projects"</strong>
                    " and the energy flows back to sustain your care."
                </div>
            </section>

            // Contribution streams
            <section class="streams">
                <h2>"Active Nutrient Streams"</h2>
                {streams.into_iter().map(|(name, detail, yield_amt, pct)| {
                    view! {
                        <div class="stream-card">
                            <div class="stream-name">{name}</div>
                            <div class="stream-detail">{detail}</div>
                            <div class="stream-yield">{format!("${:.0} returned", yield_amt)}</div>
                            <div class="stream-bar">
                                <div class="stream-fill" style=format!("width: {}%", pct) />
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </section>

            // Yield allocation — interactive sliders
            <section class="yield-preferences">
                <h2>"Yield Allocation"</h2>
                <p class="yield-pref-desc">"How should your metabolic return flow?"</p>

                <div class="pref-slider">
                    <div class="pref-header">
                        <span class="pref-name">"Reinvest in care"</span>
                        <span class="pref-pct">{move || format!("{}%", care_pct.get())}</span>
                    </div>
                    <input
                        type="range"
                        min="0" max="100"
                        class="slider"
                        prop:value=move || care_pct.get().to_string()
                        on:input=move |ev| {
                            let val: u32 = event_target_value(&ev).parse().unwrap_or(60);
                            let remaining = 100u32.saturating_sub(val);
                            care_pct.set(val.min(100));
                            if payout_pct.get() > remaining {
                                payout_pct.set(remaining);
                            }
                        }
                    />
                </div>

                <div class="pref-slider">
                    <div class="pref-header">
                        <span class="pref-name">"Direct payout"</span>
                        <span class="pref-pct">{move || format!("{}%", payout_pct.get())}</span>
                    </div>
                    <input
                        type="range"
                        min="0"
                        max=move || (100 - care_pct.get()).to_string()
                        class="slider"
                        prop:value=move || payout_pct.get().to_string()
                        on:input=move |ev| {
                            let val: u32 = event_target_value(&ev).parse().unwrap_or(30);
                            let max = 100 - care_pct.get();
                            payout_pct.set(val.min(max));
                        }
                    />
                </div>

                <div class="pref-slider">
                    <div class="pref-header">
                        <span class="pref-name">"Community health fund"</span>
                        <span class="pref-pct">{move || format!("{}%", community_pct())}</span>
                    </div>
                    <div class="pref-auto">"Auto-calculated from remaining allocation"</div>
                </div>

                // Visual breakdown
                <div class="allocation-bar">
                    <div class="alloc-care" style=move || format!("width: {}%", care_pct.get()) />
                    <div class="alloc-payout" style=move || format!("width: {}%", payout_pct.get()) />
                    <div class="alloc-community" style=move || format!("width: {}%", community_pct()) />
                </div>
                <div class="alloc-legend">
                    <span class="legend-item"><span class="legend-dot care" />"Care"</span>
                    <span class="legend-item"><span class="legend-dot payout" />"Payout"</span>
                    <span class="legend-item"><span class="legend-dot community" />"Community"</span>
                </div>
            </section>
        </div>
    }
}
