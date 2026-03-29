// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Extended tests for hdc-core modules: similarity search, confidence scoring,
//! encoding edge cases, batch operations, and cross-module integration.

use hdc_core::{
    DnaEncoder, Hypervector, Seed,
    HYPERVECTOR_DIM,
};
use hdc_core::confidence::{MatchConfidence, SimilarityWithConfidence};
use hdc_core::similarity::HdcIndex;
use hdc_core::batch::{BatchEncoder, BatchConfig};

// ═══════════════════════════════════════════════════════════════════════════════
// Similarity Search — HdcIndex
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_index_empty_search() {
    let index = HdcIndex::new();
    let query = Hypervector::random(&Seed::from_string("query"), "test");
    let results = index.search(&query, 5);
    assert!(results.is_empty(), "Empty index should return no results");
}

#[test]
fn test_index_top_k_ordering() {
    let mut index = HdcIndex::new();
    let seed = Seed::from_string("ordering-test");

    // Add several vectors with known relationships
    let base = Hypervector::random(&seed, "base");
    index.add("base".into(), base.clone());

    // Add a similar vector (small permutation)
    let similar = base.permute(1);
    index.add("similar".into(), similar);

    // Add random vectors
    for i in 0..5 {
        let random = Hypervector::random(&seed, &format!("random-{i}"));
        index.add(format!("random-{i}"), random);
    }

    // Search for the base vector
    let results = index.search(&base, 3);
    assert_eq!(results.len(), 3);

    // First result should be the exact match (highest similarity)
    assert_eq!(results[0].id, "base");
    assert!(results[0].similarity > 0.99, "Self-similarity should be ~1.0");

    // Results should be in descending order
    for i in 1..results.len() {
        assert!(
            results[i].similarity <= results[i - 1].similarity,
            "Results should be sorted descending"
        );
    }
}

#[test]
fn test_index_threshold_search() {
    let mut index = HdcIndex::new();
    let seed = Seed::from_string("threshold-test");

    let query = Hypervector::random(&seed, "query");
    index.add("exact".into(), query.clone());

    for i in 0..10 {
        let random = Hypervector::random(&seed, &format!("r{i}"));
        index.add(format!("r{i}"), random);
    }

    // High threshold should only match the exact copy
    let results = index.search_threshold(&query, 0.95);
    assert!(
        results.len() >= 1,
        "Should find at least the exact match above 0.95"
    );
    assert_eq!(results[0].id, "exact");

    // Low threshold should match more
    let low_results = index.search_threshold(&query, 0.0);
    assert_eq!(low_results.len(), 11, "Threshold 0 should match everything");
}

