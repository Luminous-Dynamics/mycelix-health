// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Mycelial Node — replaces the padlock icon for encryption indicators.
//!
//! When tapped, a bioluminescent pulse traces the cryptographic pathway
//! from the encrypted record to the agents holding decryption keys.
//! This visualizes the DHT's decentralized nature — sovereignty, not restriction.

use leptos::prelude::*;
use serde::{Serialize, Deserialize};

/// State of a mycelial node's cryptographic connections.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CryptoPathway {
    /// Name of the key holder (e.g., "Dr. Sarah Chen").
    pub holder_name: String,
    /// Whether this pathway is currently active (consent not revoked).
    pub active: bool,
}

/// Mycelial Node — the living encryption indicator.
///
/// Resting state: a softly glowing cyan node.
/// On tap: bioluminescent pulse traces outward to key holders.
/// Active pathways glow emerald. Revoked pathways dim.
#[component]
pub fn MycelialNode(
    encrypted: ReadSignal<bool>,
    pathways: ReadSignal<Vec<CryptoPathway>>,
    #[prop(optional)] category: String,
) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let pulsing = RwSignal::new(false);

    let activate = move || {
        if encrypted.get() {
            pulsing.set(true);
            expanded.update(|e| *e = !*e);
            gloo_timers::callback::Timeout::new(800, move || {
                pulsing.set(false);
            }).forget();
        }
    };

    let on_click = move |_| activate();
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" || ev.key() == " " {
            ev.prevent_default();
            activate();
        }
    };

    let node_class = move || {
        let mut cls = String::from("mycelial-node");
        if encrypted.get() { cls.push_str(" encrypted"); }
        if pulsing.get() { cls.push_str(" pulsing"); }
        if expanded.get() { cls.push_str(" expanded"); }
        cls
    };

    let cat = category;

    view! {
        <div class=node_class on:click=on_click on:keydown=on_keydown role="button" tabindex="0"
             aria-label=format!("Encryption status for {}", cat)>

            // The node itself — SVG mycelial symbol
            <svg class="node-icon" viewBox="0 0 24 24" width="20" height="20"
                 aria-hidden="true">
                // Central node
                <circle cx="12" cy="12" r="3" class="node-core" />
                // Mycelial tendrils radiating outward
                <path d="M12 9 L12 3" class="tendril" />
                <path d="M12 15 L12 21" class="tendril" />
                <path d="M9 12 L3 12" class="tendril" />
                <path d="M15 12 L21 12" class="tendril" />
                <path d="M9.9 9.9 L5.1 5.1" class="tendril" />
                <path d="M14.1 14.1 L18.9 18.9" class="tendril" />
                <path d="M9.9 14.1 L5.1 18.9" class="tendril" />
                <path d="M14.1 9.9 L18.9 5.1" class="tendril" />
                // Outer connection points (key holders)
                {move || pathways.get().iter().enumerate().map(|(i, p)| {
                    let angle = (i as f64) * std::f64::consts::TAU / pathways.get().len().max(1) as f64;
                    let cx = 12.0 + 9.0 * angle.cos();
                    let cy = 12.0 + 9.0 * angle.sin();
                    let class = if p.active { "endpoint active" } else { "endpoint revoked" };
                    view! {
                        <circle cx=cx.to_string() cy=cy.to_string() r="1.5" class=class />
                    }
                }).collect::<Vec<_>>()}
            </svg>

            // Expanded pathway view — shows who holds keys
            <Show when=move || expanded.get()>
                <div class="pathway-panel">
                    <div class="pathway-header">"Cryptographic Pathways"</div>
                    <For
                        each=move || pathways.get()
                        key=|p| p.holder_name.clone()
                        let:pathway
                    >
                        <div class=move || if pathway.active { "pathway active" } else { "pathway revoked" }>
                            <span class="pathway-dot" />
                            <span class="pathway-name">{pathway.holder_name.clone()}</span>
                            <span class="pathway-status">
                                {if pathway.active { "Connected" } else { "Revoked" }}
                            </span>
                        </div>
                    </For>
                    <div class="pathway-footer">
                        "Your data is encrypted on the distributed network. "
                        "Only the connected agents above can read it."
                    </div>
                </div>
            </Show>
        </div>
    }
}
