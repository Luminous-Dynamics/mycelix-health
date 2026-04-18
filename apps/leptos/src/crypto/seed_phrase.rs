// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! BIP-39 compatible seed phrase generation and recovery.
//!
//! The seed phrase is the patient's ONLY backup for their health vault.
//! 24 words → 256 bits of entropy → deterministic key derivation.

use super::bip39_wordlist::WORDLIST;

/// BIP-39 English wordlist — full 2048 words.
/// 11 bits per word × 24 words = 264 bits (256 entropy + 8 checksum).
// Full 2048-word BIP-39 wordlist is in bip39_wordlist.rs

/// Generate a 24-word seed phrase from 32 bytes of entropy.
pub fn entropy_to_phrase(entropy: &[u8; 32]) -> Vec<String> {
    // Convert 256 bits to word indices (11 bits each = 23.27 words)
    // We use 24 words = 264 bits, last 8 bits are a simple checksum
    let mut bits = Vec::with_capacity(264);

    // Add entropy bits
    for byte in entropy.iter() {
        for bit in (0..8).rev() {
            bits.push((byte >> bit) & 1);
        }
    }

    // BIP-39 checksum: first 8 bits of SHA-256(entropy)
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(entropy);
    let checksum = hash[0]; // First byte = first 8 bits of SHA-256
    for bit in (0..8).rev() {
        bits.push((checksum >> bit) & 1);
    }

    // Convert 11-bit groups to word indices
    let mut words = Vec::with_capacity(24);
    for chunk in bits.chunks(11) {
        if chunk.len() < 11 { break; }
        let mut index: usize = 0;
        for &bit in chunk {
            index = (index << 1) | (bit as usize);
        }
        let word_idx = index % WORDLIST.len();
        words.push(WORDLIST[word_idx].to_string());
    }

    words
}

/// Convert a seed phrase back to 32 bytes of entropy.
pub fn phrase_to_entropy(words: &[String]) -> Result<[u8; 32], String> {
    if words.len() != 24 {
        return Err(format!("Expected 24 words, got {}", words.len()));
    }

    // Convert words back to 11-bit indices
    let mut bits = Vec::with_capacity(264);
    for word in words {
        let lower = word.to_lowercase();
        let index = WORDLIST.iter().position(|&w| w == lower)
            .ok_or_else(|| format!("Unknown word: '{}'", word))?;
        for bit in (0..11).rev() {
            bits.push(((index >> bit) & 1) as u8);
        }
    }

    // Extract first 256 bits as entropy
    let mut entropy = [0u8; 32];
    for (i, byte_bits) in bits[..256].chunks(8).enumerate() {
        let mut byte: u8 = 0;
        for &bit in byte_bits {
            byte = (byte << 1) | bit;
        }
        entropy[i] = byte;
    }

    Ok(entropy)
}

/// Verify a seed phrase by checking the checksum.
pub fn verify_phrase(words: &[String]) -> bool {
    match phrase_to_entropy(words) {
        Ok(entropy) => {
            let regenerated = entropy_to_phrase(&entropy);
            regenerated == words
        },
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let entropy = [42u8; 32];
        let phrase = entropy_to_phrase(&entropy);
        assert_eq!(phrase.len(), 24);
        let recovered = phrase_to_entropy(&phrase).unwrap();
        assert_eq!(recovered, entropy);
    }

    #[test]
    fn different_entropy_different_phrase() {
        let a = entropy_to_phrase(&[1u8; 32]);
        let b = entropy_to_phrase(&[2u8; 32]);
        assert_ne!(a, b);
    }
}
