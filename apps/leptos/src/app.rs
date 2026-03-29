// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! App shell — router, providers, biological background layer.

use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::components::homeostasis_bg::HomeostasisBackground;
use crate::components::nav::BottomNav;
use crate::crypto::key_manager;
use crate::holochain::{HolochainProvider, ConnectionBadge};
use crate::pages;
use crate::zome_clients::consent::{mock_consents, ConsentSummary, ConsentStatus};
use crate::zome_clients::records::{mock_records, mock_access_events, HealthRecord, AccessEvent};

/// Biological health state — drives the homeostasis background.
#[derive(Clone, Copy, Debug)]
pub struct HomeostasisState {
    pub alignment: f64,
    pub phi: f64,
    pub metabolic_yield: f64,
}

impl Default for HomeostasisState {
    fn default() -> Self {
        Self {
            alignment: 0.85,
            phi: 0.5,
            metabolic_yield: 847.0,
        }
    }
}

/// Vault state — whether the patient's encryption key is available.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VaultState {
    /// No vault exists — needs onboarding.
    NoVault,
    /// Vault exists but locked (needs passphrase).
    Locked,
    /// Vault is unlocked — encryption/decryption available.
    Unlocked,
}

/// Global app state — shared across all pages via context.
#[derive(Clone, Debug)]
pub struct AppState {
    pub vault: RwSignal<VaultState>,
    pub consents: RwSignal<Vec<ConsentSummary>>,
    pub records: RwSignal<Vec<HealthRecord>>,
    pub access_events: RwSignal<Vec<AccessEvent>>,
    pub privacy_budget: RwSignal<f64>,
    pub privacy_budget_max: RwSignal<f64>,
}

#[component]
pub fn App() -> impl IntoView {
    let homeostasis = RwSignal::new(HomeostasisState::default());
    provide_context(homeostasis);

    // Initialize vault state
    let vault_state = if key_manager::has_stored_key() {
        VaultState::Locked
    } else {
        VaultState::NoVault
    };

    // Global reactive app state
    let app_state = AppState {
        vault: RwSignal::new(vault_state),
        consents: RwSignal::new(mock_consents()),
        records: RwSignal::new(mock_records()),
        access_events: RwSignal::new(mock_access_events()),
        privacy_budget: RwSignal::new(7.2),
        privacy_budget_max: RwSignal::new(10.0),
    };
    provide_context(app_state);

    // Update homeostasis based on active consent count
    let app = use_context::<AppState>().unwrap();
    Effect::new(move |_| {
        let active = app.consents.get().iter()
            .filter(|c| c.status == ConsentStatus::Active)
            .count();
        let encrypted = app.records.get().iter()
            .filter(|r| r.encrypted)
            .count();
        let total = app.records.get().len().max(1);
        let budget_pct = app.privacy_budget.get() / app.privacy_budget_max.get();

        // Alignment is a composite of encryption coverage, consent health, and budget
        let encryption_score = encrypted as f64 / total as f64;
        let consent_score = (active as f64 / 3.0).min(1.0); // Normalize to ~3 expected
        let alignment = (encryption_score * 0.4 + consent_score * 0.3 + budget_pct * 0.3).min(1.0);

        homeostasis.update(|h| {
            h.alignment = alignment;
            h.phi = encryption_score * 0.7; // Phi tracks integration (encryption = integration)
        });
    });

    view! {
        <div class="portal-root">
            // Living background — driven by patient's homeostatic state
            <HomeostasisBackground />

            // Content layer
            <div class="portal-content">
                <HolochainProvider>
                <Router>
                    <main class="portal-main">
                        <Routes fallback=|| view! { <p class="not-found">"404 — This pathway does not exist."</p> }>
                            <Route path=path!("/") view=pages::home::HomePage />
                            <Route path=path!("/records") view=pages::records::RecordsPage />
                            <Route path=path!("/consent") view=pages::consent::ConsentPage />
                            <Route path=path!("/privacy") view=pages::privacy::PrivacyPage />
                            <Route path=path!("/metabolism") view=pages::metabolism::MetabolismPage />
                            <Route path=path!("/welcome") view=pages::onboarding::OnboardingPage />
                            <Route path=path!("/settings") view=pages::settings::SettingsPage />
                        </Routes>
                    </main>
                    <BottomNav />
                    <ConnectionBadge />
                </Router>
                </HolochainProvider>
            </div>
        </div>
    }
}
