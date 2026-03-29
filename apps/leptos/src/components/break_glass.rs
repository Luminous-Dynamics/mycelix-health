// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Break-glass emergency access alert.
//!
//! When a provider invokes emergency access (bypassing normal consent),
//! this alert appears prominently until acknowledged.

use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct BreakGlassEvent {
    pub provider_name: String,
    pub categories: Vec<String>,
    pub reason: String,
    pub when: String,
    pub acknowledged: bool,
}

#[component]
pub fn BreakGlassAlert(
    event: BreakGlassEvent,
    acknowledged: RwSignal<bool>,
) -> impl IntoView {

    view! {
        <Show when=move || !acknowledged.get()>
            <div class="break-glass-alert" role="alert" aria-live="assertive">
                <div class="break-glass-icon">
                    <svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true">
                        <path d="M12 2L2 22h20L12 2z" fill="none" stroke="var(--revoked)" stroke-width="2" />
                        <line x1="12" y1="9" x2="12" y2="14" stroke="var(--revoked)" stroke-width="2" stroke-linecap="round" />
                        <circle cx="12" cy="17" r="1" fill="var(--revoked)" />
                    </svg>
                </div>
                <div class="break-glass-content">
                    <div class="break-glass-title">"Emergency Access Used"</div>
                    <div class="break-glass-detail">
                        <strong>{event.provider_name.clone()}</strong>
                        " accessed your "
                        {event.categories.join(", ")}
                    </div>
                    <div class="break-glass-reason">
                        "Reason: "{event.reason.clone()}
                    </div>
                    <div class="break-glass-time">{event.when.clone()}</div>
                </div>
                <button
                    class="break-glass-ack"
                    on:click=move |_| acknowledged.set(true)
                >
                    "Acknowledge"
                </button>
            </div>
        </Show>
    }
}
