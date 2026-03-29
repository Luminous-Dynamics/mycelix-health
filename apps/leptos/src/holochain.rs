// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Holochain conductor connection context for the health portal.
//!
//! Provides HolochainCtx via Leptos context. Pages call zome functions
//! through this. Falls back to mock data when no conductor is available.

use leptos::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::JsValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Mock,
}

impl ConnectionStatus {
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Disconnected => "status-disconnected",
            Self::Connecting => "status-connecting",
            Self::Connected => "status-connected",
            Self::Mock => "status-mock",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Disconnected => "Disconnected",
            Self::Connecting => "Connecting...",
            Self::Connected => "Live",
            Self::Mock => "Local",
        }
    }
}

/// Holochain client context shared across the portal.
#[derive(Clone)]
pub struct HolochainCtx {
    pub status: ReadSignal<ConnectionStatus>,
    set_status: WriteSignal<ConnectionStatus>,
}

impl HolochainCtx {
    /// Call a zome function. Returns mock error when no conductor.
    pub async fn call_zome<I: Serialize, O: DeserializeOwned>(
        &self,
        zome: &str,
        fn_name: &str,
        _input: &I,
    ) -> Result<O, String> {
        // TODO: Wire to conductor via WebSocket + MessagePack
        // For now, returns error so pages fall back to mock data.
        Err(format!("[health] Mock: {}.{}", zome, fn_name))
    }

    pub fn is_mock(&self) -> bool {
        self.status.get_untracked() == ConnectionStatus::Mock
    }
}

fn read_js_conductor_status() -> ConnectionStatus {
    let Some(window) = web_sys::window() else {
        return ConnectionStatus::Mock;
    };
    let val = js_sys::Reflect::get(&window, &JsValue::from_str("__HC_STATUS"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "mock".to_string());
    match val.as_str() {
        "connected" => ConnectionStatus::Connected,
        "connecting" => ConnectionStatus::Connecting,
        "disconnected" => ConnectionStatus::Disconnected,
        _ => ConnectionStatus::Mock,
    }
}

/// Wrap the app with HolochainCtx.
#[component]
pub fn HolochainProvider(children: Children) -> impl IntoView {
    let initial = read_js_conductor_status();
    let (status, set_status) = signal(initial);

    if status.get_untracked() == ConnectionStatus::Connecting {
        set_timeout(
            move || {
                let resolved = read_js_conductor_status();
                set_status.set(resolved);
                web_sys::console::log_1(
                    &format!("[health] Conductor status: {:?}", resolved).into(),
                );
            },
            std::time::Duration::from_millis(3500),
        );
    } else {
        web_sys::console::log_1(
            &format!("[health] Conductor status: {:?}", initial).into(),
        );
    }

    let ctx = HolochainCtx { status, set_status };
    provide_context(ctx);

    children()
}

/// Access the Holochain context from any page.
pub fn use_holochain() -> HolochainCtx {
    use_context::<HolochainCtx>()
        .expect("HolochainProvider must wrap the app")
}

/// Connection status badge component.
#[component]
pub fn ConnectionBadge() -> impl IntoView {
    let ctx = use_holochain();
    let status_class = move || format!("connection-badge {}", ctx.status.get().css_class());
    let status_label = move || ctx.status.get().label();

    view! {
        <div class=status_class aria-label="Conductor connection status">
            <span class="badge-dot" />
            <span class="badge-label">{status_label}</span>
        </div>
    }
}
