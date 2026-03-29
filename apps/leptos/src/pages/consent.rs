// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Consent — symbiotic connections. Fully reactive — revocations update global state.

use leptos::prelude::*;
use crate::app::AppState;
use crate::zome_clients::consent::ConsentStatus;

/// ID of the consent pending revocation (for confirmation modal).
#[derive(Clone, Debug, PartialEq)]
struct PendingRevocation {
    id: String,
    grantee_name: String,
}

#[component]
pub fn ConsentPage() -> impl IntoView {
    let app = use_context::<AppState>().expect("AppState");

    let active_consents = move || {
        app.consents.get().iter()
            .filter(|c| c.status == ConsentStatus::Active)
            .cloned()
            .collect::<Vec<_>>()
    };

    let revoked_consents = move || {
        app.consents.get().iter()
            .filter(|c| c.status == ConsentStatus::Revoked)
            .cloned()
            .collect::<Vec<_>>()
    };

    let active_count = move || active_consents().len();
    let pending_revoke: RwSignal<Option<PendingRevocation>> = RwSignal::new(None);

    view! {
        <div class="page consent-page">
            <header class="page-header">
                <h1 class="bio-title">"Symbiosis"</h1>
                <p class="bio-subtitle">
                    {move || format!("{} active connections", active_count())}
                </p>
            </header>

            // Active consents
            <section>
                <h2 class="section-title">"Active Connections"</h2>
                <For
                    each=active_consents
                    key=|c| c.id.clone()
                    let:consent
                >
                    {
                        let consent_id = consent.id.clone();
                        let is_sensitive = consent.is_sensitive;
                        let no_redisclosure = consent.no_further_disclosure;

                        view! {
                            <div class=move || {
                                let mut cls = "consent-card active".to_string();
                                if is_sensitive { cls.push_str(" sensitive"); }
                                cls
                            }>
                                <Show when=move || is_sensitive>
                                    <div class="consent-badge">"42 CFR Part 2 Protected"</div>
                                </Show>

                                <div class="consent-who">{consent.grantee_name.clone()}</div>
                                <div class="consent-what">{consent.categories.join(", ")}</div>
                                <div class="consent-why">
                                    "Purpose: "{consent.purpose.clone()}
                                </div>
                                <div class="consent-until">
                                    {match &consent.expires_at {
                                        Some(d) => format!("Until {}", d),
                                        None => "Until you revoke".into(),
                                    }}
                                </div>

                                <Show when=move || no_redisclosure>
                                    <div class="consent-redisclosure">
                                        "This provider cannot share your data with anyone else."
                                    </div>
                                </Show>

                                <button
                                    class="consent-revoke"
                                    on:click={
                                        let id = consent_id.clone();
                                        let name = consent.grantee_name.clone();
                                        move |_| {
                                            pending_revoke.set(Some(PendingRevocation {
                                                id: id.clone(),
                                                grantee_name: name.clone(),
                                            }));
                                        }
                                    }
                                >
                                    "Sever Connection"
                                </button>
                            </div>
                        }
                    }
                </For>

                <Show when=move || active_consents().is_empty()>
                    <div class="empty-state">
                        <p>"No active connections. Your data is fully sovereign."</p>
                    </div>
                </Show>
            </section>

            // Revoked
            <Show when=move || !revoked_consents().is_empty()>
                <section>
                    <h2 class="section-title">"Severed Connections"</h2>
                    <For
                        each=revoked_consents
                        key=|c| c.id.clone()
                        let:consent
                    >
                        <div class="consent-card revoked">
                            <div class="consent-who">{consent.grantee_name.clone()}</div>
                            <div class="consent-what">{consent.categories.join(", ")}</div>
                            <div class="consent-status">"Connection severed"</div>
                        </div>
                    </For>
                </section>
            </Show>

            <button class="consent-new">"Form New Symbiotic Link"</button>

            // Confirmation modal
            <Show when=move || pending_revoke.get().is_some()>
                <div class="modal-overlay" on:click=move |_| pending_revoke.set(None)>
                    <div class="modal-card" on:click=|ev| ev.stop_propagation()>
                        <h3>"Sever This Connection?"</h3>
                        <p class="modal-warning">
                            {move || {
                                let pr = pending_revoke.get().unwrap();
                                format!(
                                    "{} will immediately lose access to your data. \
                                     Any copies they already downloaded are not affected.",
                                    pr.grantee_name
                                )
                            }}
                        </p>
                        <div class="modal-actions">
                            <button
                                class="modal-cancel"
                                on:click=move |_| pending_revoke.set(None)
                            >
                                "Keep Connection"
                            </button>
                            <button
                                class="modal-confirm"
                                on:click=move |_| {
                                    if let Some(pr) = pending_revoke.get() {
                                        app.consents.update(|consents| {
                                            if let Some(c) = consents.iter_mut().find(|c| c.id == pr.id) {
                                                c.status = ConsentStatus::Revoked;
                                            }
                                        });
                                        app.access_events.update(|events| {
                                            events.insert(0, crate::zome_clients::records::AccessEvent {
                                                who: "You".into(),
                                                what: format!("severed connection with {}", pr.grantee_name),
                                                when: "Just now".into(),
                                                event_type: crate::zome_clients::records::AccessEventType::ConsentChange,
                                            });
                                        });
                                        pending_revoke.set(None);
                                    }
                                }
                            >
                                "Sever Connection"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
