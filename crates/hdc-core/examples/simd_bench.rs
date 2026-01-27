use hdc_core::{Hypervector, Seed, ops};
use std::time::Instant;

fn main() {
    println!("=== SIMD Benchmark ===\n");
    
    // Check AVX2 availability
    println!("AVX2 available: {}", ops::has_avx2());
    
    let seed = Seed::from_string("benchmark");
    let hv1 = Hypervector::random(&seed, "item1");
    let hv2 = Hypervector::random(&seed, "item2");
    
    // Warm up
    for _ in 0..1000 {
        let _ = hv1.normalized_cosine_similarity(&hv2);
    }
    
    // Benchmark similarity calculation
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = hv1.normalized_cosine_similarity(&hv2);
    }
    let elapsed = start.elapsed();
    
    println!("\nSimilarity benchmark ({} iterations):", iterations);
    println!("  Total time: {:?}", elapsed);
    println!("  Per operation: {:.2} ns", elapsed.as_nanos() as f64 / iterations as f64);
    println!("  Operations/sec: {:.0}", iterations as f64 / elapsed.as_secs_f64());
    
    // Benchmark bind operation
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = hv1.bind(&hv2);
    }
    let elapsed = start.elapsed();
    
    println!("\nBind benchmark ({} iterations):", iterations);
    println!("  Total time: {:?}", elapsed);
    println!("  Per operation: {:.2} ns", elapsed.as_nanos() as f64 / iterations as f64);
    println!("  Operations/sec: {:.0}", iterations as f64 / elapsed.as_secs_f64());
    
    // Benchmark popcount
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = ops::popcount(hv1.as_bytes());
    }
    let elapsed = start.elapsed();
    
    println!("\nPopcount benchmark ({} iterations):", iterations);
    println!("  Total time: {:?}", elapsed);
    println!("  Per operation: {:.2} ns", elapsed.as_nanos() as f64 / iterations as f64);
}
