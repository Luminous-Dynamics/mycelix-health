//! Benchmarks for HDC encoding performance
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use hdc_core::{encoding::DnaEncoder, Seed};

fn bench_encoding(c: &mut Criterion) {
    let seed = Seed::from_string("benchmark-v1");
    let encoder = DnaEncoder::new(seed, 6);

    // Test sequences of varying lengths
    let short = "ATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCG";
    let medium: String = "ATCGATCG".repeat(62);
    let long: String = "ATCGATCG".repeat(125);

    let mut group = c.benchmark_group("dna_encoding");

    group.bench_with_input(BenchmarkId::new("encode", "100bp"), &short, |b, s| {
        b.iter(|| encoder.encode_sequence(black_box(*s)))
    });

    group.bench_with_input(BenchmarkId::new("encode", "500bp"), &medium, |b, s| {
        b.iter(|| encoder.encode_sequence(black_box(s)))
    });

    group.bench_with_input(BenchmarkId::new("encode", "1000bp"), &long, |b, s| {
        b.iter(|| encoder.encode_sequence(black_box(s)))
    });

    group.finish();
}

fn bench_similarity(c: &mut Criterion) {
    let seed = Seed::from_string("benchmark-v1");
    let encoder = DnaEncoder::new(seed, 6);

    let seq1 = "ATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCG";
    let seq2 = "ATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCG";
    let seq3 = "GCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTA";

    let enc1 = encoder.encode_sequence(seq1).unwrap();
    let enc2 = encoder.encode_sequence(seq2).unwrap();
    let enc3 = encoder.encode_sequence(seq3).unwrap();

    c.bench_function("similarity_identical", |b| {
        b.iter(|| enc1.vector.normalized_cosine_similarity(black_box(&enc2.vector)))
    });

    c.bench_function("similarity_different", |b| {
        b.iter(|| enc1.vector.normalized_cosine_similarity(black_box(&enc3.vector)))
    });
}

fn bench_batch_similarity(c: &mut Criterion) {
    let seed = Seed::from_string("benchmark-v1");
    let encoder = DnaEncoder::new(seed, 6);

    // Create 100 encoded sequences with valid DNA only
    let bases = ["ATCG", "GCTA", "TAGC", "CGAT"];
    let sequences: Vec<_> = (0..100)
        .map(|i| format!("{}{}{}", bases[i % 4], bases[(i + 1) % 4], bases[(i + 2) % 4]).repeat(10))
        .collect();

    let encoded: Vec<_> = sequences.iter()
        .map(|s| encoder.encode_sequence(s).unwrap())
        .collect();

    c.bench_function("batch_100_pairwise", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for i in 0..encoded.len() {
                for j in (i+1)..encoded.len() {
                    sum += encoded[i].vector.normalized_cosine_similarity(&encoded[j].vector);
                }
            }
            black_box(sum)
        })
    });
}

criterion_group!(benches, bench_encoding, bench_similarity, bench_batch_similarity);
criterion_main!(benches);
