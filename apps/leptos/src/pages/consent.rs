// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Consent — symbiotic connections to providers.

use leptos::prelude::*;

#[component]
pub fn ConsentPage() -> impl IntoView {
    view! {
        <div class="page consent-page">
            <header class="page-header">
                <h1 class="bio-title">"Symbiosis"</h1>
                <p class="bio-subtitle">"Your connections — who can access your tissue"</p>
            </header>

            <div class="consent-list">
                <div class="consent-card active">
                    <div class="consent-who">"Dr. Sarah Chen"</div>
                    <div class="consent-what">"Lab Results, Medications, Vital Signs"</div>
                    <div class="consent-why">"Treatment"</div>
                    <div class="consent-until">"Until December 2026"</div>
                    <button class="consent-revoke">"Sever Connection"</button>
                </div>

                <div class="consent-card active sensitive">
                    <div class="consent-badge">"42 CFR Part 2 Protected"</div>
                    <div class="consent-who">"Dr. James Park"</div>
                    <div class="consent-what">"Substance Abuse Treatment"</div>
                    <div class="consent-why">"Treatment"</div>
                    <div class="consent-redisclosure">
                        "This provider cannot share your data with anyone else. "
                        "Federal law prevents re-disclosure."
                    </div>
                    <button class="consent-revoke">"Sever Connection"</button>
                </div>

                <div class="consent-card revoked">
                    <div class="consent-who">"Valley Urgent Care"</div>
                    <div class="consent-what">"Demographics, Allergies"</div>
                    <div class="consent-status">"Connection severed March 1, 2026"</div>
                </div>
            </div>

            <button class="consent-new">"Form New Symbiotic Link"</button>
        </div>
    }
}
