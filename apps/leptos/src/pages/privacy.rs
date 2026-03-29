// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Privacy — the membrane. Reactive budget gauge and access timeline.

use leptos::prelude::*;
use crate::app::AppState;
use crate::zome_clients::records::AccessEventType;

/// Simulated FL round result for the UI.
#[derive(Clone, Debug)]
struct FlRoundResult {
    loinc_family: String,
    cohort_size: usize,
    excluded: usize,
    interpretation: String,
    quality_pct: u32,
}

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

            // FL Research Projects
            <section class="fl-section">
                <h2>"Federated Learning"</h2>
                <p class="fl-description">
                    "Your data contributes to research without leaving your device. "
                    "Only statistical gradients are shared — raw values stay here."
                </p>

                <FlProjectCard
                    name="Type 2 Diabetes Cohort"
                    loinc="Glucose (2345-7)"
                    rounds_completed=7
                    total_rounds=12
                    app=app.clone()
                />
                <FlProjectCard
                    name="Cardiovascular Risk"
                    loinc="Cholesterol (2093-3)"
                    rounds_completed=3
                    total_rounds=10
                    app=app.clone()
                />
            </section>

            // Data export
            <section class="export-section">
                <h2>"Data Portability"</h2>
                <button class="export-btn" on:click=move |_| {
                    // Generate a JSON export of the patient's data
                    let records = app.records.get();
                    let consents = app.consents.get();
                    let export = serde_json::json!({
                        "export_version": "1.0",
                        "export_date": "2026-03-29",
                        "records_count": records.len(),
                        "consents_count": consents.len(),
                        "privacy_budget_remaining": app.privacy_budget.get(),
                        "note": "Full FHIR R4 export requires conductor connection"
                    });
                    let text = serde_json::to_string_pretty(&export).unwrap_or_default();
                    web_sys::console::log_1(&format!("Export:\n{}", text).into());
                    // In production: trigger download via Blob URL
                    let window = web_sys::window().unwrap();
                    let _ = window.alert_with_message(&format!(
                        "Export ready ({} records, {} consents). Check browser console for data.",
                        records.len(), consents.len()
                    ));
                }>
                    "Export My Health Data"
                </button>
                <p class="export-note">
                    "HIPAA Right to Access (45 CFR 164.524) — you can export your data at any time."
                </p>
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

/// FL Project card — shows a federated learning research project.
#[component]
fn FlProjectCard(
    name: &'static str,
    loinc: &'static str,
    rounds_completed: u32,
    total_rounds: u32,
    app: AppState,
) -> impl IntoView {
    let contributed = RwSignal::new(false);
    let result: RwSignal<Option<FlRoundResult>> = RwSignal::new(None);
    let progress_pct = (rounds_completed as f64 / total_rounds as f64 * 100.0) as u32;

    let contribute = move |_| {
        let budget = app.privacy_budget.get();
        if budget < 1.0 { return; }

        app.privacy_budget.set(budget - 1.0);
        contributed.set(true);

        // Simulate FL round result
        result.set(Some(FlRoundResult {
            loinc_family: loinc.chars().take(4).collect(),
            cohort_size: 6,
            excluded: 0,
            interpretation: format!("{}: cohort values within normal range", loinc),
            quality_pct: 100,
        }));

        app.access_events.update(|events| {
            events.insert(0, crate::zome_clients::records::AccessEvent {
                who: "Federated Learning".into(),
                what: format!("contributed to {} (ε=1.0)", name),
                when: "Just now".into(),
                event_type: AccessEventType::FlContribution,
            });
        });
    };

    view! {
        <div class="fl-project-card">
            <div class="fl-project-header">
                <span class="fl-project-name">{name}</span>
                <span class="fl-project-loinc">{loinc}</span>
            </div>
            <div class="fl-progress">
                <div class="fl-progress-bar">
                    <div class="fl-progress-fill" style=format!("width: {}%", progress_pct) />
                </div>
                <span class="fl-progress-label">
                    {format!("{}/{} rounds", rounds_completed, total_rounds)}
                </span>
            </div>

            <Show when=move || result.get().is_some()>
                <div class="fl-result">
                    <span class="fl-result-label">"Latest insight:"</span>
                    <span class="fl-result-text">
                        {move || result.get().map(|r| r.interpretation).unwrap_or_default()}
                    </span>
                    <span class="fl-result-quality">
                        {move || result.get().map(|r| format!("Quality: {}%", r.quality_pct)).unwrap_or_default()}
                    </span>
                </div>
            </Show>

            <Show when=move || !contributed.get()>
                <button
                    class="fl-contribute-btn"
                    on:click=contribute
                    disabled=move || app.privacy_budget.get() < 1.0
                >
                    "Contribute Gradient (ε=1.0)"
                </button>
            </Show>
            <Show when=move || contributed.get()>
                <div class="fl-contributed">"Contributed this session"</div>
            </Show>
        </div>
    }
}
