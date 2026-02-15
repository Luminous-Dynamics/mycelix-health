//! HDC Encode CLI Tool
//!
//! Encode genetic data (DNA sequences, VCF files, SNPs, HLA types, star alleles)
//! into hyperdimensional vectors for privacy-preserving similarity analysis.
//!
//! Usage:
//!   hdc-encode dna <sequence> [--kmer-length <k>] [--output <file>]
//!   hdc-encode vcf <file.vcf> [--output <file>] [--chromosome <chr>]
//!   hdc-encode snp <rsid:allele,...> [--output <file>]
//!   hdc-encode hla <alleles...> [--output <file>]
//!   hdc-encode pgx <gene:allele1/allele2,...> [--ancestry <ancestry>] [--output <file>]

use clap::{Parser, Subcommand};
use hdc_core::*;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hdc-encode")]
#[command(author = "Mycelix Health")]
#[command(version = "0.1.0")]
#[command(about = "Encode genetic data into hyperdimensional vectors", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format: json, binary, or hex
    #[arg(short, long, default_value = "json")]
    format: String,

    /// Random seed for reproducible encoding (32 hex bytes)
    #[arg(long)]
    seed: Option<String>,

    /// Output file (stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode a DNA sequence
    Dna {
        /// DNA sequence (ACGT characters) or '-' for stdin
        sequence: String,

        /// K-mer length for encoding (default: 6)
        #[arg(short, long, default_value = "6")]
        kmer_length: u8,

        /// Use learned codebook from file
        #[arg(long)]
        codebook: Option<PathBuf>,
    },

    /// Encode variants from a VCF file
    Vcf {
        /// Path to VCF file (.vcf or .vcf.gz)
        file: PathBuf,

        /// Filter to specific chromosome
        #[arg(short, long)]
        chromosome: Option<String>,

        /// Only include PASS variants
        #[arg(long)]
        pass_only: bool,

        /// Process in parallel (requires parallel feature)
        #[arg(long)]
        parallel: bool,
    },

    /// Encode a SNP panel
    Snp {
        /// SNPs as rsID:allele pairs (e.g., "rs123:A,rs456:G")
        snps: String,
    },

    /// Encode HLA typing
    Hla {
        /// HLA alleles (e.g., "A*02:01" "B*07:02")
        alleles: Vec<String>,

        /// Use locus-weighted encoding
        #[arg(long)]
        weighted: bool,

        /// Use allele-level resolution
        #[arg(long)]
        allele_level: bool,
    },

    /// Encode pharmacogenomic star alleles
    Pgx {
        /// Diplotypes as gene:allele1/allele2 (e.g., "CYP2D6:*1/*4,CYP2C19:*1/*2")
        diplotypes: String,

        /// Ancestry for population-specific encoding
        #[arg(short, long)]
        ancestry: Option<String>,
    },

    /// Batch encode sequences from a file
    Batch {
        /// Input file (one sequence per line, or FASTA format)
        file: PathBuf,

        /// K-mer length for encoding
        #[arg(short, long, default_value = "6")]
        kmer_length: u8,

        /// Process in parallel
        #[arg(long)]
        parallel: bool,
    },
}

