//! Experiment 7: Pharmacogenomics Validation
//!
//! Validates HDC encoding using CYP450 pharmacogene reference sequences.
//! Tests whether HDC similarity can distinguish drug-metabolizing enzyme variants.

use colored::*;
use hdc_core::{encoding::DnaEncoder, similarity::SimilarityStats, Seed};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Results of pharmacogenomics experiment
#[derive(Serialize, Deserialize)]
pub struct PharmacogenomicsResults {
    pub config: PgxConfig,
    pub within_gene: SimilarityDistribution,
    pub between_genes: SimilarityDistribution,
    pub gene_classification_accuracy: f64,
    pub genes_tested: Vec<GeneStats>,
    pub encoding_time_ms: f64,
    pub comparison_time_ms: f64,
}

#[derive(Serialize, Deserialize)]
pub struct PgxConfig {
    pub total_sequences: usize,
    pub num_genes: usize,
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
pub struct GeneStats {
    pub name: String,
    pub sequence_length: usize,
    pub num_segments: usize,
}

/// Parsed CYP gene from NCBI RefSeq
#[derive(Clone, Debug)]
pub struct CypGene {
    pub name: String,       // e.g., "CYP2D6"
    pub accession: String,  // e.g., "NG_008376.4"
    pub sequence: String,
    pub segments: Vec<String>,  // Gene split into overlapping segments
}

/// Parse NCBI RefSeq FASTA file
fn parse_refseq_fasta(path: &Path) -> Result<CypGene, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut header: Option<String> = None;
    let mut sequence = String::new();

    for line in reader.lines() {
        let line = line?;
        if line.starts_with('>') {
            header = Some(line[1..].to_string());
        } else {
            // Append sequence, removing gaps and whitespace
            sequence.push_str(&line.trim().replace(" ", ""));
        }
    }

    let header = header.ok_or_else(|| std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "No header found in FASTA"
    ))?;

    // Parse header: "NG_008376.4 Homo sapiens cytochrome P450 family 2 subfamily D member 6..."
    let parts: Vec<&str> = header.split_whitespace().collect();
    let accession = parts.first().unwrap_or(&"unknown").to_string();

    // Extract gene name from header (e.g., "CYP2D6" from the description)
    let gene_name = extract_gene_name(&header);

    // Create overlapping segments (simulating different regions/variants)
    let segments = create_gene_segments(&sequence, 1000, 500);

    Ok(CypGene {
        name: gene_name,
        accession,
        sequence: sequence.to_uppercase(),
        segments,
    })
}

/// Extract CYP gene name from FASTA header
fn extract_gene_name(header: &str) -> String {
    // Look for CYP followed by numbers and letters
    let header_upper = header.to_uppercase();
    if let Some(start) = header_upper.find("CYP") {
        let rest = &header_upper[start..];
        let end = rest.find(|c: char| c == ' ' || c == ',' || c == ')' || c == '(')
            .unwrap_or(rest.len());
        let name = &rest[..end];
        // Clean up common suffixes
        name.trim_end_matches(|c: char| !c.is_alphanumeric())
            .to_string()
    } else {
        "UNKNOWN".to_string()
    }
}

/// Create overlapping segments from a gene sequence
fn create_gene_segments(sequence: &str, segment_len: usize, step: usize) -> Vec<String> {
    let mut segments = Vec::new();
    let seq_len = sequence.len();

    if seq_len < segment_len {
        segments.push(sequence.to_string());
        return segments;
    }

    let mut start = 0;
    while start + segment_len <= seq_len {
        segments.push(sequence[start..start + segment_len].to_string());
        start += step;
    }

    // Add final segment if there's remaining sequence
    if start < seq_len && seq_len - start >= segment_len / 2 {
        segments.push(sequence[seq_len - segment_len..].to_string());
    }

    segments
}

