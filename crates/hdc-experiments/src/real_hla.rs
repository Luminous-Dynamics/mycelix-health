//! Experiment 6: Real HLA Validation
//!
//! Validates HDC encoding using real IMGT/HLA reference allele sequences.
//! Tests whether HDC similarity correlates with HLA allele field-level matching.

use colored::*;
use hdc_core::{encoding::DnaEncoder, similarity::SimilarityStats, Seed};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Results of real HLA experiment
#[derive(Serialize, Deserialize)]
pub struct RealHlaResults {
    pub config: RealHlaConfig,
    pub same_two_field: SimilarityDistribution,
    pub same_locus: SimilarityDistribution,
    pub different_locus: SimilarityDistribution,
    pub field_matching_accuracy: f64,
    pub locus_classification_accuracy: f64,
    pub loci_tested: Vec<LocusStats>,
    pub encoding_time_ms: f64,
    pub comparison_time_ms: f64,
}

#[derive(Serialize, Deserialize)]
pub struct RealHlaConfig {
    pub total_alleles: usize,
    pub num_loci: usize,
    pub kmer_length: u8,
    pub data_source: String,
}

#[derive(Serialize, Deserialize)]
pub struct SimilarityDistribution {
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub count: usize,
}

impl From<SimilarityStats> for SimilarityDistribution {
    fn from(stats: SimilarityStats) -> Self {
        Self {
            mean: stats.mean,
            std_dev: stats.std_dev,
            min: stats.min,
            max: stats.max,
            count: stats.count,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct LocusStats {
    pub name: String,
    pub num_alleles: usize,
}

/// Parsed HLA allele from IMGT format
#[derive(Clone, Debug)]
pub struct HlaAllele {
    pub name: String,       // e.g., "A*01:01:01:01"
    pub locus: String,      // e.g., "A"
    pub two_field: String,  // e.g., "A*01:01"
    pub sequence: String,
}

/// Parse IMGT/HLA FASTA file
fn parse_imgt_hla_fasta(path: &Path) -> Result<Vec<HlaAllele>, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut alleles = Vec::new();
    let mut current_header: Option<String> = None;
    let mut current_seq = String::new();

    for line in reader.lines() {
        let line = line?;
        if line.starts_with('>') {
            if let Some(header) = current_header.take() {
                if let Some(allele) = parse_hla_header(&header, &current_seq) {
                    alleles.push(allele);
                }
            }
            current_header = Some(line[1..].to_string());
            current_seq.clear();
        } else {
            // Append sequence, removing gaps and whitespace
            current_seq.push_str(&line.trim().replace(" ", ""));
        }
    }

    // Don't forget the last sequence
    if let Some(header) = current_header {
        if let Some(allele) = parse_hla_header(&header, &current_seq) {
            alleles.push(allele);
        }
    }

    Ok(alleles)
}

fn parse_hla_header(header: &str, sequence: &str) -> Option<HlaAllele> {
    // Skip short sequences
    if sequence.is_empty() || sequence.len() < 200 {
        return None;
    }

    // IMGT format: "HLA:HLA00001 A*01:01:01:01 3503 bp"
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let allele_name = parts[1].to_string();

    // Extract locus (before *)
    let locus = allele_name.split('*').next()
        .unwrap_or("unknown")
        .to_string();

    // Extract two-field resolution (e.g., "A*01:01" from "A*01:01:01:01")
    let fields: Vec<&str> = allele_name.split(':').collect();
    let two_field = if fields.len() >= 2 {
        format!("{}:{}", fields[0], fields[1])
    } else {
        allele_name.clone()
    };

    Some(HlaAllele {
        name: allele_name,
        locus,
        two_field,
        sequence: sequence.to_uppercase(),
    })
}

pub fn run_real_hla_experiment(
    data_dir: std::path::PathBuf,
    kmer_length: u8,
    output_dir: std::path::PathBuf,
) {
    use std::time::Instant;

    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    println!("{}", "═".repeat(60).blue());
    println!("{}", "  EXPERIMENT 6: REAL HLA VALIDATION".blue().bold());
    println!("{}", "  Using IMGT/HLA Reference Allele Sequences".blue());
    println!("{}", "═".repeat(60).blue());
    println!();

    // Load HLA FASTA files
    println!("{}", "1. Loading real HLA sequences...".yellow());
    let mut all_alleles: Vec<HlaAllele> = Vec::new();

    let hla_files = [
        ("hla_a_nuc.fasta", "HLA-A"),
        ("hla_b_nuc.fasta", "HLA-B"),
        ("hla_drb1_nuc.fasta", "HLA-DRB1"),
    ];

    for (filename, locus_name) in &hla_files {
        let path = data_dir.join("hla").join(filename);
        if path.exists() {
            match parse_imgt_hla_fasta(&path) {
                Ok(alleles) => {
                    let count = alleles.len();
                    all_alleles.extend(alleles);
                    println!("   {}: {} alleles", locus_name, count);
                }
                Err(e) => {
                    eprintln!("   Warning: Failed to parse {}: {}", filename, e);
                }
            }
        } else {
            println!("   Skipping {} (not found at {:?})", filename, path);
        }
    }

    println!("   Total: {} alleles", all_alleles.len());

    // Limit to 200 alleles per locus for reasonable runtime
    let max_per_locus = 50;
    let mut locus_counts: HashMap<String, usize> = HashMap::new();
    let limited_alleles: Vec<HlaAllele> = all_alleles.into_iter()
        .filter(|a| {
            let count = locus_counts.entry(a.locus.clone()).or_insert(0);
            if *count < max_per_locus {
                *count += 1;
                true
            } else {
                false
            }
        })
        .collect();

    println!("   Using {} alleles (max {} per locus)", limited_alleles.len(), max_per_locus);

    // Compute statistics
    let locus_set: std::collections::HashSet<_> = limited_alleles.iter()
        .map(|a| a.locus.clone())
        .collect();
    let two_field_set: std::collections::HashSet<_> = limited_alleles.iter()
        .map(|a| a.two_field.clone())
        .collect();

    println!("   Loci: {}, Two-field groups: {}", locus_set.len(), two_field_set.len());
    println!();

    // Encode sequences
    println!("{}", "2. Encoding HLA sequences...".yellow());
    let start = Instant::now();
    let seed = Seed::from_string("real-hla-v1");
    let encoder = DnaEncoder::new(seed, kmer_length);

    let encoded: Vec<_> = limited_alleles.iter()
        .filter_map(|allele| {
            encoder.encode_sequence(&allele.sequence).ok().map(|enc| {
                (allele.clone(), enc)
            })
        })
        .collect();
    let encoding_time = start.elapsed();

    println!("   Encoded {} alleles in {:.2}s ({:.2}ms/allele)",
             encoded.len(),
             encoding_time.as_secs_f64(),
             encoding_time.as_millis() as f64 / encoded.len().max(1) as f64);

    // Compute similarity distributions
    println!("{}", "3. Computing similarities...".yellow());
    let start = Instant::now();

    let mut same_two_field_sims = Vec::new();
    let mut same_locus_sims = Vec::new();
    let mut different_locus_sims = Vec::new();

    for i in 0..encoded.len() {
        for j in (i + 1)..encoded.len() {
            let (allele_i, enc_i) = &encoded[i];
            let (allele_j, enc_j) = &encoded[j];

            let sim = enc_i.vector.normalized_cosine_similarity(&enc_j.vector);

            if allele_i.two_field == allele_j.two_field {
                same_two_field_sims.push(sim);
            } else if allele_i.locus == allele_j.locus {
                same_locus_sims.push(sim);
            } else {
                different_locus_sims.push(sim);
            }
        }
    }
    let comparison_time = start.elapsed();

    // Compute statistics
    let two_field_stats = SimilarityStats::from_values(&same_two_field_sims);
    let locus_stats = SimilarityStats::from_values(&same_locus_sims);
    let different_stats = SimilarityStats::from_values(&different_locus_sims);

    // Check monotonic separation
    let monotonic = two_field_stats.mean > locus_stats.mean
        && locus_stats.mean > different_stats.mean;

    // k-NN accuracy for locus classification
    println!("{}", "4. Computing k-NN accuracy...".yellow());
    let locus_accuracy = compute_knn_accuracy_hla(&encoded, |a| &a.locus, 1);
    let two_field_accuracy = compute_knn_accuracy_hla(&encoded, |a| &a.two_field, 1);

    // Print results
    println!();
    println!("{}", "═".repeat(50).green());
    println!("{}", "REAL HLA RESULTS".green().bold());
    println!("{}", "═".repeat(50).green());

    println!();
    println!("Similarity Distributions (Real IMGT/HLA Data):");
    println!("  Same two-field:    {:.4} ± {:.4} (n={})",
             two_field_stats.mean, two_field_stats.std_dev, two_field_stats.count);
    println!("  Same locus:        {:.4} ± {:.4} (n={})",
             locus_stats.mean, locus_stats.std_dev, locus_stats.count);
    println!("  Different locus:   {:.4} ± {:.4} (n={})",
             different_stats.mean, different_stats.std_dev, different_stats.count);

    println!();
    let sep_two_field_locus = two_field_stats.mean - locus_stats.mean;
    let sep_locus_different = locus_stats.mean - different_stats.mean;
    println!("Separation gaps:");
    println!("  Two-field → Same locus: {:.4}", sep_two_field_locus);
    println!("  Same locus → Different: {:.4}", sep_locus_different);

    println!();
    println!("Monotonic separation: {}",
             if monotonic { "YES ✓".green() } else { "NO ✗".red() });

    println!();
    println!("k-NN Accuracy (k=1):");
    println!("  Locus:     {:.1}%", locus_accuracy * 100.0);
    println!("  Two-field: {:.1}%", two_field_accuracy * 100.0);

    println!();
    println!("Timing:");
    println!("  Encoding: {:.2}s", encoding_time.as_secs_f64());
    println!("  Comparisons: {:.2}s", comparison_time.as_secs_f64());

    // Locus-level breakdown
    let mut locus_counts_final: HashMap<String, usize> = HashMap::new();
    for (allele, _) in &encoded {
        *locus_counts_final.entry(allele.locus.clone()).or_insert(0) += 1;
    }

    let locus_stats_vec: Vec<LocusStats> = locus_counts_final.iter()
        .map(|(name, count)| LocusStats {
            name: name.clone(),
            num_alleles: *count,
        })
        .collect();

    // Save results
    let results = RealHlaResults {
        config: RealHlaConfig {
            total_alleles: encoded.len(),
            num_loci: locus_set.len(),
            kmer_length,
            data_source: "IMGT/HLA".to_string(),
        },
        same_two_field: two_field_stats.clone().into(),
        same_locus: locus_stats.clone().into(),
        different_locus: different_stats.clone().into(),
        field_matching_accuracy: two_field_accuracy,
        locus_classification_accuracy: locus_accuracy,
        loci_tested: locus_stats_vec,
        encoding_time_ms: encoding_time.as_millis() as f64,
        comparison_time_ms: comparison_time.as_millis() as f64,
    };

    let output_path = output_dir.join("real-hla-results.json");
    let file = File::create(&output_path).expect("Failed to create output file");
    serde_json::to_writer_pretty(file, &results).expect("Failed to write results");
    println!();
    println!("Results saved to: {}", output_path.display());

    // Publishable claim
    println!();
    println!("{}", "─".repeat(50));
    println!("{}", "PUBLISHABLE CLAIM".cyan().bold());
    println!("{}", "─".repeat(50));

    if monotonic && locus_accuracy > 0.90 {
        println!(
            "\"Using {} real HLA allele sequences from IMGT/HLA across\n\
             {} loci (HLA-A, HLA-B, DRB1),\n\
             HDC encoding achieves {:.0}% locus classification accuracy\n\
             with clear monotonic separation:\n\
             same-two-field ({:.3}) > same-locus ({:.3}) > different-locus ({:.3}).\"",
            encoded.len(), locus_set.len(),
            locus_accuracy * 100.0,
            two_field_stats.mean, locus_stats.mean, different_stats.mean
        );
    } else {
        println!(
            "Real HLA validation: {} alleles, {} loci\n\
             Locus accuracy: {:.1}%, Two-field accuracy: {:.1}%\n\
             Monotonic: {}",
            encoded.len(), locus_set.len(),
            locus_accuracy * 100.0, two_field_accuracy * 100.0,
            if monotonic { "Yes" } else { "No" }
        );
    }

    // Clinical relevance
    println!();
    println!("{}", "─".repeat(50));
    println!("{}", "CLINICAL RELEVANCE".magenta().bold());
    println!("{}", "─".repeat(50));
    println!(
        "HDC encoding of HLA alleles enables:\n\
         • Privacy-preserving donor matching (no raw sequence exposure)\n\
         • Fast similarity search across millions of donors\n\
         • Approximate matching for partial HLA typing data\n\
         • Secure multi-party computation for transplant registries"
    );
}

fn compute_knn_accuracy_hla<'a, F, T>(
    encoded: &'a [(HlaAllele, hdc_core::encoding::EncodedSequence)],
    get_label: F,
    k: usize,
) -> f64
where
    F: Fn(&'a HlaAllele) -> T,
    T: PartialEq + 'a,
{
    let mut correct = 0;
    let total = encoded.len();

    for i in 0..encoded.len() {
        let query_label = get_label(&encoded[i].0);

        let mut sims: Vec<(usize, f64)> = encoded
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(j, (_, enc))| (j, encoded[i].1.vector.normalized_cosine_similarity(&enc.vector)))
            .collect();

        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let top_k_correct = sims
            .iter()
            .take(k)
            .filter(|(j, _)| get_label(&encoded[*j].0) == query_label)
            .count();

        if top_k_correct == k {
            correct += 1;
        }
    }

    correct as f64 / total as f64
}
