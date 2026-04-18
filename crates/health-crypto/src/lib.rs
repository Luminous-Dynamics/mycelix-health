// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Post-quantum hybrid encryption for Mycelix Health records.
//!
//! Combines ML-KEM-768 (NIST FIPS 203) for key encapsulation with
//! XChaCha20-Poly1305 for authenticated symmetric encryption.
//!
//! Flow:
//! 1. Patient generates ML-KEM-768 keypair (client-side)
//! 2. Encryptor uses patient's public key to encapsulate a shared secret
//! 3. Shared secret → HKDF-SHA256 → 32-byte symmetric key
//! 4. XChaCha20-Poly1305 encrypts the health record
//! 5. Both KEM ciphertext and AEAD ciphertext stored on DHT
//!
//! Decryption requires the patient's ML-KEM private key (never stored on DHT).

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use chacha20poly1305::{XChaCha20Poly1305, KeyInit, aead::Aead};

/// ML-KEM-768 public key size (1184 bytes).
pub const KEM_PUBLIC_KEY_SIZE: usize = 1184;
/// ML-KEM-768 ciphertext size (1088 bytes).
pub const KEM_CIPHERTEXT_SIZE: usize = 1088;
/// ML-KEM-768 shared secret size (32 bytes).
pub const KEM_SHARED_SECRET_SIZE: usize = 32;

/// Post-quantum encrypted health record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PqEncryptedRecord {
    /// ML-KEM-768 encapsulated shared secret (1088 bytes).
    pub kem_ciphertext: Vec<u8>,
    /// XChaCha20-Poly1305 ciphertext (plaintext + 16-byte auth tag).
    pub aead_ciphertext: Vec<u8>,
    /// XChaCha20 nonce (24 bytes).
    pub nonce: [u8; 24],
    /// Key version for rotation support.
    pub key_version: u32,
    /// Data category (cleartext for consent checking).
    pub data_category: String,
    /// Entry type name for deserialization routing.
    pub entry_type: String,
}

/// Patient's ML-KEM-768 key bundle (public part only — private stays client-side).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PqKeyBundle {
    /// ML-KEM-768 public key (1184 bytes).
    pub kem_public_key: Vec<u8>,
    /// Key version (monotonically increasing).
    pub key_version: u32,
    /// SHA-256 fingerprint of the public key (first 8 bytes).
    pub fingerprint: [u8; 8],
}

/// Generate a random 24-byte nonce.
pub fn generate_nonce() -> [u8; 24] {
    let mut nonce = [0u8; 24];
    getrandom::getrandom(&mut nonce).expect("getrandom failed");
    nonce
}

