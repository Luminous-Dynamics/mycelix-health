// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Privacy — the membrane. Reactive budget gauge and access timeline.

use leptos::prelude::*;
use crate::app::AppState;
use crate::zome_clients::records::AccessEventType;

#[component]
pub fn PrivacyPage() -> impl IntoView {
    let app = use_context::<AppState>().expect("AppState");

    let budget_pct = move || {
        let max = app.privacy_budget_max.get();
        if max > 0.0 {
            (app.privacy_budget.get() / max * 100.0) as u32
        } else {
            0
        }
    };

    let budget_remaining = move || app.privacy_budget.get();
    let contributions_remaining = move || (app.privacy_budget.get() / 1.0) as u32;

    // Circumference of circle r=50: 2π×50 = 314.16
    let circumference = 314.16_f64;
    let gauge_stroke = move || {
        let pct = budget_pct() as f64 / 100.0;
        format!("stroke-dasharray: {:.1} {:.1};", circumference * pct, circumference)
    };

    let gauge_color_class = move || {
        let pct = budget_pct();
        if pct > 50 { "gauge-healthy" }
        else if pct > 25 { "gauge-caution" }
        else if pct > 10 { "gauge-warning" }
        else { "gauge-critical" }
    };

    let events = move || app.access_events.get();

    view! {
        <div class="page privacy-page">
            <header class="page-header">
                <h1 class="bio-title">"Membrane"</h1>
                <p class="bio-subtitle">"Your privacy boundary"</p>
            </header>

            // Privacy budget gauge
            <section class="budget-section">
                <div class="budget-gauge"
                     role="meter"
                     aria-valuemin="0"
                     aria-valuemax="100"
                     aria-valuenow=budget_pct
                     aria-label="Privacy budget remaining">
                    <svg viewBox="0 0 120 120" class="gauge-svg">
                        <circle cx="60" cy="60" r="50" class="gauge-bg" />
                        <circle cx="60" cy="60" r="50"
                            class=move || format!("gauge-fill {}", gauge_color_class())
                            style=gauge_stroke
                        />
                    </svg>
                    <div class="gauge-label">
                        <span class="gauge-value">{move || format!("{:.1}", budget_remaining())}"ε"</span>
                        <span class="gauge-text">"remaining"</span>
                    </div>
                </div>
                <p class="budget-explanation">
                    "Your privacy membrane has capacity for approximately "
                    <strong>{contributions_remaining}" more"</strong>
                    " research contributions before renewal."
                </p>

                // Simulate FL contribution button
                <button
                    class="fl-contribute-btn"
                    on:click=move |_| {
                        let current = app.privacy_budget.get();
                        if current >= 1.0 {
                            app.privacy_budget.set(current - 1.0);
                            app.access_events.update(|events| {
                                events.insert(0, crate::zome_clients::records::AccessEvent {
                                    who: "Federated Learning".into(),
                                    what: "gradient contributed (ε=1.0)".into(),
                                    when: "Just now".into(),
                                    event_type: AccessEventType::FlContribution,
                                });
                            });
                        }
                    }
                    disabled=move || app.privacy_budget.get() < 1.0
                >
                    "Contribute to Research (ε=1.0)"
                </button>
            </section>

            // Access log timeline
            <section class="access-timeline">
                <h2>"Membrane Crossings"</h2>
                <div class="timeline">
                    <For
                        each=events
                        key=|e| format!("{}-{}-{}", e.who, e.what, e.when)
                        let:event
                    >
                        <div class=move || match event.event_type {
                            AccessEventType::FlContribution => "timeline-entry fl",
                            AccessEventType::BreakGlass => "timeline-entry break-glass",
                            _ => "timeline-entry",
                        }>
                            <div class=move || match event.event_type {
                                AccessEventType::DividendPayout => "timeline-dot metabolic",
                                AccessEventType::FlContribution => "timeline-dot fl",
                                AccessEventType::BreakGlass => "timeline-dot break-glass",
                                _ => "timeline-dot",
                            } />
                            <div class="timeline-content">
                                <span class="timeline-who">{event.who.clone()}</span>
                                <span class="timeline-what">" "{event.what.clone()}</span>
                            </div>
                            <span class="timeline-when">{event.when.clone()}</span>
                        </div>
                    </For>
                </div>
            </section>
        </div>
    }
}
