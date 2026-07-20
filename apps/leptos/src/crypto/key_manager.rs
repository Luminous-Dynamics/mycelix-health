// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Client-side health-vault key management.
//!
//! Security boundary:
//! 1. Web Crypto supplies patient-vault entropy, per-vault salt, and nonce.
//! 2. The patient encryption key is derived with RFC 5869 HMAC-HKDF-SHA256.
//! 3. The local wrapping key is derived with PBKDF2-HMAC-SHA256 using a random
//!    salt and a versioned work factor.
//! 4. The vault key is wrapped with XChaCha20-Poly1305 and authenticated
//!    metadata. No plaintext key or unauthenticated ciphertext is persisted.
//! 5. Legacy XOR-wrapped material is detected but never decrypted by this path.

use chacha20poly1305::{
    aead::{Aead, Payload},
    KeyInit, XChaCha20Poly1305, XNonce,
};
use serde::{Deserialize, Serialize};
use web_sys::Window;
use zeroize::{Zeroize, Zeroizing};

use super::seed_phrase;

const VAULT_STORAGE_KEY_V2: &str = "mycelix_health_vault_v2";
const LEGACY_VAULT_KEY: &str = "mycelix_health_vault_key";
const LEGACY_FINGERPRINT_KEY: &str = "mycelix_health_vault_fp";
const VAULT_ENVELOPE_VERSION: u8 = 2;
const VAULT_KDF: &str = "PBKDF2-HMAC-SHA256";
const PBKDF2_ITERATIONS: u32 = 210_000;
const MIN_ACCEPTED_ITERATIONS: u32 = 100_000;
const MAX_ACCEPTED_ITERATIONS: u32 = 2_000_000;
const MIN_PASSPHRASE_BYTES: usize = 12;
const MAX_PASSPHRASE_BYTES: usize = 1_024;
const VAULT_WRAP_AAD_DOMAIN: &[u8] = b"MYCELIX-HEALTH-VAULT-WRAP-V2";
const PATIENT_KEY_SALT: &[u8] = b"mycelix-health-v1-patient-encryption";
const PATIENT_KEY_CONTEXT: &[u8] = b"patient-vault-key";

/// Storage format present in the browser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultStorageStatus {
    None,
    /// The former fixed-salt/XOR wrapper exists. Recovery and re-sealing are
    /// required; the insecure wrapper is never decrypted by current code.
    LegacyInsecure,
    CurrentV2,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct StoredVaultEnvelopeV2 {
    version: u8,
    kdf: String,
    iterations: u32,
    salt_hex: String,
    nonce_hex: String,
    ciphertext_hex: String,
    fingerprint_hex: String,
}

/// Generate 32 bytes of cryptographic entropy using Web Crypto API.
pub fn generate_entropy() -> Result<[u8; 32], String> {
    generate_random_bytes()
}

fn generate_random_bytes<const N: usize>() -> Result<[u8; N], String> {
    let window: Window = web_sys::window().ok_or("No window object")?;
    let crypto = window
        .crypto()
        .map_err(|_| "Web Crypto API not available")?;

    let mut bytes = [0u8; N];
    crypto
        .get_random_values_with_u8_array(&mut bytes)
        .map_err(|_| "crypto.getRandomValues failed")?;
    Ok(bytes)
}

/// Generate a new vault key and seed phrase.
///
/// The phrase is the recovery root. The derived key is held in memory until it
/// is wrapped with a passphrase and transferred into a bounded session.
pub fn generate_vault() -> Result<([u8; 32], Vec<String>), String> {
    let mut entropy = generate_entropy()?;
    let phrase = seed_phrase::entropy_to_phrase(&entropy);
    let key = derive_key_from_entropy(&entropy);
    entropy.zeroize();
    Ok((key, phrase))
}

/// Recover a vault key from a seed phrase.
pub fn recover_vault(words: &[String]) -> Result<[u8; 32], String> {
    let mut entropy = seed_phrase::phrase_to_entropy(words)?;
    let key = derive_key_from_entropy(&entropy);
    entropy.zeroize();
    Ok(key)
}

/// Derive the patient encryption key with RFC 5869 HMAC-HKDF-SHA256.
fn derive_key_from_entropy(entropy: &[u8; 32]) -> [u8; 32] {
    let mut prk = hmac_sha256(PATIENT_KEY_SALT, entropy);
    let okm = hkdf_expand_sha256(&prk, PATIENT_KEY_CONTEXT);
    prk.zeroize();
    okm
}

