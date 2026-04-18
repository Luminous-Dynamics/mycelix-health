// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Mycelix Health Portal — Biological Sovereignty
//!
//! Patient health data sovereignty portal built on Leptos CSR.
//! The UI is a living biological operating system, not a corporate vault.

use leptos::prelude::*;

mod app;
mod components;
mod crypto;
mod pages;
mod zome_clients;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
