//! Experiment 1: Taxonomy Sanity Check
//!
//! Tests whether HDC similarity respects biological taxonomy.
//! Same-species pairs should have higher similarity than different-species.

use colored::*;
use hdc_core::{encoding::DnaEncoder, similarity::SimilarityStats, Seed};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Instant;

/// Synthetic specimen for testing
#[derive(Clone, Debug)]
struct Specimen {
    id: String,
    sequence: String,
    species: String,
    genus: String,
    family: String,
}

/// JSON structure for real COI sequences from BOLD
#[derive(Deserialize)]
struct RealCoiData {
    source: String,
    retrieval_date: String,
    marker: String,
    sequences: Vec<RealSequence>,
}

#[derive(Deserialize)]
struct RealSequence {
    id: String,
    species: String,
    genus: String,
    family: String,
    order: String,
    accession: Option<String>,
    sequence: String,
}

/// Results of taxonomy experiment
#[derive(Serialize, Deserialize)]
pub struct TaxonomyResults {
    pub config: TaxonomyConfig,
    pub within_species: SimilarityDistribution,
    pub within_genus: SimilarityDistribution,
    pub within_family: SimilarityDistribution,
    pub random_pairs: SimilarityDistribution,
    pub monotonic_separation: bool,
    pub species_top1_accuracy: f64,
    pub genus_top1_accuracy: f64,
    pub encoding_time_ms: f64,
    pub comparison_time_ms: f64,
}

