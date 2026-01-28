//! Benchmark: Learned vs Random Codebook Encoding
//!
//! Compares encoding performance and memory usage between:
//! - Random k-mer codebook (generated on-the-fly)
//! - Learned k-mer codebook (pre-trained embeddings)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use hdc_core::encoding::{DnaEncoder, KmerCodebook, LearnedKmerCodebook};
use hdc_core::Seed;

/// Generate random DNA sequences for benchmarking
fn generate_sequences(count: usize, length: usize, seed: u64) -> Vec<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let bases = ['A', 'C', 'G', 'T'];
    let mut sequences = Vec::with_capacity(count);

    for i in 0..count {
        let mut hasher = DefaultHasher::new();
        (seed, i).hash(&mut hasher);
        let mut h = hasher.finish();

        let seq: String = (0..length)
            .map(|_| {
                let base = bases[(h % 4) as usize];
                h = h.wrapping_mul(6364136223846793005).wrapping_add(1);
                base
            })
            .collect();
        sequences.push(seq);
    }

    sequences
}

fn bench_random_codebook(c: &mut Criterion) {
    let seed = Seed::from_string("benchmark");
    let encoder = DnaEncoder::new(seed.clone(), 6);
    let codebook = KmerCodebook::new(&seed, 6);

    let sequences = generate_sequences(100, 100, 42);

    c.bench_function("random_codebook_encode_100_seqs", |b| {
        b.iter(|| {
            for seq in &sequences {
                let _ = black_box(encoder.encode_with_codebook(seq, &codebook));
            }
        })
    });
}

fn bench_learned_codebook(c: &mut Criterion) {
    // Try to load the learned codebook
    let codebook_path = "../../research/learned_hdc/models/learned_6mers.json";

    let codebook = match LearnedKmerCodebook::load(codebook_path) {
        Ok(cb) => cb,
        Err(e) => {
            eprintln!("Could not load learned codebook: {}. Skipping benchmark.", e);
            return;
        }
    };

    let seed = Seed::from_string("benchmark");
    let encoder = DnaEncoder::new(seed, 6);

    let sequences = generate_sequences(100, 100, 42);

    c.bench_function("learned_codebook_encode_100_seqs", |b| {
        b.iter(|| {
            for seq in &sequences {
                let _ = black_box(encoder.encode_with_learned_codebook(seq, &codebook));
            }
        })
    });
}

fn bench_codebook_loading(c: &mut Criterion) {
    let seed = Seed::from_string("benchmark");

    c.bench_function("random_codebook_create", |b| {
        b.iter(|| {
            black_box(KmerCodebook::new(&seed, 6))
        })
    });

    let codebook_path = "../../research/learned_hdc/models/learned_6mers.json";

    c.bench_function("learned_codebook_load", |b| {
        b.iter(|| {
            let _ = black_box(LearnedKmerCodebook::load(codebook_path));
        })
    });
}

fn bench_sequence_lengths(c: &mut Criterion) {
    let seed = Seed::from_string("benchmark");
    let encoder = DnaEncoder::new(seed.clone(), 6);
    let random_codebook = KmerCodebook::new(&seed, 6);

    let codebook_path = "../../research/learned_hdc/models/learned_6mers.json";
    let learned_codebook = match LearnedKmerCodebook::load(codebook_path) {
        Ok(cb) => Some(cb),
        Err(_) => None,
    };

    let mut group = c.benchmark_group("sequence_length_scaling");

    for length in [50, 100, 200, 500, 1000].iter() {
        let sequences = generate_sequences(10, *length, 42);

        group.bench_with_input(
            BenchmarkId::new("random", length),
            length,
            |b, _| {
                b.iter(|| {
                    for seq in &sequences {
                        let _ = black_box(encoder.encode_with_codebook(seq, &random_codebook));
                    }
                })
            },
        );

        if let Some(ref learned) = learned_codebook {
            group.bench_with_input(
                BenchmarkId::new("learned", length),
                length,
                |b, _| {
                    b.iter(|| {
                        for seq in &sequences {
                            let _ = black_box(encoder.encode_with_learned_codebook(seq, learned));
                        }
                    })
                },
            );
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_random_codebook,
    bench_learned_codebook,
    bench_codebook_loading,
    bench_sequence_lengths,
);

criterion_main!(benches);
