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
                let check_active = move |href: &str, path: &str| -> bool {
                    if href == "/" { path == "/" }
                    else { path == href || path.starts_with(&format!("{}/", href)) }
                };
                let is_active = {
                    let h = href_str.clone();
                    move || check_active(&h, &location.pathname.get())
                };
                let is_active2 = {
                    let h = href_str2.clone();
                    move || check_active(&h, &location.pathname.get())
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
