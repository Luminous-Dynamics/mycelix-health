// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! App shell — router, providers, biological background layer.

use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::components::homeostasis_bg::HomeostasisBackground;
use crate::components::nav::BottomNav;
use crate::crypto::key_manager;
use crate::pages;

/// Biological health state — drives the homeostasis background.
/// 0.0 = critical instability, 1.0 = perfect homeostatic alignment.
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
            metabolic_yield: 0.0,
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    let homeostasis = RwSignal::new(HomeostasisState::default());
    provide_context(homeostasis);

    view! {
        <div class="portal-root">
            // Living background — driven by patient's homeostatic state
            <HomeostasisBackground />

            // Content layer
            <div class="portal-content">
                <Router>
                    <main class="portal-main">
                        <Routes fallback=|| view! { <p class="not-found">"404 — This pathway does not exist."</p> }>
                            <Route path=path!("/") view=pages::home::HomePage />
                            <Route path=path!("/records") view=pages::records::RecordsPage />
                            <Route path=path!("/consent") view=pages::consent::ConsentPage />
                            <Route path=path!("/privacy") view=pages::privacy::PrivacyPage />
                            <Route path=path!("/metabolism") view=pages::metabolism::MetabolismPage />
                            <Route path=path!("/welcome") view=pages::onboarding::OnboardingPage />
                        </Routes>
                    </main>
                    <BottomNav />
                </Router>
            </div>
        </div>
    }
}
