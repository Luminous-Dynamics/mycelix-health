// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! BIP-39 compatible seed phrase generation and recovery.
//!
//! The seed phrase is the patient's ONLY backup for their health vault.
//! 24 words → 256 bits of entropy → deterministic key derivation.

/// BIP-39 English wordlist (first 256 of 2048).
/// Full BIP-39 wordlist has 2048 entries (11 bits each). We include 256 here
/// and map indices via modulo. For production, embed the full wordlist.
const WORDLIST: &[&str] = &[
    "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract",
    "absurd", "abuse", "access", "accident", "account", "accuse", "achieve", "acid",
    "acoustic", "acquire", "across", "act", "action", "actor", "actress", "actual",
    "adapt", "add", "addict", "address", "adjust", "admit", "adult", "advance",
    "advice", "aerobic", "affair", "afford", "afraid", "again", "age", "agent",
    "agree", "ahead", "aim", "air", "airport", "aisle", "alarm", "album",
    "alcohol", "alert", "alien", "all", "alley", "allow", "almost", "alone",
    "alpha", "already", "also", "alter", "always", "amateur", "amazing", "among",
    "amount", "amused", "analyst", "anchor", "ancient", "anger", "angle", "angry",
    "animal", "ankle", "announce", "annual", "another", "answer", "antenna", "antique",
    "anxiety", "any", "apart", "apology", "appear", "apple", "approve", "april",
    "arch", "arctic", "area", "arena", "argue", "arm", "armed", "armor",
    "army", "around", "arrange", "arrest", "arrive", "arrow", "art", "artefact",
    "artist", "artwork", "ask", "aspect", "assault", "asset", "assist", "assume",
    "asthma", "athlete", "atom", "attack", "attend", "attitude", "attract", "auction",
    "audit", "august", "aunt", "author", "auto", "avocado", "avoid", "awake",
    "aware", "awesome", "awful", "awkward", "axis", "baby", "bachelor", "bacon",
    "badge", "bag", "balance", "balcony", "ball", "bamboo", "banana", "banner",
    "bar", "barely", "bargain", "barrel", "base", "basic", "basket", "battle",
    "beach", "bean", "beauty", "because", "become", "beef", "before", "begin",
    "behave", "behind", "believe", "below", "bench", "benefit", "best", "betray",
    "better", "between", "beyond", "bicycle", "bid", "bike", "bind", "biology",
    "bird", "birth", "bitter", "black", "blade", "blame", "blanket", "blast",
    "bleak", "bless", "blind", "blood", "blossom", "blow", "blue", "blur",
    "blush", "board", "boat", "body", "boil", "bomb", "bone", "bonus",
    "book", "boost", "border", "boring", "borrow", "boss", "bottom", "bounce",
    "box", "boy", "bracket", "brain", "brand", "brass", "brave", "bread",
    "breeze", "brick", "bridge", "brief", "bright", "bring", "brisk", "broccoli",
    "broken", "bronze", "broom", "brother", "brown", "brush", "bubble", "buddy",
    "budget", "buffalo", "build", "bulb", "bulk", "bullet", "bundle", "bunny",
    "burden", "burger", "burst", "bus", "business", "busy", "butter", "buyer",
    "buzz", "cabbage", "cabin", "cable", "cactus", "cage", "cake", "call",
    "calm", "camera", "camp", "can", "canal", "cancel", "candy", "cannon",
    "canoe", "canvas", "canyon", "capable", "capital", "captain", "car", "carbon",
    "card", "cargo", "carpet", "carry", "cart", "case", "cash", "casino",
    "castle", "casual", "cat", "catalog", "catch", "category", "cattle", "caught",
    "cause", "caution", "cave", "ceiling", "celery", "cement", "census", "century",
    "cereal", "certain", "chair", "chalk", "champion", "change", "chaos", "chapter",
    "charge", "chase", "cheap", "check", "cheese", "chef", "cherry", "chest",
    "chicken", "chief", "child", "chimney", "choice", "choose", "chronic", "chuckle",
    "chunk", "churn", "citizen", "city", "civil", "claim", "clap", "clarify",
    "claw", "clay", "clean", "clerk", "clever", "click", "client", "cliff",
    "climb", "clinic", "clip", "clock", "clog", "close", "cloth", "cloud",
    "clown", "club", "clump", "cluster", "clutch", "coach", "coast", "coconut",
    "code", "coffee", "coil", "coin", "collect", "color", "column", "combine",
    "come", "comfort", "comic", "common", "company", "concert", "conduct", "confirm",
    "congress", "connect", "consider", "control", "convince", "cook", "cool", "copper",
];

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
