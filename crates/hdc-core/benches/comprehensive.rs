//! Comprehensive HDC Benchmark Suite
//!
//! Tests performance across all encoding types, operations, and similarity metrics.
//! Run with: cargo bench --features "std dp" -- --verbose

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use hdc_core::{
    Hypervector, Seed, HYPERVECTOR_DIM, HYPERVECTOR_BYTES,
    encoding::{DnaEncoder, SnpEncoder, HlaEncoder},
};

// ============================================================================
// Hypervector Operations Benchmarks
// ============================================================================

fn bench_hypervector_creation(c: &mut Criterion) {
    let seed = Seed::from_string("bench-hypervector");

    let mut group = c.benchmark_group("hypervector_creation");
    group.throughput(Throughput::Elements(1));

    group.bench_function("random_from_seed", |b| {
        b.iter(|| Hypervector::random(black_box(&seed)))
    });

    group.bench_function("zeros", |b| {
        b.iter(|| Hypervector::zeros())
    });

    group.bench_function("ones", |b| {
        b.iter(|| Hypervector::ones())
    });

    let bytes = vec![0xAA; HYPERVECTOR_BYTES];
    group.bench_function("from_bytes", |b| {
        b.iter(|| Hypervector::from_bytes(black_box(&bytes)))
    });

    group.finish();
}

fn bench_hypervector_operations(c: &mut Criterion) {
    let seed1 = Seed::from_string("bench-op-1");
    let seed2 = Seed::from_string("bench-op-2");
    let v1 = Hypervector::random(&seed1);
    let v2 = Hypervector::random(&seed2);

    let mut group = c.benchmark_group("hypervector_ops");

    group.bench_function("xor_bind", |b| {
        b.iter(|| black_box(&v1).xor(black_box(&v2)))
    });

    group.bench_function("permute_1", |b| {
        b.iter(|| black_box(&v1).permute(1))
    });

    group.bench_function("permute_100", |b| {
        b.iter(|| black_box(&v1).permute(100))
    });

    group.bench_function("popcount", |b| {
        b.iter(|| black_box(&v1).popcount())
    });

    group.bench_function("hamming_distance", |b| {
        b.iter(|| black_box(&v1).hamming_distance(black_box(&v2)))
    });

    group.finish();
}

fn bench_bundling(c: &mut Criterion) {
    let seeds: Vec<_> = (0..100)
        .map(|i| Seed::from_string(&format!("bundle-{}", i)))
        .collect();
    let vectors: Vec<_> = seeds.iter()
        .map(|s| Hypervector::random(s))
        .collect();

    let mut group = c.benchmark_group("bundling");

    for count in [2, 5, 10, 25, 50, 100].iter() {
        let subset: Vec<_> = vectors[0..*count].iter().collect();

        group.bench_with_input(
            BenchmarkId::new("majority_bundle", count),
            &subset,
            |b, vecs| {
                b.iter(|| Hypervector::majority_bundle(black_box(vecs)))
            },
        );
    }

    group.finish();
}

// ============================================================================
// Similarity Metrics Benchmarks
// ============================================================================

fn bench_similarity_metrics(c: &mut Criterion) {
    let seed1 = Seed::from_string("sim-1");
    let seed2 = Seed::from_string("sim-2");
    let v1 = Hypervector::random(&seed1);
    let v2 = Hypervector::random(&seed2);

    let mut group = c.benchmark_group("similarity_metrics");

    group.bench_function("cosine", |b| {
        b.iter(|| black_box(&v1).normalized_cosine_similarity(black_box(&v2)))
    });

    group.bench_function("hamming", |b| {
        b.iter(|| black_box(&v1).hamming_similarity(black_box(&v2)))
    });

    group.bench_function("jaccard", |b| {
        b.iter(|| black_box(&v1).jaccard_similarity(black_box(&v2)))
    });

    group.finish();
}