#[derive(Serialize, Deserialize)]
pub struct TaxonomyConfig {
    pub sequences_per_species: usize,
    pub kmer_length: u8,
    pub total_specimens: usize,
    pub num_species: usize,
    pub num_genera: usize,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SimilarityDistribution {
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub count: usize,
}

impl From<SimilarityStats> for SimilarityDistribution {
    fn from(stats: SimilarityStats) -> Self {
        SimilarityDistribution {
            mean: stats.mean,
            std_dev: stats.std_dev,
            min: stats.min,
            max: stats.max,
            count: stats.count,
        }
    }
}

/// Generate synthetic specimens with taxonomic structure
fn generate_specimens(
    sequences_per_species: usize,
    rng: &mut ChaCha8Rng,
) -> Vec<Specimen> {
    let taxonomy = vec![
        // Family -> Genus -> Species
        ("Canidae", "Canis", vec!["lupus", "familiaris", "latrans"]),
        ("Canidae", "Vulpes", vec!["vulpes", "lagopus", "zerda"]),
        ("Felidae", "Felis", vec!["catus", "silvestris", "margarita"]),
        ("Felidae", "Panthera", vec!["leo", "tigris", "pardus"]),
        ("Ursidae", "Ursus", vec!["arctos", "americanus", "maritimus"]),
    ];

    let nucleotides = ['A', 'C', 'G', 'T'];
    let seq_length = 650; // COI barcode length
    let mut specimens = Vec::new();
    let mut id_counter = 0;

    for (family, genus, species_list) in &taxonomy {
        // Generate base sequence for genus (shared evolutionary history)
        let genus_base: String = (0..seq_length)
            .map(|_| nucleotides[rng.gen_range(0..4)])
            .collect();

        for species in species_list {
            // Generate species-specific variant (5% divergence from genus)
            let species_base: String = genus_base
                .chars()
                .map(|c| {
                    if rng.gen::<f64>() < 0.05 {
                        nucleotides[rng.gen_range(0..4)]
                    } else {
                        c
                    }
                })
                .collect();

            // Generate individual specimens (2% intraspecific variation)
            for _ in 0..sequences_per_species {
                let sequence: String = species_base
                    .chars()
                    .map(|c| {
                        if rng.gen::<f64>() < 0.02 {
                            nucleotides[rng.gen_range(0..4)]
                        } else {
                            c
                        }
                    })
                    .collect();

                specimens.push(Specimen {
                    id: format!("SPEC{:06}", id_counter),
                    sequence,
                    species: format!("{} {}", genus, species),
                    genus: genus.to_string(),
                    family: family.to_string(),
                });
                id_counter += 1;
            }
        }
    }

    specimens
}

pub fn run_taxonomy_experiment(
    sequences_per_species: usize,
    kmer_length: u8,
    output_dir: PathBuf,
) {
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    println!("Configuration:");
    println!("  Sequences per species: {}", sequences_per_species);
    println!("  K-mer length: {}", kmer_length);
    println!();

    // Generate specimens
    println!("{}", "1. Generating specimens...".yellow());
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let specimens = generate_specimens(sequences_per_species, &mut rng);
    println!("   Generated {} specimens", specimens.len());

    let num_species: usize = specimens.iter()
        .map(|s| s.species.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let num_genera: usize = specimens.iter()
        .map(|s| s.genus.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();
    println!("   {} species, {} genera", num_species, num_genera);

    // Encode all specimens
    println!("{}", "2. Encoding sequences...".yellow());
    let start = Instant::now();
    let seed = Seed::from_string("taxonomy-experiment-v1");
    let encoder = DnaEncoder::new(seed, kmer_length);

    let encoded: Vec<_> = specimens
        .iter()
        .filter_map(|spec| {
            encoder.encode_sequence(&spec.sequence).ok().map(|enc| (spec, enc))
        })
        .collect();
    let encoding_time = start.elapsed();
    println!("   Encoded {} sequences in {:.2}s", encoded.len(), encoding_time.as_secs_f64());

    // Compute similarity distributions
    println!("{}", "3. Computing similarities...".yellow());
    let start = Instant::now();

    let mut within_species_sims = Vec::new();
    let mut within_genus_sims = Vec::new();
    let mut within_family_sims = Vec::new();
    let mut random_sims = Vec::new();

    for i in 0..encoded.len() {
        for j in (i + 1)..encoded.len() {
            let (spec_i, enc_i) = &encoded[i];
            let (spec_j, enc_j) = &encoded[j];

            let sim = enc_i.vector.normalized_cosine_similarity(&enc_j.vector);

            if spec_i.species == spec_j.species {
                within_species_sims.push(sim);
            } else if spec_i.genus == spec_j.genus {
                within_genus_sims.push(sim);
            } else if spec_i.family == spec_j.family {
                within_family_sims.push(sim);
            } else {
                random_sims.push(sim);
            }
        }
    }
    let comparison_time = start.elapsed();

    // Compute statistics
    let species_stats = SimilarityStats::from_values(&within_species_sims);
    let genus_stats = SimilarityStats::from_values(&within_genus_sims);
    let family_stats = SimilarityStats::from_values(&within_family_sims);
    let random_stats = SimilarityStats::from_values(&random_sims);

    // Check monotonic separation
    let monotonic = species_stats.mean > genus_stats.mean
        && genus_stats.mean > family_stats.mean
        && family_stats.mean >= random_stats.mean;

    // Compute k-NN accuracy
    println!("{}", "4. Computing k-NN accuracy...".yellow());
    let species_accuracy = compute_knn_accuracy(&encoded, |s| &s.species, 1);
    let genus_accuracy = compute_knn_accuracy(&encoded, |s| &s.genus, 1);

    // Print results
    println!();
    println!("{}", "═".repeat(50).green());
    println!("{}", "RESULTS".green().bold());
    println!("{}", "═".repeat(50).green());

    println!();
    println!("Similarity Distributions:");
    println!("  Same species:  {:.3} ± {:.3} (n={})",
             species_stats.mean, species_stats.std_dev, species_stats.count);
    println!("  Same genus:    {:.3} ± {:.3} (n={})",
             genus_stats.mean, genus_stats.std_dev, genus_stats.count);
    println!("  Same family:   {:.3} ± {:.3} (n={})",
             family_stats.mean, family_stats.std_dev, family_stats.count);
    println!("  Random pairs:  {:.3} ± {:.3} (n={})",
             random_stats.mean, random_stats.std_dev, random_stats.count);

    println!();
    println!("Monotonic separation: {}",
             if monotonic { "YES ✓".green() } else { "NO ✗".red() });

    println!();
    println!("k-NN Accuracy (k=1):");
    println!("  Species: {:.1}%", species_accuracy * 100.0);
    println!("  Genus:   {:.1}%", genus_accuracy * 100.0);

    println!();
    println!("Timing:");
    println!("  Encoding: {:.2}s ({:.2}ms/seq)",
             encoding_time.as_secs_f64(),
             encoding_time.as_millis() as f64 / encoded.len() as f64);
    println!("  Comparisons: {:.2}s", comparison_time.as_secs_f64());

    // Save results
    let results = TaxonomyResults {
        config: TaxonomyConfig {
            sequences_per_species,
            kmer_length,
            total_specimens: encoded.len(),
            num_species,
            num_genera,
        },
        within_species: species_stats.clone().into(),
        within_genus: genus_stats.clone().into(),
        within_family: family_stats.clone().into(),
        random_pairs: random_stats.clone().into(),
        monotonic_separation: monotonic,
        species_top1_accuracy: species_accuracy,
        genus_top1_accuracy: genus_accuracy,
        encoding_time_ms: encoding_time.as_millis() as f64,
        comparison_time_ms: comparison_time.as_millis() as f64,
    };

    let output_path = output_dir.join("taxonomy-results.json");
    let file = File::create(&output_path).expect("Failed to create output file");
    serde_json::to_writer_pretty(file, &results).expect("Failed to write results");
    println!();
    println!("Results saved to: {}", output_path.display());

    // Publishable claim
    println!();
    println!("{}", "─".repeat(50));
    println!("{}", "PUBLISHABLE CLAIM".cyan().bold());
    println!("{}", "─".repeat(50));
    if monotonic {
        println!(
            "\"HDC encoding preserves taxonomic structure:\n\
             same-species similarity ({:.2}) > same-genus ({:.2}) >\n\
             same-family ({:.2}) ≥ random ({:.2}),\n\
             with {:.0}% species and {:.0}% genus classification accuracy.\"",
            species_stats.mean, genus_stats.mean, family_stats.mean, random_stats.mean,
            species_accuracy * 100.0, genus_accuracy * 100.0
        );
    } else {
        println!("{}", "WARNING: Monotonic separation not achieved!".red());
    }
}

fn compute_knn_accuracy<F>(
    encoded: &[(&Specimen, hdc_core::encoding::EncodedSequence)],
    get_label: F,
    k: usize,
) -> f64
where
    F: Fn(&Specimen) -> &String,
{
    let mut correct = 0;
    let total = encoded.len();

    for i in 0..encoded.len() {
        let (query_spec, query_enc) = &encoded[i];
        let query_label = get_label(query_spec);

        // Find k nearest neighbors (excluding self)
        let mut sims: Vec<(usize, f64)> = encoded
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(j, (_, enc))| (j, query_enc.vector.normalized_cosine_similarity(&enc.vector)))
            .collect();

        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Check if top-k share the same label
        let top_k_correct = sims
            .iter()
            .take(k)
            .filter(|(j, _)| get_label(encoded[*j].0) == query_label)
            .count();

        if top_k_correct == k {
            correct += 1;
        }
    }

    correct as f64 / total as f64
}

pub fn run_parameter_sweep(
    kmer_lengths: &[u8],
    sequences_per_species: usize,
    output_dir: PathBuf,
) {
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    println!("{}", "PARAMETER SWEEP".yellow().bold());
    println!("Testing k-mer lengths: {:?}", kmer_lengths);
    println!();

    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let specimens = generate_specimens(sequences_per_species, &mut rng);

    #[derive(Serialize)]
    struct SweepResult {
        kmer_length: u8,
        species_mean: f64,
        random_mean: f64,
        separation: f64,
        species_accuracy: f64,
    }

    let mut results = Vec::new();

    for &kmer in kmer_lengths {
        let seed = Seed::from_string(&format!("sweep-k{}", kmer));
        let encoder = DnaEncoder::new(seed, kmer);

        let encoded: Vec<_> = specimens
            .iter()
            .filter_map(|spec| {
                encoder.encode_sequence(&spec.sequence).ok().map(|enc| (spec, enc))
            })
            .collect();

        let mut species_sims = Vec::new();
        let mut random_sims = Vec::new();

        for i in 0..encoded.len().min(100) {
            for j in (i + 1)..encoded.len().min(100) {
                let (spec_i, enc_i) = &encoded[i];
                let (spec_j, enc_j) = &encoded[j];
                let sim = enc_i.vector.normalized_cosine_similarity(&enc_j.vector);

                if spec_i.species == spec_j.species {
                    species_sims.push(sim);
                } else if spec_i.family != spec_j.family {
                    random_sims.push(sim);
                }
            }
        }

        let species_mean = species_sims.iter().sum::<f64>() / species_sims.len().max(1) as f64;
        let random_mean = random_sims.iter().sum::<f64>() / random_sims.len().max(1) as f64;
        let separation = species_mean - random_mean;

        let species_accuracy = compute_knn_accuracy(&encoded, |s| &s.species, 1);

        println!(
            "k={:2}: species={:.3}, random={:.3}, sep={:.3}, acc={:.1}%",
            kmer, species_mean, random_mean, separation, species_accuracy * 100.0
        );

        results.push(SweepResult {
            kmer_length: kmer,
            species_mean,
            random_mean,
            separation,
            species_accuracy,
        });
    }

    // Find optimal
    let best_sep = results.iter().max_by(|a, b| a.separation.partial_cmp(&b.separation).unwrap());
    let best_acc = results.iter().max_by(|a, b| a.species_accuracy.partial_cmp(&b.species_accuracy).unwrap());

    println!();
    println!("Optimal k-mer for separation: {}", best_sep.map(|r| r.kmer_length).unwrap_or(6));
    println!("Optimal k-mer for accuracy: {}", best_acc.map(|r| r.kmer_length).unwrap_or(6));

    // Save results
    let output_path = output_dir.join("parameter-sweep.json");
    let file = File::create(&output_path).expect("Failed to create output file");
    serde_json::to_writer_pretty(file, &results).expect("Failed to write results");
    println!("Results saved to: {}", output_path.display());
}

/// Run taxonomy experiment with real BOLD COI sequences
pub fn run_real_taxonomy_experiment(
    data_path: PathBuf,
    kmer_length: u8,
    output_dir: PathBuf,
) {
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    println!("{}", "═".repeat(60).blue());
    println!("{}", "  REAL DATA TAXONOMY EXPERIMENT".blue().bold());
    println!("{}", "  Using BOLD COI Barcode Sequences".blue());
    println!("{}", "═".repeat(60).blue());
    println!();

    // Load real data
    println!("{}", "1. Loading real COI sequences...".yellow());
    let file = File::open(&data_path).expect("Failed to open data file");
    let reader = BufReader::new(file);
    let data: RealCoiData = serde_json::from_reader(reader).expect("Failed to parse JSON");

    println!("   Source: {}", data.source);
    println!("   Retrieved: {}", data.retrieval_date);
    println!("   Marker: {}", data.marker);
    println!("   Sequences: {}", data.sequences.len());

    // Convert to specimens
    let specimens: Vec<Specimen> = data.sequences.iter().map(|seq| {
        Specimen {
            id: seq.id.clone(),
            sequence: seq.sequence.clone(),
            species: seq.species.clone(),
            genus: seq.genus.clone(),
            family: seq.family.clone(),
        }
    }).collect();

    let num_species: usize = specimens.iter()
        .map(|s| s.species.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let num_genera: usize = specimens.iter()
        .map(|s| s.genus.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let num_families: usize = specimens.iter()
        .map(|s| s.family.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();

    println!("   {} species, {} genera, {} families", num_species, num_genera, num_families);
    println!();

    // Encode all specimens
    println!("{}", "2. Encoding sequences...".yellow());
    let start = Instant::now();
    let seed = Seed::from_string("real-taxonomy-v1");
    let encoder = DnaEncoder::new(seed, kmer_length);

    let encoded: Vec<_> = specimens
        .iter()
        .filter_map(|spec| {
            encoder.encode_sequence(&spec.sequence).ok().map(|enc| (spec, enc))
        })
        .collect();
    let encoding_time = start.elapsed();
    println!("   Encoded {} sequences in {:.2}s", encoded.len(), encoding_time.as_secs_f64());

    // Compute similarity distributions
    println!("{}", "3. Computing similarities...".yellow());
    let start = Instant::now();

    let mut within_species_sims = Vec::new();
    let mut within_genus_sims = Vec::new();
    let mut within_family_sims = Vec::new();
    let mut between_family_sims = Vec::new();

    for i in 0..encoded.len() {
        for j in (i + 1)..encoded.len() {
            let (spec_i, enc_i) = &encoded[i];
            let (spec_j, enc_j) = &encoded[j];

            let sim = enc_i.vector.normalized_cosine_similarity(&enc_j.vector);

            if spec_i.species == spec_j.species {
                within_species_sims.push(sim);
            } else if spec_i.genus == spec_j.genus {
                within_genus_sims.push(sim);
            } else if spec_i.family == spec_j.family {
                within_family_sims.push(sim);
            } else {
                between_family_sims.push(sim);
            }
        }
    }
    let comparison_time = start.elapsed();

    // Compute statistics
    let species_stats = SimilarityStats::from_values(&within_species_sims);
    let genus_stats = SimilarityStats::from_values(&within_genus_sims);
    let family_stats = SimilarityStats::from_values(&within_family_sims);
    let between_stats = SimilarityStats::from_values(&between_family_sims);

    // Check monotonic separation
    let monotonic = species_stats.mean > genus_stats.mean
        && genus_stats.mean > family_stats.mean
        && (between_family_sims.is_empty() || family_stats.mean >= between_stats.mean);

    // Compute k-NN accuracy
    println!("{}", "4. Computing k-NN accuracy...".yellow());
    let species_accuracy = compute_knn_accuracy(&encoded, |s| &s.species, 1);
    let genus_accuracy = compute_knn_accuracy(&encoded, |s| &s.genus, 1);

    // Print results
    println!();
    println!("{}", "═".repeat(50).green());
    println!("{}", "REAL DATA RESULTS".green().bold());
    println!("{}", "═".repeat(50).green());

    println!();
    println!("Similarity Distributions (Real BOLD Sequences):");
    println!("  Same species:    {:.4} ± {:.4} (n={})",
             species_stats.mean, species_stats.std_dev, species_stats.count);
    if genus_stats.count > 0 {
        println!("  Same genus:      {:.4} ± {:.4} (n={})",
                 genus_stats.mean, genus_stats.std_dev, genus_stats.count);
    }
    if family_stats.count > 0 {
        println!("  Same family:     {:.4} ± {:.4} (n={})",
                 family_stats.mean, family_stats.std_dev, family_stats.count);
    }
    println!("  Between family:  {:.4} ± {:.4} (n={})",
             between_stats.mean, between_stats.std_dev, between_stats.count);

    println!();
    println!("Monotonic separation: {}",
             if monotonic { "YES ✓".green() } else { "NO ✗".red() });

    println!();
    println!("k-NN Accuracy (k=1):");
    println!("  Species: {:.1}%", species_accuracy * 100.0);
    println!("  Genus:   {:.1}%", genus_accuracy * 100.0);

    println!();
    println!("Timing:");
    println!("  Encoding: {:.2}s ({:.2}ms/seq)",
             encoding_time.as_secs_f64(),
             encoding_time.as_millis() as f64 / encoded.len().max(1) as f64);
    println!("  Comparisons: {:.2}s", comparison_time.as_secs_f64());

    // Save results
    let results = TaxonomyResults {
        config: TaxonomyConfig {
            sequences_per_species: 0, // N/A for real data
            kmer_length,
            total_specimens: encoded.len(),
            num_species,
            num_genera,
        },
        within_species: species_stats.clone().into(),
        within_genus: genus_stats.clone().into(),
        within_family: family_stats.clone().into(),
        random_pairs: between_stats.clone().into(),
        monotonic_separation: monotonic,
        species_top1_accuracy: species_accuracy,
        genus_top1_accuracy: genus_accuracy,
        encoding_time_ms: encoding_time.as_millis() as f64,
        comparison_time_ms: comparison_time.as_millis() as f64,
    };

    let output_path = output_dir.join("real-taxonomy-results.json");
    let file = File::create(&output_path).expect("Failed to create output file");
    serde_json::to_writer_pretty(file, &results).expect("Failed to write results");
    println!();
    println!("Results saved to: {}", output_path.display());

    // Publishable claim
    println!();
    println!("{}", "─".repeat(50));
    println!("{}", "VALIDATED CLAIM (Real Data)".cyan().bold());
    println!("{}", "─".repeat(50));
    if monotonic {
        println!(
            "\"Using real COI barcode sequences from {} species,\n\
             HDC encoding preserves taxonomic structure:\n\
             same-species ({:.3}) > same-genus ({:.3}) > same-family ({:.3})\n\
             with {:.0}% species classification accuracy (k-NN, k=1).\"",
            num_species, species_stats.mean, genus_stats.mean, family_stats.mean,
            species_accuracy * 100.0
        );
        println!();
        println!("Data source: {} ({})", data.source, data.retrieval_date);
    } else {
        println!("{}", "WARNING: Monotonic separation not achieved with real data!".red());
        println!("This may indicate need for parameter tuning or more diverse dataset.");
    }
}