fn hkdf_expand_sha256(prk: &[u8; 32], info: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(info.len() + 1);
    input.extend_from_slice(info);
    input.push(1);
    hmac_sha256(prk, &input)
}

/// Compute a display/routing fingerprint. It is authenticated as part of the
/// wrapping AAD and is not used as a substitute for AEAD verification.
pub fn compute_fingerprint(key: &[u8; 32]) -> [u8; 8] {
    let hash = sha256(key);
    let mut fingerprint = [0u8; 8];
    fingerprint.copy_from_slice(&hash[..8]);
    fingerprint
}

/// Persist a versioned, authenticated wrapper for the patient vault key.
pub fn store_wrapped_key(key: &[u8; 32], passphrase: &str) -> Result<(), String> {
    validate_passphrase(passphrase)?;
    let storage = local_storage()?;

    let salt = generate_random_bytes::<16>()?;
    let nonce = generate_random_bytes::<24>()?;
    let fingerprint = compute_fingerprint(key);
    let wrapping_key = Zeroizing::new(pbkdf2_hmac_sha256(
        passphrase.as_bytes(),
        &salt,
        PBKDF2_ITERATIONS,
    ));
    let aad = vault_wrap_aad(VAULT_ENVELOPE_VERSION, PBKDF2_ITERATIONS, &fingerprint);
    let cipher = XChaCha20Poly1305::new_from_slice(&wrapping_key)
        .map_err(|_| "Invalid local wrapping key length")?;
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: key,
                aad: &aad,
            },
        )
        .map_err(|_| "Failed to authenticate and wrap vault key")?;

    let envelope = StoredVaultEnvelopeV2 {
        version: VAULT_ENVELOPE_VERSION,
        kdf: VAULT_KDF.to_string(),
        iterations: PBKDF2_ITERATIONS,
        salt_hex: bytes_to_hex(&salt),
        nonce_hex: bytes_to_hex(&nonce),
        ciphertext_hex: bytes_to_hex(&ciphertext),
        fingerprint_hex: bytes_to_hex(&fingerprint),
    };
    validate_envelope(&envelope)?;
    let encoded = serde_json::to_string(&envelope)
        .map_err(|error| format!("Failed to encode vault envelope: {error}"))?;
    storage
        .set_item(VAULT_STORAGE_KEY_V2, &encoded)
        .map_err(|_| "Failed to store vault envelope")?;

    // A successful v2 seal supersedes the insecure legacy representation.
    let _ = storage.remove_item(LEGACY_VAULT_KEY);
    let _ = storage.remove_item(LEGACY_FINGERPRINT_KEY);
    Ok(())
}

pub fn vault_storage_status() -> VaultStorageStatus {
    let Ok(storage) = local_storage() else {
        return VaultStorageStatus::None;
    };
    if storage
        .get_item(VAULT_STORAGE_KEY_V2)
        .ok()
        .flatten()
        .is_some()
    {
        return VaultStorageStatus::CurrentV2;
    }
    if storage
        .get_item(LEGACY_VAULT_KEY)
        .ok()
        .flatten()
        .is_some()
    {
        return VaultStorageStatus::LegacyInsecure;
    }
    VaultStorageStatus::None
}

pub fn has_stored_key() -> bool {
    !matches!(vault_storage_status(), VaultStorageStatus::None)
}

