//! HDC Genetics Experiments
//!
//! Scientific experiments to validate HDC encoding for genetic data.
//! Also provides practical CLI tools for VCF encoding and pharmacogenomics.

mod taxonomy;
mod prefilter;
mod privacy;
mod hla;
mod fasta;
mod real_taxonomy;
mod real_hla;
mod pharmacogenomics;
mod benchmark;
mod cli_tools;

use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hdc-experiments")]
#[command(about = "HDC genomics experiments for scientific validation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Experiment 1: Taxonomy sanity check
    Taxonomy {
        /// Number of sequences per species (synthetic mode)
        #[arg(short, long, default_value = "20")]
        sequences: usize,

        /// K-mer length
        #[arg(short, long, default_value = "6")]
        kmer: u8,

        /// Output directory
        #[arg(short, long, default_value = "./results")]
        output: PathBuf,

        /// Use real BOLD COI sequences instead of synthetic
        #[arg(long)]
        real_data: bool,

        /// Path to real data directory containing FASTA files (default: data/real)
        #[arg(long)]
        data_path: Option<PathBuf>,
    },

    /// Experiment 2: Prefilter benchmark
    Prefilter {
        /// Corpus size
        #[arg(short, long, default_value = "5000")]
        corpus: usize,

        /// Number of queries
        #[arg(short = 'q', long, default_value = "100")]
        queries: usize,

        /// Top-K candidates
        #[arg(short = 'k', long, default_value = "100")]
        top_k: usize,

        /// K-mer length
        #[arg(long, default_value = "6")]
        kmer: u8,

        /// Output directory
        #[arg(short, long, default_value = "./results")]
        output: PathBuf,
    },

    /// Experiment 3: Privacy probing
    Privacy {
        /// Number of training sequences
        #[arg(short, long, default_value = "500")]
        training: usize,

        /// Number of attacker samples
        #[arg(short, long, default_value = "200")]
        attacker: usize,

        /// Noise level (0.0-0.5)
        #[arg(short, long, default_value = "0.0")]
        noise: f64,

        /// K-mer length
        #[arg(long, default_value = "6")]
        kmer: u8,

        /// Output directory
        #[arg(short, long, default_value = "./results")]
        output: PathBuf,
    },

    /// Experiment 4: HLA matching for transplant
    Hla {
        /// Number of donors
        #[arg(short, long, default_value = "1000")]
        donors: usize,

        /// Number of recipients to test
        #[arg(short, long, default_value = "100")]
        recipients: usize,

        /// Output directory
        #[arg(short, long, default_value = "./results")]
        output: PathBuf,

        /// Use realistic population allele frequencies
        #[arg(long)]
        use_frequencies: bool,

        /// Path to frequency data JSON file
        #[arg(long)]
        freq_path: Option<PathBuf>,
    },

    /// Experiment 6: Real HLA validation with IMGT/HLA data
    RealHla {
        /// Path to data directory containing hla/ subfolder
        #[arg(short, long)]
        data_path: Option<PathBuf>,

        /// K-mer length
        #[arg(short, long, default_value = "6")]
        kmer: u8,

        /// Output directory
        #[arg(short, long, default_value = "./results")]
        output: PathBuf,
    },

    /// Experiment 7: Pharmacogenomics validation with CYP450 genes
    Pharmacogenomics {
        /// Path to data directory containing cyp/ subfolder
        #[arg(short, long)]
        data_path: Option<PathBuf>,

        /// K-mer length
        #[arg(short, long, default_value = "6")]
        kmer: u8,

        /// Output directory
        #[arg(short, long, default_value = "./results")]
        output: PathBuf,
    },

    /// Run all experiments
    All {
        /// Output directory
        #[arg(short, long, default_value = "./results")]
        output: PathBuf,
    },

    /// Parameter sweep for k-mer optimization
    Sweep {
        /// K-mer lengths to test (comma-separated)
        #[arg(short, long, default_value = "4,5,6,7,8,9,10")]
        kmers: String,

        /// Sequences per test
        #[arg(short, long, default_value = "100")]
        sequences: usize,

        /// Output directory
        #[arg(short, long, default_value = "./results")]
        output: PathBuf,
    },

    /// Benchmark HDC performance vs BLAST/minimap2 references
    Benchmark {
        /// Output JSON results file
        #[arg(short, long)]
        output_json: Option<PathBuf>,
    },

    // ========================================================================
    // PRACTICAL CLINICAL TOOLS
    // ========================================================================

    /// Encode a VCF file to hypervector representation
    EncodeVcf {
        /// Input VCF file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output file for encoded vector (binary or JSON)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Apply differential privacy with given epsilon
        #[arg(long)]
        dp_epsilon: Option<f64>,

        /// Output format: json, binary, or hex
        #[arg(long, default_value = "json")]
        format: String,

        /// Seed for reproducible encoding
        #[arg(long, default_value = "hdc-clinical-v1")]
        seed: String,
    },

    /// Get pharmacogenomic predictions for a patient
    Pgx {
        /// Patient diplotypes in format: GENE:ALLELE1/ALLELE2 (e.g., CYP2D6:*1/*4)
        #[arg(short, long, num_args = 1..)]
        diplotypes: Vec<String>,

        /// Drug to check interaction for
        #[arg(short = 'r', long)]
        drug: Option<String>,

        /// Check all known drug interactions
        #[arg(long)]
        all_drugs: bool,

        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Search for similar patients in an encoded database
    Search {
        /// Query VCF file
        #[arg(short, long)]
        query: PathBuf,

        /// Database directory containing encoded patient vectors
        #[arg(short, long)]
        database: PathBuf,

        /// Number of top matches to return
        #[arg(short, long, default_value = "10")]
        top_k: usize,

        /// Minimum similarity threshold (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        threshold: f64,

        /// Apply differential privacy to query
        #[arg(long)]
        dp_epsilon: Option<f64>,

        /// Seed for encoding
        #[arg(long, default_value = "hdc-clinical-v1")]
        seed: String,

        /// Use GPU acceleration for batch similarity search
        #[arg(long)]
        gpu: bool,
    },

    /// Build a patient database from VCF files
    BuildDb {
        /// Directory containing VCF files
        #[arg(short, long)]
        input_dir: PathBuf,

        /// Output database directory
        #[arg(short, long)]
        output: PathBuf,

        /// Apply differential privacy with given epsilon
        #[arg(long)]
        dp_epsilon: Option<f64>,

        /// Seed for encoding
        #[arg(long, default_value = "hdc-clinical-v1")]
        seed: String,
    },
}