pub fn run_pharmacogenomics_experiment(
    data_dir: std::path::PathBuf,
    kmer_length: u8,
    output_dir: std::path::PathBuf,
) {
    use std::time::Instant;

    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    println!("{}", "═".repeat(60).blue());
    println!("{}", "  EXPERIMENT 7: PHARMACOGENOMICS VALIDATION".blue().bold());
    println!("{}", "  Using NCBI RefSeq CYP450 Gene Sequences".blue());
    println!("{}", "═".repeat(60).blue());
    println!();

    // Load CYP FASTA files
    println!("{}", "1. Loading CYP450 gene sequences...".yellow());
    let mut all_genes: Vec<CypGene> = Vec::new();

    let cyp_files = [
        ("CYP2D6_ref.fasta", "CYP2D6"),
        ("CYP2C19_ref.fasta", "CYP2C19"),
        ("CYP2C9_ref.fasta", "CYP2C9"),
        ("CYP3A4_ref.fasta", "CYP3A4"),
    ];

    for (filename, gene_name) in &cyp_files {
        let path = data_dir.join("cyp").join(filename);
        if path.exists() {
            match parse_refseq_fasta(&path) {
                Ok(mut gene) => {
                    gene.name = gene_name.to_string();
                    println!("   {}: {} bp, {} segments",
                             gene_name, gene.sequence.len(), gene.segments.len());
                    all_genes.push(gene);
                }
                Err(e) => {
                    eprintln!("   Warning: Failed to parse {}: {}", filename, e);
                }
            }
        } else {
            println!("   Skipping {} (not found at {:?})", filename, path);
        }
    }

    if all_genes.is_empty() {
        println!("{}", "No CYP gene files found. Please download from NCBI.".red());
        return;
    }

    println!("   Total: {} genes", all_genes.len());
    println!();

    // Encode all segments
    println!("{}", "2. Encoding gene segments...".yellow());
    let start = Instant::now();
    let seed = Seed::from_string("pharmacogenomics-v1");
    let encoder = DnaEncoder::new(seed, kmer_length);

    let mut encoded: Vec<(String, String, hdc_core::encoding::EncodedSequence)> = Vec::new();

    for gene in &all_genes {
        for (i, segment) in gene.segments.iter().enumerate() {
            if let Ok(enc) = encoder.encode_sequence(segment) {
                encoded.push((gene.name.clone(), format!("{}_{}", gene.name, i), enc));
            }
        }
    }
    let encoding_time = start.elapsed();

    println!("   Encoded {} segments in {:.2}s ({:.2}ms/segment)",
             encoded.len(),
             encoding_time.as_secs_f64(),
             encoding_time.as_millis() as f64 / encoded.len().max(1) as f64);

    // Compute similarity distributions
    println!("{}", "3. Computing similarities...".yellow());
    let start = Instant::now();

    let mut within_gene_sims = Vec::new();
    let mut between_gene_sims = Vec::new();

    for i in 0..encoded.len() {
        for j in (i + 1)..encoded.len() {
            let (gene_i, _, enc_i) = &encoded[i];
            let (gene_j, _, enc_j) = &encoded[j];

            let sim = enc_i.vector.normalized_cosine_similarity(&enc_j.vector);

            if gene_i == gene_j {
                within_gene_sims.push(sim);
            } else {
                between_gene_sims.push(sim);
            }
        }
    }
    let comparison_time = start.elapsed();

    // Compute statistics
    let within_stats = SimilarityStats::from_values(&within_gene_sims);
    let between_stats = SimilarityStats::from_values(&between_gene_sims);

    // Check separation
    let good_separation = within_stats.mean > between_stats.mean + 0.05;

    // k-NN accuracy for gene classification
    println!("{}", "4. Computing k-NN accuracy...".yellow());
    let gene_accuracy = compute_knn_accuracy_pgx(&encoded, 1);

    // Print results
    println!();
    println!("{}", "═".repeat(50).green());
    println!("{}", "PHARMACOGENOMICS RESULTS".green().bold());
    println!("{}", "═".repeat(50).green());

    println!();
    println!("Similarity Distributions:");
    println!("  Within gene:    {:.4} ± {:.4} (n={})",
             within_stats.mean, within_stats.std_dev, within_stats.count);
    println!("  Between genes:  {:.4} ± {:.4} (n={})",
             between_stats.mean, between_stats.std_dev, between_stats.count);

    println!();
    let separation = within_stats.mean - between_stats.mean;
    println!("Separation gap: {:.4}", separation);
    println!("Good separation: {}",
             if good_separation { "YES ✓".green() } else { "NO ✗".red() });

    println!();
    println!("k-NN Gene Classification Accuracy (k=1): {:.1}%", gene_accuracy * 100.0);

    println!();
    println!("Timing:");
    println!("  Encoding: {:.2}s", encoding_time.as_secs_f64());
    println!("  Comparisons: {:.2}s", comparison_time.as_secs_f64());

    // Gene-level breakdown
    let gene_stats: Vec<GeneStats> = all_genes.iter()
        .map(|g| GeneStats {
            name: g.name.clone(),
            sequence_length: g.sequence.len(),
            num_segments: g.segments.len(),
        })
        .collect();

    // Save results
    let results = PharmacogenomicsResults {
        config: PgxConfig {
            total_sequences: encoded.len(),
            num_genes: all_genes.len(),
            kmer_length,
            data_source: "NCBI RefSeq".to_string(),
        },
        within_gene: within_stats.clone().into(),
        between_genes: between_stats.clone().into(),
        gene_classification_accuracy: gene_accuracy,
        genes_tested: gene_stats,
        encoding_time_ms: encoding_time.as_millis() as f64,
        comparison_time_ms: comparison_time.as_millis() as f64,
    };

    let output_path = output_dir.join("pharmacogenomics-results.json");
    let file = File::create(&output_path).expect("Failed to create output file");
    serde_json::to_writer_pretty(file, &results).expect("Failed to write results");
    println!();
    println!("Results saved to: {}", output_path.display());

    // Clinical relevance
    println!();
    println!("{}", "─".repeat(50));
    println!("{}", "CLINICAL RELEVANCE".magenta().bold());
    println!("{}", "─".repeat(50));
    println!(
        "HDC encoding of pharmacogenes enables:\n\
         • Privacy-preserving drug metabolism prediction\n\
         • Fast variant lookup without exposing raw sequences\n\
         • Federated pharmacogenomics research\n\
         • Personalized medicine with genetic privacy"
    );
}

fn compute_knn_accuracy_pgx(
    encoded: &[(String, String, hdc_core::encoding::EncodedSequence)],
    k: usize,
) -> f64 {
    let mut correct = 0;
    let total = encoded.len();

    for i in 0..encoded.len() {
        let query_gene = &encoded[i].0;

        let mut sims: Vec<(usize, f64)> = encoded
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(j, (_, _, enc))| (j, encoded[i].2.vector.normalized_cosine_similarity(&enc.vector)))
            .collect();

        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let top_k_correct = sims
            .iter()
            .take(k)
            .filter(|(j, _)| &encoded[*j].0 == query_gene)
            .count();

        if top_k_correct == k {
            correct += 1;
        }
    }

    correct as f64 / total as f64
}