fn bench_batch_similarity(c: &mut Criterion) {
    let seeds: Vec<_> = (0..500)
        .map(|i| Seed::from_string(&format!("batch-sim-{}", i)))
        .collect();
    let vectors: Vec<_> = seeds.iter()
        .map(|s| Hypervector::random(s))
        .collect();

    let mut group = c.benchmark_group("batch_similarity");

    for count in [10, 50, 100, 250, 500].iter() {
        let n = *count;
        let pairs = n * (n - 1) / 2;
        group.throughput(Throughput::Elements(pairs as u64));

        group.bench_with_input(
            BenchmarkId::new("pairwise_cosine", count),
            &n,
            |b, &n| {
                b.iter(|| {
                    let mut sum = 0.0;
                    for i in 0..n {
                        for j in (i+1)..n {
                            sum += vectors[i].normalized_cosine_similarity(&vectors[j]);
                        }
                    }
                    black_box(sum)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// DNA Encoder Benchmarks
// ============================================================================

fn bench_dna_encoder(c: &mut Criterion) {
    let seed = Seed::from_string("dna-bench");

    let mut group = c.benchmark_group("dna_encoder");

    // Test different k-mer lengths
    for k in [4, 5, 6, 7, 8].iter() {
        let encoder = DnaEncoder::new(seed.clone(), *k);
        let seq = "ATCGATCGATCG".repeat(10); // 120bp

        group.bench_with_input(
            BenchmarkId::new("encode_kmer", k),
            &seq,
            |b, s| b.iter(|| encoder.encode_sequence(black_box(s))),
        );
    }

    group.finish();
}

fn bench_dna_sequence_lengths(c: &mut Criterion) {
    let seed = Seed::from_string("dna-length-bench");
    let encoder = DnaEncoder::new(seed, 6);

    let mut group = c.benchmark_group("dna_sequence_length");

    for length in [50, 100, 250, 500, 1000, 2500, 5000].iter() {
        let seq = "ATCG".repeat(*length / 4);
        group.throughput(Throughput::Bytes(*length as u64));

        group.bench_with_input(
            BenchmarkId::new("encode", length),
            &seq,
            |b, s| b.iter(|| encoder.encode_sequence(black_box(s))),
        );
    }

    group.finish();
}

// ============================================================================
// SNP Encoder Benchmarks
// ============================================================================

fn bench_snp_encoder(c: &mut Criterion) {
    let seed = Seed::from_string("snp-bench");
    let encoder = SnpEncoder::new(seed);

    let mut group = c.benchmark_group("snp_encoder");

    // Generate SNP panels of various sizes
    for count in [10, 50, 100, 500, 1000].iter() {
        let snps: Vec<_> = (0..*count)
            .map(|i| (format!("rs{}", 1000000 + i), 0u8)) // homozygous ref
            .collect();

        let snp_refs: Vec<_> = snps.iter()
            .map(|(id, gt)| (id.as_str(), *gt))
            .collect();

        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(
            BenchmarkId::new("encode_panel", count),
            &snp_refs,
            |b, panel| b.iter(|| encoder.encode_panel(black_box(panel))),
        );
    }

    group.finish();
}

fn bench_snp_genotypes(c: &mut Criterion) {
    let seed = Seed::from_string("snp-genotype-bench");
    let encoder = SnpEncoder::new(seed);

    let mut group = c.benchmark_group("snp_genotypes");

    // Test all three genotype values
    for genotype in [0u8, 1u8, 2u8].iter() {
        let snps: Vec<_> = (0..100)
            .map(|i| (format!("rs{}", 1000000 + i), *genotype))
            .collect();

        let snp_refs: Vec<_> = snps.iter()
            .map(|(id, gt)| (id.as_str(), *gt))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("genotype", genotype),
            &snp_refs,
            |b, panel| b.iter(|| encoder.encode_panel(black_box(panel))),
        );
    }

    group.finish();
}

// ============================================================================
// HLA Encoder Benchmarks
// ============================================================================

fn bench_hla_encoder(c: &mut Criterion) {
    let seed = Seed::from_string("hla-bench");
    let encoder = HlaEncoder::new(seed);

    let mut group = c.benchmark_group("hla_encoder");

    // Common HLA allele combinations
    let allele_sets = [
        vec!["A*01:01", "A*02:01"],
        vec!["A*01:01", "A*02:01", "B*07:02", "B*08:01"],
        vec!["A*01:01", "A*02:01", "B*07:02", "B*08:01", "C*01:02", "C*07:01"],
        vec!["A*01:01", "A*02:01", "B*07:02", "B*08:01", "C*01:02", "C*07:01",
             "DRB1*03:01", "DRB1*04:01"],
        vec!["A*01:01", "A*02:01", "B*07:02", "B*08:01", "C*01:02", "C*07:01",
             "DRB1*03:01", "DRB1*04:01", "DQB1*02:01", "DQB1*03:02"],
    ];

    for alleles in allele_sets.iter() {
        let allele_refs: Vec<&str> = alleles.iter().map(|s| s.as_ref()).collect();

        group.bench_with_input(
            BenchmarkId::new("encode", alleles.len()),
            &allele_refs,
            |b, a| b.iter(|| encoder.encode_hla_typing(black_box(a))),
        );
    }

    group.finish();
}

// ============================================================================
// Confidence Calculation Benchmarks
// ============================================================================

fn bench_confidence(c: &mut Criterion) {
    use hdc_core::SimilarityWithConfidence;

    let seed1 = Seed::from_string("conf-1");
    let seed2 = Seed::from_string("conf-2");
    let v1 = Hypervector::random(&seed1);
    let v2 = Hypervector::random(&seed2);

    let mut group = c.benchmark_group("confidence");

    group.bench_function("calculate_with_confidence", |b| {
        b.iter(|| SimilarityWithConfidence::calculate(black_box(&v1), black_box(&v2)))
    });

    group.finish();
}

// ============================================================================
// Memory Benchmarks
// ============================================================================

fn bench_memory_layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory");

    // Measure creation of many vectors
    for count in [100, 1000, 10000].iter() {
        let seeds: Vec<_> = (0..*count)
            .map(|i| Seed::from_string(&format!("mem-{}", i)))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("create_vectors", count),
            &seeds,
            |b, s| {
                b.iter(|| {
                    let vecs: Vec<_> = s.iter()
                        .map(|seed| Hypervector::random(seed))
                        .collect();
                    black_box(vecs)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Real-world Scenario Benchmarks
// ============================================================================

fn bench_genetic_profile_creation(c: &mut Criterion) {
    let seed = Seed::from_string("profile-bench");
    let dna_encoder = DnaEncoder::new(seed.clone(), 6);
    let snp_encoder = SnpEncoder::new(seed.clone());
    let hla_encoder = HlaEncoder::new(seed);

    // Simulated genetic profile data
    let dna_seq = "ATCGATCG".repeat(125); // 1000bp
    let snps: Vec<_> = (0..100)
        .map(|i| (format!("rs{}", 1000000 + i), (i % 3) as u8))
        .collect();
    let snp_refs: Vec<_> = snps.iter()
        .map(|(id, gt)| (id.as_str(), *gt))
        .collect();
    let hla_alleles = vec!["A*01:01", "A*02:01", "B*07:02", "B*08:01"];

    let mut group = c.benchmark_group("genetic_profile");

    group.bench_function("full_profile_encoding", |b| {
        b.iter(|| {
            let dna = dna_encoder.encode_sequence(black_box(&dna_seq)).unwrap();
            let snp = snp_encoder.encode_panel(black_box(&snp_refs));
            let hla = hla_encoder.encode_hla_typing(black_box(&hla_alleles));

            // Combine into unified profile
            let combined = dna.vector.xor(&snp).xor(&hla);
            black_box(combined)
        })
    });

    group.bench_function("profile_comparison", |b| {
        let profile1 = {
            let dna = dna_encoder.encode_sequence(&dna_seq).unwrap();
            let snp = snp_encoder.encode_panel(&snp_refs);
            let hla = hla_encoder.encode_hla_typing(&hla_alleles);
            dna.vector.xor(&snp).xor(&hla)
        };

        // Slightly different profile
        let snps2: Vec<_> = (0..100)
            .map(|i| (format!("rs{}", 1000000 + i), ((i + 1) % 3) as u8))
            .collect();
        let snp_refs2: Vec<_> = snps2.iter()
            .map(|(id, gt)| (id.as_str(), *gt))
            .collect();

        let profile2 = {
            let dna = dna_encoder.encode_sequence(&dna_seq).unwrap();
            let snp = snp_encoder.encode_panel(&snp_refs2);
            let hla = hla_encoder.encode_hla_typing(&hla_alleles);
            dna.vector.xor(&snp).xor(&hla)
        };

        b.iter(|| {
            black_box(profile1.normalized_cosine_similarity(black_box(&profile2)))
        })
    });

    group.finish();
}

fn bench_population_search(c: &mut Criterion) {
    let seed = Seed::from_string("pop-search-bench");
    let encoder = DnaEncoder::new(seed, 6);

    // Create a "database" of encoded profiles
    let population_sizes = [100, 500, 1000];

    let mut group = c.benchmark_group("population_search");

    for &pop_size in &population_sizes {
        // Generate population
        let population: Vec<_> = (0..pop_size)
            .map(|i| {
                let seq = format!("ATCG{:04}GCTA", i).repeat(25);
                encoder.encode_sequence(&seq).unwrap().vector
            })
            .collect();

        // Query vector
        let query = encoder.encode_sequence(&"ATCGATCG".repeat(12)).unwrap().vector;

        group.throughput(Throughput::Elements(pop_size as u64));

        group.bench_with_input(
            BenchmarkId::new("find_top_10", pop_size),
            &population,
            |b, pop| {
                b.iter(|| {
                    let mut scores: Vec<_> = pop.iter()
                        .enumerate()
                        .map(|(i, v)| (i, query.normalized_cosine_similarity(v)))
                        .collect();
                    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                    black_box(&scores[..10.min(scores.len())])
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Criterion Groups
// ============================================================================

criterion_group!(
    hypervector_benches,
    bench_hypervector_creation,
    bench_hypervector_operations,
    bench_bundling,
);

criterion_group!(
    similarity_benches,
    bench_similarity_metrics,
    bench_batch_similarity,
);

criterion_group!(
    encoder_benches,
    bench_dna_encoder,
    bench_dna_sequence_lengths,
    bench_snp_encoder,
    bench_snp_genotypes,
    bench_hla_encoder,
);

criterion_group!(
    advanced_benches,
    bench_confidence,
    bench_memory_layout,
);

criterion_group!(
    scenario_benches,
    bench_genetic_profile_creation,
    bench_population_search,
);

criterion_main!(
    hypervector_benches,
    similarity_benches,
    encoder_benches,
    advanced_benches,
    scenario_benches,
);
