//! Demo: Learned vs Random Codebook Encoding
//!
//! Run with:
//!   cargo run --example learned_encoding_demo

use hdc_core::encoding::{DnaEncoder, KmerCodebook, LearnedKmerCodebook, LearnedClassifier, MultiScaleEncoder};
use hdc_core::Seed;
use std::time::Instant;

fn main() {
    println!("{}", "=".repeat(60));
    println!("LEARNED HDC CODEBOOK DEMO");
    println!("{}", "=".repeat(60));
    println!();

    let seed = Seed::from_string("benchmark");
    let encoder = DnaEncoder::new(seed.clone(), 6);

    // Test sequences
    let sequences = vec![
        "ACGTACGTACGTACGTACGT",
        "TATAAAAGGCCTAATGCGTA",
        "GCGCGCGCGCGCGCGCGCGC",
        "ATATATATATATATATATAT",
        "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT",
    ];

    // 1. Random codebook
    println!("1. RANDOM CODEBOOK");
    println!("{}", "-".repeat(40));

    let start = Instant::now();
    let random_codebook = KmerCodebook::new(&seed, 6);
    let random_create_time = start.elapsed();

    println!("  Created {} k-mers in {:?}", random_codebook.len(), random_create_time);

    let start = Instant::now();
    let mut random_results = Vec::new();
    for seq in &sequences {
        if let Ok(encoded) = encoder.encode_with_codebook(seq, &random_codebook) {
            random_results.push(encoded);
        }
    }
    let random_encode_time = start.elapsed();

    println!("  Encoded {} sequences in {:?}", random_results.len(), random_encode_time);
    println!();

    // 2. Learned codebook
    println!("2. LEARNED CODEBOOK");
    println!("{}", "-".repeat(40));

    // Try multiple possible paths
    let possible_paths = [
        "research/learned_hdc/models/learned_6mers.json",
        "../research/learned_hdc/models/learned_6mers.json",
        "../../research/learned_hdc/models/learned_6mers.json",
        "/srv/luminous-dynamics/mycelix-health/research/learned_hdc/models/learned_6mers.json",
    ];

    let codebook_path = possible_paths
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|s| *s)
        .unwrap_or(possible_paths[3]);

    let start = Instant::now();
    match LearnedKmerCodebook::load(codebook_path) {
        Ok(learned_codebook) => {
            let learned_load_time = start.elapsed();
            println!(
                "  Loaded {} k-mers (dim={}) in {:?}",
                learned_codebook.len(),
                learned_codebook.source_dimension(),
                learned_load_time
            );

            let start = Instant::now();
            let mut learned_results = Vec::new();
            for seq in &sequences {
                if let Ok(encoded) = encoder.encode_with_learned_codebook(seq, &learned_codebook) {
                    learned_results.push(encoded);
                }
            }
            let learned_encode_time = start.elapsed();

            println!("  Encoded {} sequences in {:?}", learned_results.len(), learned_encode_time);
            println!();

            // 3. Compare encodings
            println!("3. ENCODING COMPARISON");
            println!("{}", "-".repeat(40));

            for (i, seq) in sequences.iter().enumerate() {
                if i < random_results.len() && i < learned_results.len() {
                    let sim = random_results[i]
                        .vector
                        .hamming_similarity(&learned_results[i].vector);
                    println!(
                        "  Seq {} (len={}): random vs learned similarity = {:.3}",
                        i + 1,
                        seq.len(),
                        sim
                    );
                }
            }
            println!();

            // 4. Classification with MLP
            println!("4. MLP CLASSIFICATION");
            println!("{}", "-".repeat(40));

            let mlp_path = codebook_path.replace(".json", "_mlp.json");
            match LearnedClassifier::load(&mlp_path) {
                Ok(classifier) => {
                    println!("  Loaded MLP: {} → {} → {} classes",
                        classifier.input_dim(),
                        256,  // hidden dim
                        classifier.num_classes()
                    );

                    // Test sequences with known classes
                    // Positive: contains TATAAA, Negative: no TATAAA
                    let test_sequences = [
                        ("ACGTACGTATATAAAGCTAGC", "positive (has TATAAA)"),
                        ("GCTAGCTAGCTAGCTAGCTAG", "negative (random)"),
                        ("NNNNNNTATAAANNNNNNNN", "positive (has TATAAA)"),
                        ("ATATATATATATATAT", "negative (no TATAAA)"),
                    ];

                    println!();
                    for (seq, expected) in &test_sequences {
                        if let Ok(encoded) = encoder.encode_with_learned_codebook(seq, &learned_codebook) {
                            let result = classifier.predict(&encoded.vector);
                            let class_name = if result.class == 1 { "positive" } else { "negative" };
                            println!(
                                "  {} → {} ({:.1}% confidence) [expected: {}]",
                                &seq[..20.min(seq.len())],
                                class_name,
                                result.confidence * 100.0,
                                expected
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("  Could not load MLP: {}", e);
                }
            }
            println!();

            // 5. Multi-scale encoding
            println!("5. MULTI-SCALE ENCODING (k=4,6,8)");
            println!("{}", "-".repeat(40));

            let ms_encoder = MultiScaleEncoder::new(seed.clone());
            println!("  Scales: {:?}", ms_encoder.scales());

            let start = Instant::now();
            let mut ms_results = Vec::new();
            for seq in &sequences {
                if let Ok(encoded) = ms_encoder.encode_sequence(seq) {
                    ms_results.push(encoded);
                }
            }
            let ms_encode_time = start.elapsed();

            println!("  Encoded {} sequences in {:?}", ms_results.len(), ms_encode_time);

            if ms_results.len() >= 2 {
                println!("\n  Per-scale k-mer counts:");
                for (i, result) in ms_results.iter().enumerate().take(3) {
                    print!("    Seq {}: ", i + 1);
                    for (k, count) in &result.kmer_counts {
                        print!("k{}={} ", k, count);
                    }
                    println!();
                }

                // Compare similarity using multi-scale vs single-scale
                let ms_sim = ms_results[0].similarity(&ms_results[1]);
                let single_sim = random_results[0].vector.hamming_similarity(&random_results[1].vector);

                println!("\n  Similarity comparison (seq1 vs seq2):");
                println!("    Single-scale (k=6): {:.3}", single_sim);
                println!("    Multi-scale (k=4,6,8): {:.3}", ms_sim);
            }
            println!();

            // 6. Performance summary
            println!("6. PERFORMANCE SUMMARY");
            println!("{}", "-".repeat(40));
            println!("  Random codebook create: {:?}", random_create_time);
            println!("  Learned codebook load:  {:?}", learned_load_time);
            println!("  Random encode (5 seq):  {:?}", random_encode_time);
            println!("  Learned encode (5 seq): {:?}", learned_encode_time);
            println!("  Multi-scale encode (5 seq): {:?}", ms_encode_time);

            let speedup = random_encode_time.as_nanos() as f64 / learned_encode_time.as_nanos().max(1) as f64;
            println!(
                "  Encoding speedup (learned): {:.2}x {}",
                speedup.abs(),
                if speedup > 1.0 { "(random faster)" } else { "(learned faster)" }
            );
        }
        Err(e) => {
            println!("  Error loading learned codebook: {}", e);
            println!("  Make sure to run: python export_learned_embeddings.py");
            println!("  from research/learned_hdc/ directory first");
        }
    }

    println!();
    println!("{}", "=".repeat(60));
    println!("DEMO COMPLETE");
    println!("{}", "=".repeat(60));
}
