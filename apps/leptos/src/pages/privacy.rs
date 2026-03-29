// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Privacy — the membrane. Who passed through, and how much capacity remains.

use leptos::prelude::*;

#[component]
pub fn PrivacyPage() -> impl IntoView {
    let budget_remaining = 7.2_f64;
    let budget_max = 10.0_f64;
    let budget_pct = (budget_remaining / budget_max * 100.0) as u32;

    view! {
        <div class="page privacy-page">
            <header class="page-header">
                <h1 class="bio-title">"Membrane"</h1>
                <p class="bio-subtitle">"Your privacy boundary — what passes through"</p>
            </header>

            // Privacy budget as biological fuel gauge
            <section class="budget-section">
                <div class="budget-gauge"
                     role="meter"
                     aria-valuemin="0"
                     aria-valuemax="100"
                     aria-valuenow=budget_pct
                     aria-label="Privacy budget remaining">
                    <svg viewBox="0 0 120 120" class="gauge-svg">
                        // Background ring
                        <circle cx="60" cy="60" r="50" class="gauge-bg" />
                        // Filled arc
                        <circle cx="60" cy="60" r="50" class="gauge-fill"
                            style=format!(
                                "stroke-dasharray: {} {};",
                                314.0 * budget_remaining / budget_max,
                                314.0
                            )
                        />
                    </svg>
                    <div class="gauge-label">
                        <span class="gauge-value">{format!("{:.1}", budget_remaining)}"ε"</span>
                        <span class="gauge-text">"remaining"</span>
                    </div>
                </div>
                <p class="budget-explanation">
                    "Your privacy membrane has capacity for approximately "
                    <strong>{((budget_remaining / 1.0) as u32).to_string()}" more"</strong>
                    " research contributions before renewal."
                </p>
            </section>

            // Access log timeline
            <section class="access-timeline">
                <h2>"Membrane Crossings"</h2>
                <div class="timeline">
                    <div class="timeline-entry">
                        <div class="timeline-dot" />
                        <div class="timeline-content">
                            <span class="timeline-who">"Dr. Chen"</span>
                            <span class="timeline-what">" viewed Lab Results"</span>
                            <span class="timeline-when">"Today, 2:14 PM"</span>
                        </div>
                    </div>
                    <div class="timeline-entry fl">
                        <div class="timeline-dot fl" />
                        <div class="timeline-content">
                            <span class="timeline-who">"Federated Learning"</span>
                            <span class="timeline-what">" — gradient extracted (ε=1.0)"</span>
                            <span class="timeline-when">"Yesterday"</span>
                        </div>
                    </div>
                </div>
            </section>
        </div>
    }
}
