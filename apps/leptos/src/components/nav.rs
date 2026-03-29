// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bottom navigation — five biological sections.

use leptos::prelude::*;
use leptos_router::hooks::use_location;

#[component]
pub fn BottomNav() -> impl IntoView {
    let location = use_location();

    let tabs = vec![
        ("/", "Home", "Homeostasis"),
        ("/records", "Records", "Tissue"),
        ("/consent", "Consent", "Symbiosis"),
        ("/privacy", "Privacy", "Membrane"),
        ("/metabolism", "Metabolism", "Yield"),
    ];

    view! {
        <nav class="bottom-nav" role="tablist" aria-label="Main navigation">
            {tabs.into_iter().map(|(href, label, bio_label)| {
                let href_str = href.to_string();
                let href_str2 = href_str.clone();
                let is_active = move || {
                    let path = location.pathname.get();
                    if href_str == "/" { path == "/" } else { path.starts_with(&href_str) }
                };
                let is_active2 = {
                    let href_str = href_str2.clone();
                    move || {
                        let path = location.pathname.get();
                        if href_str == "/" { path == "/" } else { path.starts_with(&href_str) }
                    }
                };
                view! {
                    <a
                        href=href
                        class=move || if is_active() { "nav-tab active" } else { "nav-tab" }
                        role="tab"
                        aria-selected=move || is_active2().to_string()
                        aria-label=label
                    >
                        <span class="nav-bio-label">{bio_label}</span>
                        <span class="nav-label">{label}</span>
                    </a>
                }
            }).collect::<Vec<_>>()}
        </nav>
    }
}
