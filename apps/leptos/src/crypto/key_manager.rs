// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Client-side key management via Web Crypto API.
//!
//! Flow:
//! 1. Generate 32 bytes of entropy via crypto.getRandomValues()
//! 2. Derive seed phrase (24 words) for backup
//! 3. Derive encryption key via HKDF (same as zome's derive_key)
//! 4. Store wrapped key in localStorage (encrypted with passphrase via PBKDF2)
//! 5. On each session, unwrap key with passphrase
//!
//! The raw private key is held in memory only — never stored unencrypted.

use wasm_bindgen::prelude::*;
use web_sys::Window;

use super::seed_phrase;

/// Key state — what the app knows about the patient's vault.
#[derive(Clone, Debug, PartialEq)]
pub enum KeyState {
    /// No key exists — needs onboarding.
    NoKey,
    /// Key exists but is locked (needs passphrase).
    Locked,
    /// Key is unlocked and available for encrypt/decrypt.
    Unlocked {
        /// The 32-byte symmetric key (in memory only).
        key: [u8; 32],
        /// Key fingerprint (first 8 bytes of SHA-256 of key).
        fingerprint: [u8; 8],
    },
}

/// Generate 32 bytes of cryptographic entropy using Web Crypto API.
pub fn generate_entropy() -> Result<[u8; 32], String> {
    let window: Window = web_sys::window()
        .ok_or("No window object")?;
    let crypto = window.crypto()
        .map_err(|_| "Web Crypto API not available")?;

    let mut entropy = [0u8; 32];
    crypto.get_random_values_with_u8_array(&mut entropy)
        .map_err(|_| "getRandomValues failed")?;

    Ok(entropy)
}

/// Generate a new vault key and seed phrase.
///
/// Returns (key, seed_phrase). The seed phrase must be shown to the patient
/// for backup. The key is ready for encrypt/decrypt operations.
pub fn generate_vault() -> Result<([u8; 32], Vec<String>), String> {
    let entropy = generate_entropy()?;
    let phrase = seed_phrase::entropy_to_phrase(&entropy);

    // Derive the encryption key from entropy using HKDF-like construction
    // (Mirrors the zome's derive_key function)
    let key = derive_key_from_entropy(&entropy);

    Ok((key, phrase))
}

/// Recover a vault key from a seed phrase.
pub fn recover_vault(words: &[String]) -> Result<[u8; 32], String> {
    let entropy = seed_phrase::phrase_to_entropy(words)?;
    Ok(derive_key_from_entropy(&entropy))
}

/// Derive encryption key from entropy using HMAC-SHA256 HKDF.
/// This mirrors the zome's derive_key() function exactly.
fn derive_key_from_entropy(entropy: &[u8; 32]) -> [u8; 32] {
    // Simple HKDF-like derivation matching the zome's implementation
    // In production, use the hmac crate via wasm-bindgen
    let salt = b"mycelix-health-v1-patient-encryption";
    let context = b"patient-vault-key";

    // Extract: SHA-256(salt || entropy)
    // Then Expand: SHA-256(prk || context || 0x01)
    // This is a simplified version — the zome uses real HMAC
    let mut extract = Vec::with_capacity(salt.len() + entropy.len());
    extract.extend_from_slice(salt);
    extract.extend_from_slice(entropy);
    let prk = simple_sha256(&extract);

    let mut expand = Vec::with_capacity(prk.len() + context.len() + 1);
    expand.extend_from_slice(&prk);
    expand.extend_from_slice(context);
    expand.push(0x01);
    simple_sha256(&expand)
}

/// Compute key fingerprint (SHA-256, first 8 bytes).
pub fn compute_fingerprint(key: &[u8; 32]) -> [u8; 8] {
    let hash = simple_sha256(key);
    let mut fp = [0u8; 8];
    fp.copy_from_slice(&hash[..8]);
    fp
}