fn main() {
    let cli = Cli::parse();

    println!("{}", "═".repeat(60).cyan());
    println!("{}", "  HDC GENETICS EXPERIMENTS".cyan().bold());
    println!("{}", "  Hyperdimensional Computing for Genomics".cyan());
    println!("{}", "═".repeat(60).cyan());
    println!();

    match cli.command {
        Commands::Taxonomy { sequences, kmer, output, real_data, data_path } => {
            if real_data {
                let path = data_path.unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/real")
                });
                real_taxonomy::run_real_taxonomy_experiment(path, kmer, output);
            } else {
                taxonomy::run_taxonomy_experiment(sequences, kmer, output);
            }
        }
        Commands::Prefilter { corpus, queries, top_k, kmer, output } => {
            prefilter::run_prefilter_benchmark(corpus, queries, top_k, kmer, output);
        }
        Commands::Privacy { training, attacker, noise, kmer, output } => {
            privacy::run_privacy_analysis(training, attacker, noise, kmer, output);
        }
        Commands::Hla { donors, recipients, output, use_frequencies, freq_path } => {
            if use_frequencies {
                let path = freq_path.unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/hla_allele_frequencies.json")
                });
                hla::run_hla_matching_with_frequencies(donors, recipients, path, output);
            } else {
                hla::run_hla_matching(donors, recipients, output);
            }
        }
        Commands::RealHla { data_path, kmer, output } => {
            let path = data_path.unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/real")
            });
            real_hla::run_real_hla_experiment(path, kmer, output);
        }
        Commands::Pharmacogenomics { data_path, kmer, output } => {
            let path = data_path.unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/real")
            });
            pharmacogenomics::run_pharmacogenomics_experiment(path, kmer, output);
        }
        Commands::All { output } => {
            println!("{}", "Running all experiments...".yellow().bold());
            println!();

            println!("{}", "─".repeat(60));
            println!("{}", "EXPERIMENT 1: TAXONOMY".green().bold());
            println!("{}", "─".repeat(60));
            taxonomy::run_taxonomy_experiment(20, 6, output.join("taxonomy"));

            println!();
            println!("{}", "─".repeat(60));
            println!("{}", "EXPERIMENT 2: PREFILTER".green().bold());
            println!("{}", "─".repeat(60));
            prefilter::run_prefilter_benchmark(5000, 100, 100, 6, output.join("prefilter"));

            println!();
            println!("{}", "─".repeat(60));
            println!("{}", "EXPERIMENT 3: PRIVACY".green().bold());
            println!("{}", "─".repeat(60));
            privacy::run_privacy_analysis(500, 200, 0.0, 6, output.join("privacy"));

            println!();
            println!("{}", "─".repeat(60));
            println!("{}", "EXPERIMENT 4: HLA MATCHING".green().bold());
            println!("{}", "─".repeat(60));
            hla::run_hla_matching(1000, 100, output.join("hla"));

            println!();
            println!("{}", "═".repeat(60).green());
            println!("{}", "  ALL EXPERIMENTS COMPLETE".green().bold());
            println!("{}", "═".repeat(60).green());
        }
        Commands::Sweep { kmers, sequences, output } => {
            let kmer_list: Vec<u8> = kmers
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            taxonomy::run_parameter_sweep(&kmer_list, sequences, output);
        }
        Commands::Benchmark { output_json } => {
            println!("{}", "Running HDC Performance Benchmarks...".yellow().bold());
            println!();

            let report = benchmark::run_benchmark_suite();
            report.print();

            if let Some(path) = output_json {
                let json = serde_json::to_string_pretty(&report).unwrap();
                std::fs::write(&path, json).expect("Failed to write JSON");
                println!("\nResults saved to: {}", path.display());
            }
        }

        // ====================================================================
        // PRACTICAL CLINICAL TOOLS
        // ====================================================================

        Commands::EncodeVcf { input, output, dp_epsilon, format, seed } => {
            cli_tools::encode_vcf(&input, output.as_deref(), dp_epsilon, &format, &seed);
        }

        Commands::Pgx { diplotypes, drug, all_drugs, format } => {
            cli_tools::pharmacogenomics(&diplotypes, drug.as_deref(), all_drugs, &format);
        }

        Commands::Search { query, database, top_k, threshold, dp_epsilon, seed, gpu } => {
            cli_tools::search_patients(&query, &database, top_k, threshold, dp_epsilon, &seed, gpu);
        }

        Commands::BuildDb { input_dir, output, dp_epsilon, seed } => {
            cli_tools::build_database(&input_dir, &output, dp_epsilon, &seed);
        }
    }
}
