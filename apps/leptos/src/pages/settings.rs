// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Settings — key management, vault status, accessibility.

use leptos::prelude::*;
use crate::app::{AppState, VaultState};
use crate::crypto::key_manager;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let app = use_context::<AppState>().expect("AppState");

    let has_key = move || key_manager::has_stored_key();
    let vault_status = move || match app.vault.get() {
        VaultState::NoVault => "Not Created",
        VaultState::Locked => "Locked",
        VaultState::Unlocked => "Unlocked",
    };

    // Unlock passphrase input
    let passphrase = RwSignal::new(String::new());
    let unlock_error = RwSignal::new(Option::<String>::None);

    let unlock_vault = move |_| {
        let pass = passphrase.get();
        match key_manager::unwrap_key(&pass) {
            Ok(_key) => {
                app.vault.set(VaultState::Unlocked);
                unlock_error.set(None);
                passphrase.set(String::new());
            },
            Err(e) => {
                unlock_error.set(Some(e));
            },
        }
    };

    let destroy_vault = move |_| {
        let _ = key_manager::destroy_vault();
        app.vault.set(VaultState::NoVault);
    };

    view! {
        <div class="page settings-page">
            <header class="page-header">
                <h1 class="bio-title">"Settings"</h1>
            </header>

            // Vault section
            <section class="settings-section">
                <h2 class="section-title">"Health Vault"</h2>

                <div class="settings-card">
                    <div class="settings-row">
                        <span class="settings-label">"Status"</span>
                        <span class=move || {
                            match app.vault.get() {
                                VaultState::Unlocked => "settings-value vault-unlocked",
                                VaultState::Locked => "settings-value vault-locked",
                                VaultState::NoVault => "settings-value vault-none",
                            }
                        }>{vault_status}</span>
                    </div>
                    <div class="settings-row">
                        <span class="settings-label">"Key Stored"</span>
                        <span class="settings-value">{move || if has_key() { "Yes" } else { "No" }}</span>
                    </div>
                </div>

                // Unlock form (when locked)
                <Show when=move || app.vault.get() == VaultState::Locked>
                    <div class="settings-card">
                        <div class="verify-field">
                            <label for="unlock-pass" class="verify-label">"Passphrase"</label>
                            <input
                                id="unlock-pass"
                                type="password"
                                class="verify-input"
                                placeholder="Enter your passphrase..."
                                autocomplete="current-password"
                                prop:value=move || passphrase.get()
                                on:input=move |ev| passphrase.set(event_target_value(&ev))
                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                    if ev.key() == "Enter" {
                                        let pass = passphrase.get();
                                        match key_manager::unwrap_key(&pass) {
                                            Ok(_key) => {
                                                app.vault.set(VaultState::Unlocked);
                                                unlock_error.set(None);
                                                passphrase.set(String::new());
                                            },
                                            Err(e) => unlock_error.set(Some(e)),
                                        }
                                    }
                                }
                            />
                        </div>
                        <Show when=move || unlock_error.get().is_some()>
                            <div class="verify-error">{move || unlock_error.get().unwrap_or_default()}</div>
                        </Show>
                        <button class="onboarding-cta" on:click=unlock_vault>"Unlock Vault"</button>
                    </div>
                </Show>

                // No vault
                <Show when=move || app.vault.get() == VaultState::NoVault>
                    <a href="/welcome" class="vault-warning">"Create your health vault"</a>
                </Show>

                // Destroy option (when vault exists)
                <Show when=move || app.vault.get() != VaultState::NoVault>
                    <details class="danger-section">
                        <summary class="danger-summary">"Danger Zone"</summary>
                        <p class="danger-text">
                            "Destroying your vault permanently erases your encryption key from this device. "
                            "All encrypted records become unreadable unless you have your recovery phrase."
                        </p>
                        <button class="danger-btn" on:click=destroy_vault>"Destroy Vault"</button>
                    </details>
                </Show>
            </section>

            // Accessibility section
            <section class="settings-section">
                <h2 class="section-title">"Accessibility"</h2>
                <div class="settings-card">
                    <div class="settings-row">
                        <span class="settings-label">"Reading Level"</span>
                        <span class="settings-value">"Standard"</span>
                    </div>
                    <div class="settings-row">
                        <span class="settings-label">"High Contrast"</span>
                        <span class="settings-value">"Off"</span>
                    </div>
                    <div class="settings-row">
                        <span class="settings-label">"Reduce Motion"</span>
                        <span class="settings-value">"System Default"</span>
                    </div>
                </div>
            </section>

            // About
            <section class="settings-section">
                <h2 class="section-title">"About"</h2>
                <div class="settings-card">
                    <div class="settings-row">
                        <span class="settings-label">"Version"</span>
                        <span class="settings-value">"0.1.0"</span>
                    </div>
                    <div class="settings-row">
                        <span class="settings-label">"Architecture"</span>
                        <span class="settings-value">"Holochain + Leptos WASM"</span>
                    </div>
                    <div class="settings-row">
                        <span class="settings-label">"Encryption"</span>
                        <span class="settings-value">"XChaCha20-Poly1305"</span>
                    </div>
                </div>
            </section>
        </div>
    }
}