/// Authenticate and unwrap a stored v2 vault key.
///
/// Legacy wrappers fail closed because they provide no ciphertext integrity and
/// used a fixed salt. Patients must recover from the seed phrase and re-seal.
pub fn unwrap_key(passphrase: &str) -> Result<[u8; 32], String> {
    validate_passphrase(passphrase)?;
    let storage = local_storage()?;
    let Some(encoded) = storage
        .get_item(VAULT_STORAGE_KEY_V2)
        .map_err(|_| "Failed to read vault envelope")?
    else {
        if storage
            .get_item(LEGACY_VAULT_KEY)
            .ok()
            .flatten()
            .is_some()
        {
            return Err(
                "Legacy insecure vault wrapper detected. Recover with your seed phrase and seal a new vault before accessing clinical data."
                    .to_string(),
            );
        }
        return Err("No stored vault found".to_string());
    };

    let envelope: StoredVaultEnvelopeV2 = serde_json::from_str(&encoded)
        .map_err(|_| "Stored vault envelope is malformed")?;
    validate_envelope(&envelope)?;

    let salt = decode_fixed::<16>(&envelope.salt_hex, "salt")?;
    let nonce = decode_fixed::<24>(&envelope.nonce_hex, "nonce")?;
    let fingerprint = decode_fixed::<8>(&envelope.fingerprint_hex, "fingerprint")?;
    let ciphertext = hex_to_bytes(&envelope.ciphertext_hex)?;
    let wrapping_key = Zeroizing::new(pbkdf2_hmac_sha256(
        passphrase.as_bytes(),
        &salt,
        envelope.iterations,
    ));
    let aad = vault_wrap_aad(envelope.version, envelope.iterations, &fingerprint);
    let cipher = XChaCha20Poly1305::new_from_slice(&wrapping_key)
        .map_err(|_| "Invalid local wrapping key length")?;
    let plaintext = Zeroizing::new(cipher
        .decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| "Wrong passphrase or tampered vault envelope")?);

    if plaintext.len() != 32 {
        return Err("Authenticated vault payload has the wrong length".to_string());
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&plaintext);
    if !constant_time_eq(&compute_fingerprint(&key), &fingerprint) {
        key.zeroize();
        return Err("Authenticated vault fingerprint mismatch".to_string());
    }
    Ok(key)
}

/// Destroy current and legacy local vault material.
pub fn destroy_vault() -> Result<(), String> {
    let storage = local_storage()?;
    storage
        .remove_item(VAULT_STORAGE_KEY_V2)
        .map_err(|_| "Failed to remove current vault envelope")?;
    storage
        .remove_item(LEGACY_VAULT_KEY)
        .map_err(|_| "Failed to remove legacy vault key")?;
    storage
        .remove_item(LEGACY_FINGERPRINT_KEY)
        .map_err(|_| "Failed to remove legacy fingerprint")?;
    Ok(())
}

fn validate_passphrase(passphrase: &str) -> Result<(), String> {
    let length = passphrase.as_bytes().len();
    if length < MIN_PASSPHRASE_BYTES {
        return Err(format!(
            "Passphrase must be at least {MIN_PASSPHRASE_BYTES} bytes"
        ));
    }
    if length > MAX_PASSPHRASE_BYTES {
        return Err("Passphrase is too long".to_string());
    }
    Ok(())
}

fn validate_envelope(envelope: &StoredVaultEnvelopeV2) -> Result<(), String> {
    if envelope.version != VAULT_ENVELOPE_VERSION {
        return Err(format!(
            "Unsupported vault envelope version {}",
            envelope.version
        ));
    }
    if envelope.kdf != VAULT_KDF {
        return Err("Unsupported vault KDF".to_string());
    }
    if !(MIN_ACCEPTED_ITERATIONS..=MAX_ACCEPTED_ITERATIONS).contains(&envelope.iterations) {
        return Err("Vault KDF work factor is outside the accepted policy".to_string());
    }
    let salt = decode_fixed::<16>(&envelope.salt_hex, "salt")?;
    let nonce = decode_fixed::<24>(&envelope.nonce_hex, "nonce")?;
    let fingerprint = decode_fixed::<8>(&envelope.fingerprint_hex, "fingerprint")?;
    let ciphertext = hex_to_bytes(&envelope.ciphertext_hex)?;
    if salt.iter().all(|byte| *byte == 0)
        || nonce.iter().all(|byte| *byte == 0)
        || fingerprint.iter().all(|byte| *byte == 0)
    {
        return Err("Vault envelope contains an invalid all-zero field".to_string());
    }
    if ciphertext.len() != 48 {
        return Err("Vault ciphertext must contain 32 bytes plus a 16-byte tag".to_string());
    }
    Ok(())
}

fn vault_wrap_aad(version: u8, iterations: u32, fingerprint: &[u8; 8]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(VAULT_WRAP_AAD_DOMAIN.len() + 1 + 4 + 8);
    aad.extend_from_slice(VAULT_WRAP_AAD_DOMAIN);
    aad.push(version);
    aad.extend_from_slice(&iterations.to_be_bytes());
    aad.extend_from_slice(fingerprint);
    aad
}

fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    debug_assert!(iterations > 0);
    let mut first_input = Vec::with_capacity(salt.len() + 4);
    first_input.extend_from_slice(salt);
    first_input.extend_from_slice(&1u32.to_be_bytes());

    let mut u = hmac_sha256(password, &first_input);
    let mut output = u;
    for _ in 1..iterations {
        u = hmac_sha256(password, &u);
        for (target, value) in output.iter_mut().zip(u.iter()) {
            *target ^= *value;
        }
    }
    u.zeroize();
    output
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut normalized_key = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        normalized_key[..32].copy_from_slice(&sha256(key));
    } else {
        normalized_key[..key.len()].copy_from_slice(key);
    }

    let mut inner_pad = [0x36u8; BLOCK_SIZE];
    let mut outer_pad = [0x5cu8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        inner_pad[index] ^= normalized_key[index];
        outer_pad[index] ^= normalized_key[index];
    }

    let mut inner_input = Vec::with_capacity(BLOCK_SIZE + data.len());
    inner_input.extend_from_slice(&inner_pad);
    inner_input.extend_from_slice(data);
    let mut inner_hash = sha256(&inner_input);

    let mut outer_input = Vec::with_capacity(BLOCK_SIZE + inner_hash.len());
    outer_input.extend_from_slice(&outer_pad);
    outer_input.extend_from_slice(&inner_hash);
    let result = sha256(&outer_input);

    normalized_key.zeroize();
    inner_pad.zeroize();
    outer_pad.zeroize();
    inner_hash.zeroize();
    result
}

fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let result = Sha256::digest(data);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

fn local_storage() -> Result<web_sys::Storage, String> {
    let window = web_sys::window().ok_or("No window")?;
    window
        .local_storage()
        .map_err(|_| "localStorage not available")?
        .ok_or_else(|| "localStorage is null".to_string())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Invalid odd-length hex encoding".to_string());
    }
    hex.as_bytes()
        .chunks_exact(2)
        .enumerate()
        .map(|(index, pair)| {
            let pair = std::str::from_utf8(pair)
                .map_err(|_| format!("Invalid hex at byte pair {index}"))?;
            u8::from_str_radix(pair, 16)
                .map_err(|_| format!("Invalid hex at byte pair {index}"))
        })
        .collect()
}

fn decode_fixed<const N: usize>(hex: &str, label: &str) -> Result<[u8; N], String> {
    let bytes = hex_to_bytes(hex)?;
    if bytes.len() != N {
        return Err(format!("Vault {label} has the wrong length"));
    }
    let mut fixed = [0u8; N];
    fixed.copy_from_slice(&bytes);
    Ok(fixed)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sha256_matches_rfc_4231_case_one() {
        let key = [0x0bu8; 20];
        let digest = hmac_sha256(&key, b"Hi There");
        assert_eq!(
            bytes_to_hex(&digest),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn pbkdf2_sha256_matches_known_vectors() {
        assert_eq!(
            bytes_to_hex(&pbkdf2_hmac_sha256(b"password", b"salt", 1)),
            "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
        );
        assert_eq!(
            bytes_to_hex(&pbkdf2_hmac_sha256(b"password", b"salt", 2)),
            "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43"
        );
    }

    #[test]
    fn hkdf_expand_matches_rfc_5869_prefix() {
        let ikm = [0x0bu8; 22];
        let salt = decode_fixed::<13>("000102030405060708090a0b0c", "test salt")
            .expect("valid test salt");
        let info = decode_fixed::<10>("f0f1f2f3f4f5f6f7f8f9", "test info")
            .expect("valid test info");
        let prk = hmac_sha256(&salt, &ikm);
        assert_eq!(
            bytes_to_hex(&hkdf_expand_sha256(&prk, &info)),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf"
        );
    }

    #[test]
    fn odd_hex_and_weak_envelopes_are_rejected() {
        assert!(hex_to_bytes("abc").is_err());
        let envelope = StoredVaultEnvelopeV2 {
            version: VAULT_ENVELOPE_VERSION,
            kdf: VAULT_KDF.to_string(),
            iterations: PBKDF2_ITERATIONS,
            salt_hex: "00".repeat(16),
            nonce_hex: "01".repeat(24),
            ciphertext_hex: "02".repeat(48),
            fingerprint_hex: "03".repeat(8),
        };
        assert!(validate_envelope(&envelope).is_err());
    }
}