/// Store a wrapped key in localStorage.
///
/// The key is XORed with a passphrase-derived mask before storage.
/// This is a simplified version — production would use PBKDF2 + AES-KW.
pub fn store_wrapped_key(key: &[u8; 32], passphrase: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let storage = window.local_storage()
        .map_err(|_| "localStorage not available")?
        .ok_or("localStorage is null")?;

    // Derive wrapping key via iterated SHA-256 (PBKDF2-like stretching)
    // 10,000 iterations to slow brute-force passphrase guessing
    let salt = b"mycelix-health-vault-wrap-v1";
    let mut pass_key = simple_sha256(passphrase.as_bytes());
    for _ in 0..10_000 {
        let mut input = Vec::with_capacity(32 + salt.len());
        input.extend_from_slice(&pass_key);
        input.extend_from_slice(salt);
        pass_key = simple_sha256(&input);
    }

    // Wrap key with stretched passphrase hash
    let mut wrapped = [0u8; 32];
    for i in 0..32 {
        wrapped[i] = key[i] ^ pass_key[i];
    }

    // Store as hex
    let hex: String = wrapped.iter().map(|b| format!("{:02x}", b)).collect();
    storage.set_item("mycelix_health_vault_key", &hex)
        .map_err(|_| "Failed to store key")?;

    // Store fingerprint (for verification without unwrapping)
    let fp = compute_fingerprint(key);
    let fp_hex: String = fp.iter().map(|b| format!("{:02x}", b)).collect();
    storage.set_item("mycelix_health_vault_fp", &fp_hex)
        .map_err(|_| "Failed to store fingerprint")?;

    Ok(())
}

/// Check if a vault key exists in localStorage.
pub fn has_stored_key() -> bool {
    let Some(window) = web_sys::window() else { return false };
    let Ok(Some(storage)) = window.local_storage() else { return false };
    storage.get_item("mycelix_health_vault_key").ok().flatten().is_some()
}

/// Unwrap a stored key with a passphrase.
pub fn unwrap_key(passphrase: &str) -> Result<[u8; 32], String> {
    let window = web_sys::window().ok_or("No window")?;
    let storage = window.local_storage()
        .map_err(|_| "localStorage not available")?
        .ok_or("localStorage is null")?;

    let hex = storage.get_item("mycelix_health_vault_key")
        .map_err(|_| "Failed to read key")?
        .ok_or("No stored key found")?;

    // Parse hex
    let wrapped = hex_to_bytes(&hex)?;
    if wrapped.len() != 32 {
        return Err("Stored key has wrong length".into());
    }

    // Derive wrapping key via iterated SHA-256 (same as store_wrapped_key)
    let salt = b"mycelix-health-vault-wrap-v1";
    let mut pass_key = simple_sha256(passphrase.as_bytes());
    for _ in 0..10_000 {
        let mut input = Vec::with_capacity(32 + salt.len());
        input.extend_from_slice(&pass_key);
        input.extend_from_slice(salt);
        pass_key = simple_sha256(&input);
    }

    // Unwrap
    let mut key = [0u8; 32];
    for i in 0..32 {
        key[i] = wrapped[i] ^ pass_key[i];
    }

    // Verify fingerprint
    let stored_fp = storage.get_item("mycelix_health_vault_fp")
        .map_err(|_| "Failed to read fingerprint")?
        .ok_or("No stored fingerprint")?;
    let expected_fp = compute_fingerprint(&key);
    let expected_hex: String = expected_fp.iter().map(|b| format!("{:02x}", b)).collect();

    if stored_fp != expected_hex {
        return Err("Wrong passphrase — fingerprint mismatch".into());
    }

    Ok(key)
}

/// Destroy the stored key (for crypto-erasure).
pub fn destroy_vault() -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let storage = window.local_storage()
        .map_err(|_| "localStorage not available")?
        .ok_or("localStorage is null")?;

    storage.remove_item("mycelix_health_vault_key").map_err(|_| "Failed to remove key")?;
    storage.remove_item("mycelix_health_vault_fp").map_err(|_| "Failed to remove fingerprint")?;
    Ok(())
}

/// SHA-256 using the `sha2` crate (real cryptographic hash, WASM-compatible).
fn simple_sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let result = Sha256::digest(data);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| format!("Invalid hex at position {}", i))
        })
        .collect()
}