/// Output structure for encoded results
#[derive(serde::Serialize)]
struct EncodingResult {
    encoding_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kmer_length: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kmer_count: Option<u32>,
    vector_dim: usize,
    vector_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    vector_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vector_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<ConfidenceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
struct ConfidenceInfo {
    level: String,
    description: String,
}

#[derive(serde::Serialize)]
struct BatchResult {
    total_sequences: usize,
    successful: usize,
    failed: usize,
    results: Vec<EncodingResult>,
    errors: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Parse seed if provided
    let seed = if let Some(seed_hex) = &cli.seed {
        let bytes = hex::decode(seed_hex)?;
        if bytes.len() != 32 {
            return Err("Seed must be exactly 32 bytes (64 hex characters)".into());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Seed::from_bytes(arr)
    } else {
        Seed::from_string("default-hdc-encoding-seed")
    };

    let result = match cli.command {
        Commands::Dna { sequence, kmer_length, codebook } => {
            encode_dna(&sequence, kmer_length, codebook, &seed)?
        }
        Commands::Vcf { file, chromosome, pass_only, parallel } => {
            encode_vcf(&file, chromosome, pass_only, parallel)?
        }
        Commands::Snp { snps } => {
            encode_snps(&snps, &seed)?
        }
        Commands::Hla { alleles, weighted, allele_level } => {
            encode_hla(&alleles, weighted, allele_level, &seed)?
        }
        Commands::Pgx { diplotypes, ancestry } => {
            encode_pgx(&diplotypes, ancestry, &seed)?
        }
        Commands::Batch { file, kmer_length, parallel } => {
            encode_batch(&file, kmer_length, parallel, &seed)?
        }
    };

    // Output result
    let output_str = match cli.format.as_str() {
        "json" => serde_json::to_string_pretty(&result)?,
        "compact" => serde_json::to_string(&result)?,
        _ => serde_json::to_string_pretty(&result)?,
    };

    if let Some(output_path) = cli.output {
        fs::write(&output_path, &output_str)?;
        eprintln!("Output written to: {}", output_path.display());
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

fn encode_dna(
    sequence: &str,
    kmer_length: u8,
    codebook_path: Option<PathBuf>,
    seed: &Seed,
) -> Result<EncodingResult, Box<dyn std::error::Error>> {
    // Handle stdin
    let seq = if sequence == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf.trim().to_uppercase()
    } else {
        sequence.to_uppercase()
    };

    let encoder = DnaEncoder::new(seed.clone(), kmer_length);
    let encoded = encoder.encode_sequence(&seq)?;

    Ok(EncodingResult {
        encoding_type: "dna_sequence".to_string(),
        kmer_length: Some(kmer_length),
        kmer_count: Some(encoded.kmer_count),
        vector_dim: HYPERVECTOR_DIM,
        vector_bytes: HYPERVECTOR_BYTES,
        vector_hex: Some(hex::encode(&encoded.vector.as_bytes())),
        vector_base64: None,
        confidence: None,
        metadata: Some(serde_json::json!({
            "sequence_length": seq.len(),
        })),
    })
}

#[cfg(feature = "gzip")]
fn encode_vcf(
    file: &PathBuf,
    chromosome: Option<String>,
    pass_only: bool,
    _parallel: bool,
) -> Result<EncodingResult, Box<dyn std::error::Error>> {
    use hdc_core::vcf::{WgsVcfEncoder, WgsVcfConfig};

    let config = WgsVcfConfig {
        chunk_size: 10000,
        parallel: false, // TODO: respect parallel flag when feature enabled
        pass_only,
    };

    let encoder = WgsVcfEncoder::new(config);
    let result = encoder.encode_file(file)?;

    let vector = if let Some(chr) = chromosome {
        result.chromosome_vectors.get(&chr)
            .ok_or_else(|| format!("Chromosome {} not found in VCF", chr))?
            .clone()
    } else {
        result.combined_vector
    };

    Ok(EncodingResult {
        encoding_type: "vcf_variants".to_string(),
        kmer_length: None,
        kmer_count: None,
        vector_dim: HYPERVECTOR_DIM,
        vector_bytes: HYPERVECTOR_BYTES,
        vector_hex: Some(hex::encode(&vector.as_bytes())),
        vector_base64: None,
        confidence: None,
        metadata: Some(serde_json::json!({
            "variant_count": result.variant_count,
            "chromosomes": result.chromosome_vectors.keys().collect::<Vec<_>>(),
            "file": file.display().to_string(),
        })),
    })
}

#[cfg(not(feature = "gzip"))]
fn encode_vcf(
    _file: &PathBuf,
    _chromosome: Option<String>,
    _pass_only: bool,
    _parallel: bool,
) -> Result<EncodingResult, Box<dyn std::error::Error>> {
    Err("VCF encoding requires the 'gzip' feature. Rebuild with: cargo build --features gzip".into())
}

fn encode_snps(
    snps_str: &str,
    seed: &Seed,
) -> Result<EncodingResult, Box<dyn std::error::Error>> {
    let snps: Vec<(String, char)> = snps_str
        .split(',')
        .map(|s| {
            let parts: Vec<&str> = s.trim().split(':').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid SNP format: {}. Expected rsID:allele", s));
            }
            let allele = parts[1].chars().next()
                .ok_or_else(|| format!("Empty allele for SNP: {}", parts[0]))?;
            Ok((parts[0].to_string(), allele))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let encoder = SnpEncoder::new(seed.clone());
    let encoded = encoder.encode_snp_panel(&snps)?;

    Ok(EncodingResult {
        encoding_type: "snp_panel".to_string(),
        kmer_length: None,
        kmer_count: Some(snps.len() as u32),
        vector_dim: HYPERVECTOR_DIM,
        vector_bytes: HYPERVECTOR_BYTES,
        vector_hex: Some(hex::encode(&encoded.as_bytes())),
        vector_base64: None,
        confidence: None,
        metadata: Some(serde_json::json!({
            "snp_count": snps.len(),
            "snps": snps.iter().map(|(id, a)| format!("{}:{}", id, a)).collect::<Vec<_>>(),
        })),
    })
}

fn encode_hla(
    alleles: &[String],
    weighted: bool,
    allele_level: bool,
    seed: &Seed,
) -> Result<EncodingResult, Box<dyn std::error::Error>> {
    let vector = if allele_level {
        let encoder = AlleleHlaEncoder::new(seed.clone());
        let mut combined = Hypervector::zero();
        for allele in alleles {
            let encoded = encoder.encode_allele(allele)?;
            combined = combined.bind(&encoded);
        }
        combined
    } else if weighted {
        let encoder = LocusWeightedHlaEncoder::new(seed.clone());
        let hla_types: Vec<&str> = alleles.iter().map(|s| s.as_str()).collect();
        encoder.encode_typing(&hla_types)?
    } else {
        let encoder = HlaEncoder::new(seed.clone());
        let hla_types: Vec<&str> = alleles.iter().map(|s| s.as_str()).collect();
        encoder.encode_typing(&hla_types)?
    };

    Ok(EncodingResult {
        encoding_type: "hla_typing".to_string(),
        kmer_length: None,
        kmer_count: Some(alleles.len() as u32),
        vector_dim: HYPERVECTOR_DIM,
        vector_bytes: HYPERVECTOR_BYTES,
        vector_hex: Some(hex::encode(&vector.as_bytes())),
        vector_base64: None,
        confidence: None,
        metadata: Some(serde_json::json!({
            "allele_count": alleles.len(),
            "alleles": alleles,
            "encoding_mode": if allele_level { "allele_level" } else if weighted { "locus_weighted" } else { "basic" },
        })),
    })
}

fn encode_pgx(
    diplotypes_str: &str,
    ancestry: Option<String>,
    seed: &Seed,
) -> Result<EncodingResult, Box<dyn std::error::Error>> {
    // Parse diplotypes: "CYP2D6:*1/*4,CYP2C19:*1/*2"
    let diplotypes: Vec<(String, String, String)> = diplotypes_str
        .split(',')
        .map(|s| {
            let parts: Vec<&str> = s.trim().split(':').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid diplotype format: {}. Expected gene:allele1/allele2", s));
            }
            let gene = parts[0].to_string();
            let allele_parts: Vec<&str> = parts[1].split('/').collect();
            if allele_parts.len() != 2 {
                return Err(format!("Invalid allele format: {}. Expected allele1/allele2", parts[1]));
            }
            Ok((gene, allele_parts[0].to_string(), allele_parts[1].to_string()))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let vector = if let Some(ancestry_str) = &ancestry {
        let ancestry_enum = parse_ancestry(ancestry_str)?;
        let encoder = AncestryInformedEncoder::new(seed.clone());
        let profile: Vec<_> = diplotypes.iter()
            .map(|(g, a1, a2)| (g.as_str(), a1.as_str(), a2.as_str()))
            .collect();
        let result = encoder.encode_profile_with_ancestry(&profile, ancestry_enum)?;
        result.vector
    } else {
        let encoder = StarAlleleEncoder::new(seed.clone());
        let mut combined = Hypervector::zero();
        for (gene, allele1, allele2) in &diplotypes {
            let encoded = encoder.encode_diplotype(gene, allele1, allele2)?;
            combined = bundle(&[&combined, &encoded.vector]);
        }
        combined
    };

    Ok(EncodingResult {
        encoding_type: "pharmacogenomics".to_string(),
        kmer_length: None,
        kmer_count: Some(diplotypes.len() as u32),
        vector_dim: HYPERVECTOR_DIM,
        vector_bytes: HYPERVECTOR_BYTES,
        vector_hex: Some(hex::encode(&vector.as_bytes())),
        vector_base64: None,
        confidence: None,
        metadata: Some(serde_json::json!({
            "gene_count": diplotypes.len(),
            "diplotypes": diplotypes.iter().map(|(g, a1, a2)| format!("{}:{}/*{}", g, a1, a2)).collect::<Vec<_>>(),
            "ancestry": ancestry,
        })),
    })
}

fn encode_batch(
    file: &PathBuf,
    kmer_length: u8,
    _parallel: bool,
    seed: &Seed,
) -> Result<BatchResult, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file)?;
    let sequences: Vec<&str> = content.lines()
        .filter(|l| !l.starts_with('>') && !l.is_empty())
        .collect();

    let encoder = DnaEncoder::new(seed.clone(), kmer_length);
    let mut results = Vec::new();
    let mut errors = Vec::new();
    let mut successful = 0;

    for (i, seq) in sequences.iter().enumerate() {
        match encoder.encode_sequence(seq) {
            Ok(encoded) => {
                results.push(EncodingResult {
                    encoding_type: "dna_sequence".to_string(),
                    kmer_length: Some(kmer_length),
                    kmer_count: Some(encoded.kmer_count),
                    vector_dim: HYPERVECTOR_DIM,
                    vector_bytes: HYPERVECTOR_BYTES,
                    vector_hex: Some(hex::encode(&encoded.vector.as_bytes())),
                    vector_base64: None,
                    confidence: None,
                    metadata: Some(serde_json::json!({
                        "index": i,
                        "sequence_length": seq.len(),
                    })),
                });
                successful += 1;
            }
            Err(e) => {
                errors.push(format!("Sequence {}: {}", i, e));
            }
        }
    }

    Ok(BatchResult {
        total_sequences: sequences.len(),
        successful,
        failed: errors.len(),
        results,
        errors,
    })
}

fn parse_ancestry(s: &str) -> Result<Ancestry, Box<dyn std::error::Error>> {
    match s.to_lowercase().as_str() {
        "african" => Ok(Ancestry::African),
        "american" | "indigenous" => Ok(Ancestry::American),
        "central_south_asian" | "south_asian" => Ok(Ancestry::CentralSouthAsian),
        "east_asian" | "asian" => Ok(Ancestry::EastAsian),
        "european" | "caucasian" => Ok(Ancestry::European),
        "latino" | "hispanic" => Ok(Ancestry::Latino),
        "near_eastern" | "middle_eastern" => Ok(Ancestry::NearEastern),
        "oceanian" => Ok(Ancestry::Oceanian),
        "mixed" | "multi" => Ok(Ancestry::Mixed),
        _ => Err(format!("Unknown ancestry: {}. Valid options: african, american, central_south_asian, east_asian, european, latino, near_eastern, oceanian, mixed", s).into()),
    }
}

// Hex encoding helper (simple implementation since we might not have the hex crate)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("Hex string must have even length".to_string());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}