/// Derive a 32-byte symmetric key from a KEM shared secret.
/// Uses SHA-256(salt || shared_secret || context) — simple and WASM-safe.
pub fn derive_symmetric_key(shared_secret: &[u8], context: &[u8]) -> [u8; 32] {
    let salt = b"mycelix-health-pq-v1";

    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(shared_secret);
    hasher.update(context);
    let hash = hasher.finalize();

    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

/// Encrypt a health record using hybrid PQ encryption.
///
/// 1. Encapsulates with patient's ML-KEM-768 public key → shared secret
/// 2. Derives symmetric key via HKDF
/// 3. Encrypts plaintext with XChaCha20-Poly1305
///
/// The `kem_encapsulate` closure abstracts the ML-KEM operation so this
/// function works with any KEM implementation (ml-kem crate, pqcrypto-kyber, etc.)
pub fn encrypt_hybrid<F>(
    plaintext: &[u8],
    kem_encapsulate: F,
    key_version: u32,
    data_category: &str,
    entry_type: &str,
) -> Result<PqEncryptedRecord, String>
where
    F: FnOnce() -> Result<(Vec<u8>, Vec<u8>), String>, // Returns (kem_ciphertext, shared_secret)
{
    // Step 1: KEM encapsulation
    let (kem_ciphertext, shared_secret) = kem_encapsulate()?;

    // Step 2: Derive symmetric key
    let sym_key = derive_symmetric_key(&shared_secret, b"health-record-encryption");

    // Step 3: Generate nonce
    let nonce = generate_nonce();

    // Step 4: Encrypt with XChaCha20-Poly1305
    let cipher_key = chacha20poly1305::Key::from_slice(&sym_key);
    let cipher = XChaCha20Poly1305::new(cipher_key);
    let xnonce = chacha20poly1305::XNonce::from_slice(&nonce);

    let aead_ciphertext = cipher.encrypt(xnonce, plaintext)
        .map_err(|e| format!("AEAD encryption failed: {}", e))?;

    Ok(PqEncryptedRecord {
        kem_ciphertext,
        aead_ciphertext,
        nonce,
        key_version,
        data_category: data_category.to_string(),
        entry_type: entry_type.to_string(),
    })
}

/// Decrypt a hybrid PQ encrypted health record.
///
/// The `kem_decapsulate` closure abstracts the ML-KEM decapsulation.
pub fn decrypt_hybrid<F>(
    record: &PqEncryptedRecord,
    kem_decapsulate: F,
) -> Result<Vec<u8>, String>
where
    F: FnOnce(&[u8]) -> Result<Vec<u8>, String>, // Takes kem_ciphertext, returns shared_secret
{
    // Step 1: KEM decapsulation
    let shared_secret = kem_decapsulate(&record.kem_ciphertext)?;

    // Step 2: Derive symmetric key (same derivation as encryption)
    let sym_key = derive_symmetric_key(&shared_secret, b"health-record-encryption");

    // Step 3: Decrypt with XChaCha20-Poly1305
    let cipher_key = chacha20poly1305::Key::from_slice(&sym_key);
    let cipher = XChaCha20Poly1305::new(cipher_key);
    let xnonce = chacha20poly1305::XNonce::from_slice(&record.nonce);

    cipher.decrypt(xnonce, record.aead_ciphertext.as_ref())
        .map_err(|_| "Decryption failed: authentication tag mismatch (wrong key or tampered data)".to_string())
}

/// Compute key fingerprint (SHA-256, first 8 bytes).
pub fn key_fingerprint(public_key: &[u8]) -> [u8; 8] {
    let hash = Sha256::digest(public_key);
    let mut fp = [0u8; 8];
    fp.copy_from_slice(&hash[..8]);
    fp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_encrypt_decrypt_roundtrip() {
        let plaintext = b"Patient glucose: 85 mg/dL, reference: 70-100";

        // Simulate KEM with a fixed shared secret (real ML-KEM would use the crate)
        let fake_shared_secret = vec![42u8; 32];
        let fake_kem_ct = vec![99u8; 1088];

        let encrypted = encrypt_hybrid(
            plaintext,
            || Ok((fake_kem_ct.clone(), fake_shared_secret.clone())),
            1,
            "LabResults",
            "LabResult",
        ).unwrap();

        assert_eq!(encrypted.kem_ciphertext.len(), 1088);
        assert!(encrypted.aead_ciphertext.len() > plaintext.len()); // ciphertext + tag

        let decrypted = decrypt_hybrid(
            &encrypted,
            |ct| {
                assert_eq!(ct.len(), 1088);
                Ok(fake_shared_secret.clone())
            },
        ).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let plaintext = b"Sensitive health data";
        let shared_secret = vec![42u8; 32];

        let encrypted = encrypt_hybrid(
            plaintext,
            || Ok((vec![0u8; 1088], shared_secret.clone())),
            1, "LabResults", "LabResult",
        ).unwrap();

        let wrong_secret = vec![99u8; 32]; // Wrong key
        let result = decrypt_hybrid(
            &encrypted,
            |_| Ok(wrong_secret.clone()),
        );

        assert!(result.is_err(), "Wrong key must fail decryption");
    }

    #[test]
    fn key_fingerprint_deterministic() {
        let key = vec![1u8; 1184];
        let fp1 = key_fingerprint(&key);
        let fp2 = key_fingerprint(&key);
        assert_eq!(fp1, fp2);

        let different_key = vec![2u8; 1184];
        let fp3 = key_fingerprint(&different_key);
        assert_ne!(fp1, fp3);
    }

    #[test]
    fn derive_key_deterministic() {
        let secret = vec![42u8; 32];
        let k1 = derive_symmetric_key(&secret, b"test");
        let k2 = derive_symmetric_key(&secret, b"test");
        assert_eq!(k1, k2);

        let k3 = derive_symmetric_key(&secret, b"different");
        assert_ne!(k1, k3);
    }
}
