// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Onboarding — "Creating Your Health Vault"
//!
//! Three steps:
//! 1. Welcome — explain what this is
//! 2. Vault creation — generate key (10s animation covers computation)
//! 3. Seed phrase backup — write down 24 words, verify 4 of them

use leptos::prelude::*;
use crate::crypto::key_manager;

#[derive(Clone, Copy, PartialEq)]
enum OnboardingStep {
    Welcome,
    Creating,
    SeedPhrase,
    Verify,
    Complete,
}

#[component]
pub fn OnboardingPage() -> impl IntoView {
    let step = RwSignal::new(OnboardingStep::Welcome);
    let seed_phrase: RwSignal<Vec<String>> = RwSignal::new(vec![]);
    let vault_key: RwSignal<Option<[u8; 32]>> = RwSignal::new(None);
    let error_msg: RwSignal<Option<String>> = RwSignal::new(None);

    // Verification state
    let verify_indices = RwSignal::new(vec![2usize, 7, 16, 21]); // Words 3, 8, 17, 22
    let verify_inputs: RwSignal<Vec<String>> = RwSignal::new(vec![String::new(); 4]);

    let start_creation = move |_| {
        step.set(OnboardingStep::Creating);

        // Generate vault after a brief delay (animation covers computation)
        gloo_timers::callback::Timeout::new(2000, move || {
            match key_manager::generate_vault() {
                Ok((key, phrase)) => {
                    vault_key.set(Some(key));
                    seed_phrase.set(phrase);
                    step.set(OnboardingStep::SeedPhrase);
                },
                Err(e) => {
                    error_msg.set(Some(format!("Vault creation failed: {}", e)));
                    step.set(OnboardingStep::Welcome);
                },
            }
        }).forget();
    };

    let proceed_to_verify = move |_| {
        step.set(OnboardingStep::Verify);
    };

    let verify_and_complete = move |_| {
        let phrase = seed_phrase.get();
        let indices = verify_indices.get();
        let inputs = verify_inputs.get();

        let all_correct = indices.iter().zip(inputs.iter()).all(|(&idx, input)| {
            idx < phrase.len() && phrase[idx].to_lowercase() == input.trim().to_lowercase()
        });

        if all_correct {
            // Store the key (with a default passphrase for now — production asks for one)
            if let Some(key) = vault_key.get() {
                let _ = key_manager::store_wrapped_key(&key, "mycelix-default");
            }
            step.set(OnboardingStep::Complete);
        } else {
            error_msg.set(Some("Some words don't match. Please check carefully.".into()));
        }
    };

    view! {
        <div class="page onboarding-page">
            // Step 1: Welcome
            <Show when=move || step.get() == OnboardingStep::Welcome>
                <div class="onboarding-card welcome">
                    <div class="vault-icon">
                        <svg viewBox="0 0 64 64" width="80" height="80" aria-hidden="true">
                            <circle cx="32" cy="32" r="28" fill="none" stroke="var(--teal-glow)" stroke-width="2" opacity="0.3" />
                            <circle cx="32" cy="32" r="20" fill="none" stroke="var(--teal-glow)" stroke-width="1.5" opacity="0.5" />
                            <circle cx="32" cy="32" r="8" fill="var(--bio-cyan)" opacity="0.8" />
                            // Mycelial tendrils
                            <line x1="32" y1="4" x2="32" y2="12" stroke="var(--teal-deep)" stroke-width="1" opacity="0.4" />
                            <line x1="32" y1="52" x2="32" y2="60" stroke="var(--teal-deep)" stroke-width="1" opacity="0.4" />
                            <line x1="4" y1="32" x2="12" y2="32" stroke="var(--teal-deep)" stroke-width="1" opacity="0.4" />
                            <line x1="52" y1="32" x2="60" y2="32" stroke="var(--teal-deep)" stroke-width="1" opacity="0.4" />
                        </svg>
                    </div>

                    <h1 class="onboarding-title">"You Own Your Health Data"</h1>
                    <p class="onboarding-subtitle">"No corporation. No government. You."</p>

                    <div class="onboarding-points">
                        <div class="point">
                            <span class="point-icon">"1"</span>
                            <span>"Your records are encrypted with a key only you hold"</span>
                        </div>
                        <div class="point">
                            <span class="point-icon">"2"</span>
                            <span>"You choose exactly who can see what"</span>
                        </div>
                        <div class="point">
                            <span class="point-icon">"3"</span>
                            <span>"When your data helps research, the value flows back to you"</span>
                        </div>
                    </div>

                    <button class="onboarding-cta" on:click=start_creation>
                        "Create Your Health Vault"
                    </button>
                </div>
            </Show>

            // Step 2: Creating (animation covers key generation)
            <Show when=move || step.get() == OnboardingStep::Creating>
                <div class="onboarding-card creating">
                    <div class="creation-animation">
                        <div class="creation-ring ring-1" />
                        <div class="creation-ring ring-2" />
                        <div class="creation-ring ring-3" />
                        <div class="creation-core" />
                    </div>
                    <h2>"Growing Your Health Vault"</h2>
                    <p class="creating-text">
                        "Generating a unique biological signature that protects your data. "
                        "This signature is yours alone — not even Mycelix can recreate it."
                    </p>
                </div>
            </Show>

            // Step 3: Seed phrase display
            <Show when=move || step.get() == OnboardingStep::SeedPhrase>
                <div class="onboarding-card seed-phrase">
                    <h2>"Your Recovery Phrase"</h2>
                    <p class="seed-warning">
                        "If you lose your device, these 24 words are the "
                        <strong>"only way"</strong>
                        " to recover your health vault. Write them on paper. Store them safely."
                    </p>

                    <div class="seed-grid" aria-label="Recovery phrase — 24 words">
                        {move || seed_phrase.get().iter().enumerate().map(|(i, word)| {
                            view! {
                                <div class="seed-word">
                                    <span class="seed-num">{(i + 1).to_string()}</span>
                                    <span class="seed-text">{word.clone()}</span>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>

                    <button class="onboarding-cta" on:click=proceed_to_verify>
                        "I Have Written This Down"
                    </button>
                </div>
            </Show>

            // Step 4: Verify 4 words
            <Show when=move || step.get() == OnboardingStep::Verify>
                <div class="onboarding-card verify">
                    <h2>"Verify Your Recovery Phrase"</h2>
                    <p>"Enter the following words to confirm you saved them correctly."</p>

                    {move || {
                        let indices = verify_indices.get();
                        indices.iter().enumerate().map(|(input_idx, &word_idx)| {
                            view! {
                                <div class="verify-field">
                                    <label class="verify-label">
                                        {format!("Word #{}", word_idx + 1)}
                                    </label>
                                    <input
                                        type="text"
                                        class="verify-input"
                                        placeholder="Enter word..."
                                        autocomplete="off"
                                        on:input=move |ev| {
                                            let val = event_target_value(&ev);
                                            verify_inputs.update(|inputs| {
                                                if input_idx < inputs.len() {
                                                    inputs[input_idx] = val;
                                                }
                                            });
                                        }
                                    />
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }}

                    <Show when=move || error_msg.get().is_some()>
                        <div class="verify-error">
                            {move || error_msg.get().unwrap_or_default()}
                        </div>
                    </Show>

                    <button class="onboarding-cta" on:click=verify_and_complete>
                        "Verify & Activate Vault"
                    </button>
                </div>
            </Show>

            // Step 5: Complete
            <Show when=move || step.get() == OnboardingStep::Complete>
                <div class="onboarding-card complete">
                    <div class="complete-icon">
                        <svg viewBox="0 0 64 64" width="80" height="80" aria-hidden="true">
                            <circle cx="32" cy="32" r="28" fill="var(--teal-surface)" stroke="var(--active)" stroke-width="2" />
                            <polyline points="20,32 28,40 44,24" fill="none" stroke="var(--active)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" />
                        </svg>
                    </div>
                    <h2>"Your Health Vault is Active"</h2>
                    <p>
                        "Your data is now protected by a biological signature "
                        "that only you control. Welcome to health sovereignty."
                    </p>

                    <a href="/" class="onboarding-cta">"Enter Your Vault"</a>
                </div>
            </Show>
        </div>
    }
}
