// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Consent creation wizard — three steps: Who, What, How Long.

use leptos::prelude::*;
use crate::app::AppState;
use crate::zome_clients::consent::{ConsentSummary, ConsentStatus};

#[derive(Clone, Copy, PartialEq)]
enum WizardStep {
    Who,
    What,
    Duration,
}

/// Data categories with sensitivity levels.
const STANDARD_CATEGORIES: &[&str] = &[
    "Demographics", "Allergies", "Medications", "Vital Signs", "Immunizations",
];
const SENSITIVE_CATEGORIES: &[&str] = &[
    "Mental Health", "Substance Abuse Treatment", "Sexual Health", "Genetic Data",
];

#[component]
pub fn ConsentWizard(
    on_complete: Box<dyn Fn(ConsentSummary) + 'static>,
    on_cancel: Box<dyn Fn() + 'static>,
) -> impl IntoView {
    let step = RwSignal::new(WizardStep::Who);

    // Who
    let provider_name = RwSignal::new(String::new());

    // What
    let selected_categories: RwSignal<Vec<String>> = RwSignal::new(
        STANDARD_CATEGORIES.iter().map(|s| s.to_string()).collect()
    );
    let purpose = RwSignal::new("Treatment".to_string());
    let no_redisclosure = RwSignal::new(true);

    // Duration
    let duration = RwSignal::new("1year".to_string());

    let error = RwSignal::new(Option::<String>::None);

    let on_complete = std::rc::Rc::new(on_complete);
    let on_cancel = std::rc::Rc::new(on_cancel);

    let go_next = move |_| {
        match step.get() {
            WizardStep::Who => {
                if provider_name.get().trim().is_empty() {
                    error.set(Some("Enter a provider or organization name.".into()));
                    return;
                }
                error.set(None);
                step.set(WizardStep::What);
            },
            WizardStep::What => {
                if selected_categories.get().is_empty() {
                    error.set(Some("Select at least one data category.".into()));
                    return;
                }
                error.set(None);
                step.set(WizardStep::Duration);
            },
            WizardStep::Duration => {
                // Complete — build the consent
                let expires = match duration.get().as_str() {
                    "30d" => Some("30 days from now".to_string()),
                    "90d" => Some("90 days from now".to_string()),
                    "1year" => Some("1 year from now".to_string()),
                    _ => None,
                };
                let has_sensitive = selected_categories.get().iter().any(|c| {
                    SENSITIVE_CATEGORIES.contains(&c.as_str())
                });

                let consent = ConsentSummary {
                    id: format!("c-{}", js_sys::Date::now() as u64),
                    grantee_name: provider_name.get(),
                    categories: selected_categories.get(),
                    purpose: purpose.get(),
                    status: ConsentStatus::Active,
                    granted_at: "Just now".into(),
                    expires_at: expires,
                    is_sensitive: has_sensitive,
                    no_further_disclosure: no_redisclosure.get(),
                };
                (on_complete)(consent);
            },
        }
    };

    let go_back = {
        let on_cancel = on_cancel.clone();
        move |_| {
            match step.get() {
                WizardStep::Who => (on_cancel)(),
                WizardStep::What => step.set(WizardStep::Who),
                WizardStep::Duration => step.set(WizardStep::What),
            }
        }
    };

    let step_label = move || match step.get() {
        WizardStep::Who => "Step 1 of 3 — Who",
        WizardStep::What => "Step 2 of 3 — What",
        WizardStep::Duration => "Step 3 of 3 — How Long",
    };

    view! {
        <div class="modal-overlay">
            <div class="wizard-card" on:click=|ev| ev.stop_propagation()>
                <div class="wizard-step-indicator">{step_label}</div>

                // Step 1: Who
                <Show when=move || step.get() == WizardStep::Who>
                    <h2>"Who are you connecting with?"</h2>
                    <div class="verify-field">
                        <label for="provider-name" class="verify-label">"Provider or organization"</label>
                        <input
                            id="provider-name"
                            type="text"
                            class="verify-input"
                            placeholder="e.g., Dr. Sarah Chen, Valley Medical"
                            prop:value=move || provider_name.get()
                            on:input=move |ev| provider_name.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="verify-field">
                        <label for="purpose" class="verify-label">"Purpose"</label>
                        <select
                            id="purpose"
                            class="verify-input"
                            on:change=move |ev| purpose.set(event_target_value(&ev))
                        >
                            <option value="Treatment" selected>"Treatment"</option>
                            <option value="Payment">"Payment / Insurance"</option>
                            <option value="Research">"Research"</option>
                            <option value="Other">"Other"</option>
                        </select>
                    </div>
                </Show>

                // Step 2: What
                <Show when=move || step.get() == WizardStep::What>
                    <h2>"What data can they see?"</h2>

                    <div class="category-group">
                        <div class="category-group-label">"Standard"</div>
                        {STANDARD_CATEGORIES.iter().map(|&cat| {
                            let cat_str = cat.to_string();
                            let cat_str2 = cat_str.clone();
                            let is_checked = move || selected_categories.get().contains(&cat_str);
                            view! {
                                <label class="category-toggle">
                                    <input
                                        type="checkbox"
                                        checked=is_checked
                                        on:change={
                                            let c = cat_str2.clone();
                                            move |_| {
                                                selected_categories.update(|cats| {
                                                    if cats.contains(&c) {
                                                        cats.retain(|x| x != &c);
                                                    } else {
                                                        cats.push(c.clone());
                                                    }
                                                });
                                            }
                                        }
                                    />
                                    <span>{cat}</span>
                                </label>
                            }
                        }).collect::<Vec<_>>()}
                    </div>

                    <div class="category-group sensitive-group">
                        <div class="category-group-label">"Sensitive (requires explicit consent)"</div>
                        {SENSITIVE_CATEGORIES.iter().map(|&cat| {
                            let cat_str = cat.to_string();
                            let cat_str2 = cat_str.clone();
                            let is_checked = move || selected_categories.get().contains(&cat_str);
                            view! {
                                <label class="category-toggle sensitive">
                                    <input
                                        type="checkbox"
                                        checked=is_checked
                                        on:change={
                                            let c = cat_str2.clone();
                                            move |_| {
                                                selected_categories.update(|cats| {
                                                    if cats.contains(&c) {
                                                        cats.retain(|x| x != &c);
                                                    } else {
                                                        cats.push(c.clone());
                                                    }
                                                });
                                            }
                                        }
                                    />
                                    <span>{cat}</span>
                                </label>
                            }
                        }).collect::<Vec<_>>()}
                    </div>

                    <label class="category-toggle redisclosure">
                        <input
                            type="checkbox"
                            checked=move || no_redisclosure.get()
                            on:change=move |_| no_redisclosure.update(|v| *v = !*v)
                        />
                        <span>"Prevent re-disclosure (they cannot share your data further)"</span>
                    </label>
                </Show>

                // Step 3: Duration
                <Show when=move || step.get() == WizardStep::Duration>
                    <h2>"How long does this connection last?"</h2>
                    {["30d", "90d", "1year", "forever"].iter().map(|&d| {
                        let label = match d {
                            "30d" => "30 days",
                            "90d" => "90 days",
                            "1year" => "1 year",
                            "forever" => "Until I revoke it",
                            _ => d,
                        };
                        let d_str = d.to_string();
                        let d_str2 = d_str.clone();
                        view! {
                            <label class="duration-option">
                                <input
                                    type="radio"
                                    name="duration"
                                    value=d
                                    checked=move || duration.get() == d_str
                                    on:change={
                                        let v = d_str2.clone();
                                        move |_| duration.set(v.clone())
                                    }
                                />
                                <span>{label}</span>
                            </label>
                        }
                    }).collect::<Vec<_>>()}

                    // Summary
                    <div class="wizard-summary">
                        <strong>{move || provider_name.get()}</strong>
                        " can access "
                        <strong>{move || {
                            let cats = selected_categories.get();
                            if cats.len() > 3 {
                                format!("{} categories", cats.len())
                            } else {
                                cats.join(", ")
                            }
                        }}</strong>
                        " for "
                        <strong>{move || purpose.get()}</strong>
                        "."
                    </div>
                </Show>

                // Error
                <Show when=move || error.get().is_some()>
                    <div class="verify-error">{move || error.get().unwrap_or_default()}</div>
                </Show>

                // Navigation
                <div class="wizard-nav">
                    <button class="modal-cancel" on:click=go_back>
                        {move || if step.get() == WizardStep::Who { "Cancel" } else { "Back" }}
                    </button>
                    <button class="onboarding-cta" on:click=go_next>
                        {move || if step.get() == WizardStep::Duration { "Form Connection" } else { "Next" }}
                    </button>
                </div>
            </div>
        </div>
    }
}