#[test]
fn test_index_with_capacity() {
    let index = HdcIndex::with_capacity(100);
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Confidence Scoring
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_confidence_levels_at_boundaries() {
    assert_eq!(MatchConfidence::from_similarity(0.90), MatchConfidence::VeryHigh);
    assert_eq!(MatchConfidence::from_similarity(0.85), MatchConfidence::VeryHigh);
    assert_eq!(MatchConfidence::from_similarity(0.84), MatchConfidence::High);
    assert_eq!(MatchConfidence::from_similarity(0.70), MatchConfidence::High);
    assert_eq!(MatchConfidence::from_similarity(0.69), MatchConfidence::Moderate);
    assert_eq!(MatchConfidence::from_similarity(0.58), MatchConfidence::Moderate);
    assert_eq!(MatchConfidence::from_similarity(0.57), MatchConfidence::Low);
    assert_eq!(MatchConfidence::from_similarity(0.52), MatchConfidence::Low);
    assert_eq!(MatchConfidence::from_similarity(0.51), MatchConfidence::VeryLow);
    assert_eq!(MatchConfidence::from_similarity(0.0), MatchConfidence::VeryLow);
}

#[test]
fn test_confidence_probability_ordering() {
    let levels = [
        MatchConfidence::VeryLow,
        MatchConfidence::Low,
        MatchConfidence::Moderate,
        MatchConfidence::High,
        MatchConfidence::VeryHigh,
    ];

    for i in 1..levels.len() {
        assert!(
            levels[i].probability() > levels[i - 1].probability(),
            "Probability should increase with confidence level"
        );
    }
}

#[test]
fn test_confidence_descriptions_non_empty() {
    let levels = [
        MatchConfidence::VeryLow,
        MatchConfidence::Low,
        MatchConfidence::Moderate,
        MatchConfidence::High,
        MatchConfidence::VeryHigh,
    ];

    for level in &levels {
        assert!(!level.description().is_empty());
    }
}

#[test]
fn test_similarity_with_confidence_construction() {
    let seed = Seed::from_string("conf-test");
    let a = Hypervector::random(&seed, "a");
    let b = a.clone();
    let result = SimilarityWithConfidence::compare(&a, &b);

    assert!(result.similarity > 0.99, "Self-similarity should be ~1.0");
    assert_eq!(result.confidence, MatchConfidence::VeryHigh);
    assert!(result.z_score > 0.0, "Z-score for identical vectors should be positive");
    assert!(result.bits_above_random > 0);
}

#[test]
fn test_similarity_with_confidence_random_baseline() {
    let seed = Seed::from_string("random-baseline");
    let a = Hypervector::random(&seed, "x");
    let b = Hypervector::random(&seed, "y");
    let result = SimilarityWithConfidence::compare(&a, &b);

    // Random vectors should have similarity around 0.5
    assert!(
        result.similarity > 0.35 && result.similarity < 0.65,
        "Random similarity should be near 0.5: {:.3}",
        result.similarity
    );
    assert!(
        result.confidence == MatchConfidence::VeryLow || result.confidence == MatchConfidence::Low,
        "Random vectors should have low confidence"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// DNA Encoding Edge Cases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_dna_encoding_short_sequence_error() {
    let seed = Seed::from_string("short-seq");
    let encoder = DnaEncoder::new(seed, 6);

    // Sequence shorter than k-mer length
    let result = encoder.encode_sequence("ACGT");
    assert!(result.is_err(), "Sequence shorter than k should fail");
}

#[test]
fn test_dna_encoding_invalid_nucleotide() {
    let seed = Seed::from_string("invalid-nuc");
    let encoder = DnaEncoder::new(seed, 3);

    let result = encoder.encode_sequence("ACGXYZ");
    assert!(result.is_err(), "Invalid nucleotides should fail");
}

#[test]
fn test_dna_encoding_lowercase_accepted() {
    let seed = Seed::from_string("case-test");
    let encoder = DnaEncoder::new(seed, 3);

    let upper = encoder.encode_sequence("ACGTACGT").unwrap();
    let lower = encoder.encode_sequence("acgtacgt").unwrap();

    let sim = upper.vector.normalized_cosine_similarity(&lower.vector);
    assert!(sim > 0.99, "Case should not affect encoding: sim={sim:.4}");
}

#[test]
fn test_dna_encoding_deterministic() {
    let seed = Seed::from_string("determinism");
    let encoder = DnaEncoder::new(seed, 6);

    let enc1 = encoder.encode_sequence("ACGTACGTACGTACGT").unwrap();
    let enc2 = encoder.encode_sequence("ACGTACGTACGTACGT").unwrap();

    let sim = enc1.vector.normalized_cosine_similarity(&enc2.vector);
    assert!(sim > 0.999, "Same input should produce same output: sim={sim:.6}");
}

#[test]
fn test_dna_encoding_different_sequences_dissimilar() {
    let seed = Seed::from_string("dissimilar");
    let encoder = DnaEncoder::new(seed, 6);

    let enc1 = encoder.encode_sequence("AAAAAAAAAAAAAAAA").unwrap();
    let enc2 = encoder.encode_sequence("CCCCCCCCCCCCCCCC").unwrap();

    let sim = enc1.vector.normalized_cosine_similarity(&enc2.vector);
    assert!(
        sim < 0.7,
        "Completely different sequences should have low similarity: sim={sim:.4}"
    );
}

#[test]
fn test_dna_encoding_similar_sequences_similar() {
    let seed = Seed::from_string("similar");
    let encoder = DnaEncoder::new(seed, 6);

    let enc1 = encoder.encode_sequence("ACGTACGTACGTACGT").unwrap();
    // One mutation: T→A at position 7
    let enc2 = encoder.encode_sequence("ACGTACGAACGTACGT").unwrap();

    let sim = enc1.vector.normalized_cosine_similarity(&enc2.vector);
    assert!(
        sim > 0.5,
        "Sequences with one mutation should still be somewhat similar: sim={sim:.4}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Hypervector Operations
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_hypervector_self_similarity() {
    let seed = Seed::from_string("self-sim");
    let hv = Hypervector::random(&seed, "test");
    let sim = hv.normalized_cosine_similarity(&hv);
    assert!(
        (sim - 1.0).abs() < 1e-6,
        "Self-similarity should be 1.0: {sim}"
    );
}

#[test]
fn test_hypervector_permute_preserves_information() {
    let seed = Seed::from_string("permute");
    let hv = Hypervector::random(&seed, "test");
    let perm = hv.permute(1);

    // Permuted vector should be different
    let sim = hv.normalized_cosine_similarity(&perm);
    assert!(
        sim < 0.7,
        "Permuted vector should be dissimilar: sim={sim:.4}"
    );

    // But permuting back should restore it
    // (cyclic permute by dim-1 should undo permute by 1)
    let restored = perm.permute(HYPERVECTOR_DIM - 1);
    let restore_sim = hv.normalized_cosine_similarity(&restored);
    assert!(
        restore_sim > 0.99,
        "Permute+inverse should restore: sim={restore_sim:.4}"
    );
}

#[test]
fn test_hypervector_xor_self_inverse() {
    let seed = Seed::from_string("xor-inv");
    let a = Hypervector::random(&seed, "a");
    let b = Hypervector::random(&seed, "b");

    // a XOR b XOR b should equal a
    let bound = a.bind(&b);
    let unbound = bound.bind(&b);

    let sim = a.normalized_cosine_similarity(&unbound);
    assert!(
        sim > 0.99,
        "XOR should be self-inverse: sim={sim:.4}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Batch Operations
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_batch_encoder_multiple_sequences() {
    let seed = Seed::from_string("batch-test");
    let config = BatchConfig::default();
    let encoder = BatchEncoder::new(seed, config);

    let sequences: Vec<&str> = vec![
        "ACGTACGTACGTACGT",
        "TTTTTTTTTTTTTTTT",
        "GGGGGGGGGGGGGGGG",
    ];

    let result = encoder.encode_sequences(&sequences).unwrap();
    assert_eq!(result.success_count(), 3);
    assert_eq!(result.total_count(), 3);
}

#[test]
fn test_batch_encoder_with_invalid_sequences() {
    let seed = Seed::from_string("batch-invalid");
    let config = BatchConfig::default().with_skip_invalid(true);
    let encoder = BatchEncoder::new(seed, config);

    let sequences: Vec<&str> = vec![
        "ACGTACGTACGTACGT", // Valid
        "XYZ",              // Invalid nucleotides (too short + invalid chars)
        "GGGGGGGGGGGGGGGG", // Valid
    ];

    let result = encoder.encode_sequences(&sequences).unwrap();
    assert_eq!(result.total_count(), 3);
    assert!(result.success_count() >= 2, "At least 2 should succeed");
}

#[test]
fn test_batch_similarity_matrix() {
    let seed = Seed::from_string("matrix-test");
    let config = BatchConfig::default();
    let encoder = BatchEncoder::new(seed, config);

    let sequences: Vec<&str> = vec![
        "ACGTACGTACGTACGT",
        "ACGTACGTACGTACGT", // Duplicate
        "TTTTTTTTTTTTTTTT", // Different
    ];

    let vectors = encoder.encode_to_vectors(&sequences).unwrap();
    let matrix = encoder.pairwise_similarity(&vectors);

    // Diagonal should be ~1.0
    for i in 0..matrix.size() {
        let self_sim = matrix.get(i, i);
        assert!(
            self_sim > 0.99,
            "Diagonal[{i}] should be ~1.0: {self_sim:.4}"
        );
    }

    // Identical sequences should have ~1.0 similarity
    let dup_sim = matrix.get(0, 1);
    assert!(
        dup_sim > 0.99,
        "Duplicate sequences should have ~1.0 similarity: {dup_sim:.4}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Cross-module: Encoding → Search → Confidence
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_encode_index_search_confidence_pipeline() {
    let seed = Seed::from_string("pipeline-test");
    let encoder = DnaEncoder::new(seed, 6);

    // Build a database of sequences
    let db_sequences = [
        ("patient_a", "ACGTACGTACGTACGTACGTACGT"),
        ("patient_b", "TTTTTTTTTTTTTTTTTTTTTTTT"),
        ("patient_c", "ACGTACGTACGTACGTACGTACGT"), // Same as patient_a
    ];

    let mut index = HdcIndex::new();
    for (id, seq) in &db_sequences {
        let encoded = encoder.encode_sequence(seq).unwrap();
        index.add(id.to_string(), encoded.vector);
    }

    // Query with patient_a's sequence
    let query = encoder.encode_sequence("ACGTACGTACGTACGTACGTACGT").unwrap();
    let results = index.search(&query.vector, 3);

    // Should find patient_a and patient_c as top matches
    assert_eq!(results.len(), 3);
    let top_ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    assert!(top_ids.contains(&"patient_a") || top_ids.contains(&"patient_c"));

    // Top result should have very high confidence
    let top_confidence = MatchConfidence::from_similarity(results[0].similarity);
    assert_eq!(top_confidence, MatchConfidence::VeryHigh);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Seed determinism
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_seed_from_string_deterministic() {
    let s1 = Seed::from_string("test-seed");
    let s2 = Seed::from_string("test-seed");
    assert_eq!(s1, s2, "Same string should produce same seed");
}

#[test]
fn test_seed_from_different_strings_different() {
    let s1 = Seed::from_string("seed-a");
    let s2 = Seed::from_string("seed-b");
    assert_ne!(s1, s2, "Different strings should produce different seeds");
}

#[test]
fn test_different_seeds_produce_different_vectors() {
    let s1 = Seed::from_string("alpha");
    let s2 = Seed::from_string("beta");

    let v1 = Hypervector::random(&s1, "same-label");
    let v2 = Hypervector::random(&s2, "same-label");

    let sim = v1.normalized_cosine_similarity(&v2);
    assert!(
        sim < 0.7,
        "Different seeds should produce dissimilar vectors: sim={sim:.4}"
    );
}
