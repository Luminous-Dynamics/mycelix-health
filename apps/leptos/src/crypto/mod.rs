// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Client-side cryptographic key management.
//!
//! The patient's private key NEVER leaves the browser.
//! Key generation, encryption, and decryption all happen in WASM.

pub mod key_manager;
pub mod seed_phrase;
