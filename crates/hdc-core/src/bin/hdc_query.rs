//! HDC Query CLI Tool
//!
//! Query hyperdimensional vectors for similarity analysis.
//! Supports single queries, batch searches, and database operations.
//!
//! Usage:
//!   hdc-query similarity <vector1> <vector2> [--metric <metric>]
//!   hdc-query search <query> --database <db> [--top-k <k>] [--threshold <t>]
//!   hdc-query batch <queries> --database <db> [--output <file>]
//!   hdc-query index <vectors> --output <index>

use clap::{Parser, Subcommand};
use hdc_core::*;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "hdc-query")]
#[command(author = "Mycelix Health")]
#[command(version = "0.1.0")]
#[command(about = "Query hyperdimensional vectors for similarity analysis", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format: json, csv, or table
    #[arg(short, long, default_value = "json")]
    format: String,

    /// Output file (stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Calculate similarity between two vectors
    Similarity {
        /// First vector (hex string or @file)
        vector1: String,

        /// Second vector (hex string or @file)
        vector2: String,

        /// Similarity metric: cosine, hamming, jaccard
        #[arg(short, long, default_value = "cosine")]
        metric: String,

        /// Include confidence analysis
        #[arg(long)]
        with_confidence: bool,
    },

    /// Search for similar vectors in a database
    Search {
        /// Query vector (hex string or @file)
        query: String,

        /// Database file (JSON array of vectors with metadata)
        #[arg(short, long)]
        database: PathBuf,

        /// Return top-k results
        #[arg(short = 'k', long, default_value = "10")]
        top_k: usize,

        /// Minimum similarity threshold (0.0 - 1.0)
        #[arg(short, long)]
        threshold: Option<f64>,

        /// Similarity metric
        #[arg(short, long, default_value = "cosine")]
        metric: String,
    },

    /// Batch similarity computation
    Batch {
        /// File containing query vectors (one hex per line or JSON)
        queries: PathBuf,

        /// Database file
        #[arg(short, long)]
        database: PathBuf,

        /// Return top-k for each query
        #[arg(short = 'k', long, default_value = "5")]
        top_k: usize,

        /// Similarity metric
        #[arg(short, long, default_value = "cosine")]
        metric: String,
    },

    /// Build a similarity index from vectors
    Index {
        /// Input file with vectors
        vectors: PathBuf,

        /// Output index file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Compute pairwise similarity matrix
    Matrix {
        /// File containing vectors
        vectors: PathBuf,

        /// Similarity metric
        #[arg(short, long, default_value = "cosine")]
        metric: String,

        /// Only output pairs above threshold
        #[arg(short, long)]
        threshold: Option<f64>,
    },

    /// Analyze a vector's properties
    Analyze {
        /// Vector to analyze (hex string or @file)
        vector: String,
    },
}

/// Database entry with vector and metadata
#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct DatabaseEntry {
    id: String,
    vector: String, // hex-encoded
    #[serde(default)]
    metadata: serde_json::Value,
}

/// Search result
#[derive(serde::Serialize)]
struct SearchResult {
    id: String,
    similarity: f64,
    metric: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<ConfidenceResult>,
    metadata: serde_json::Value,
}

#[derive(serde::Serialize)]
struct ConfidenceResult {
    level: String,
    bits_above_random: usize,
    z_score: f64,
    is_significant: bool,
}

#[derive(serde::Serialize)]
struct SimilarityResult {
    similarity: f64,
    metric: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<ConfidenceResult>,
}

#[derive(serde::Serialize)]
struct BatchSearchResult {
    query_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    query_id: Option<String>,
    results: Vec<SearchResult>,
}

#[derive(serde::Serialize)]
struct MatrixEntry {
    i: usize,
    j: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_i: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_j: Option<String>,
    similarity: f64,
}

#[derive(serde::Serialize)]
struct VectorAnalysis {
    dimension: usize,
    bytes: usize,
    popcount: usize,
    density: f64,
    entropy_estimate: f64,
    is_valid: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let result: serde_json::Value = match cli.command {
        Commands::Similarity { vector1, vector2, metric, with_confidence } => {
            let v1 = load_vector(&vector1)?;
            let v2 = load_vector(&vector2)?;
            let result = compute_similarity(&v1, &v2, &metric, with_confidence)?;
            serde_json::to_value(result)?
        }
        Commands::Search { query, database, top_k, threshold, metric } => {
            let query_vec = load_vector(&query)?;
            let db = load_database(&database)?;
            let results = search_database(&query_vec, &db, top_k, threshold, &metric)?;
            serde_json::to_value(results)?
        }
        Commands::Batch { queries, database, top_k, metric } => {
            let query_vecs = load_vectors(&queries)?;
            let db = load_database(&database)?;
            let results = batch_search(&query_vecs, &db, top_k, &metric)?;
            serde_json::to_value(results)?
        }
        Commands::Index { vectors, output } => {
            build_index(&vectors, &output)?;
            serde_json::json!({
                "status": "success",
                "index_file": output.display().to_string(),
            })
        }
        Commands::Matrix { vectors, metric, threshold } => {
            let vecs = load_vectors(&vectors)?;
            let matrix = compute_matrix(&vecs, &metric, threshold)?;
            serde_json::to_value(matrix)?
        }
        Commands::Analyze { vector } => {
            let v = load_vector(&vector)?;
            let analysis = analyze_vector(&v)?;
            serde_json::to_value(analysis)?
        }
    };

    // Output result
    let output_str = match cli.format.as_str() {
        "json" => serde_json::to_string_pretty(&result)?,
        "compact" => serde_json::to_string(&result)?,
        "csv" => result_to_csv(&result)?,
        "table" => result_to_table(&result)?,
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

fn load_vector(input: &str) -> Result<Hypervector, Box<dyn std::error::Error>> {
    let hex_str = if input.starts_with('@') {
        // Load from file
        let path = &input[1..];
        let content = fs::read_to_string(path)?;
        // Try to parse as JSON first
        if let Ok(entry) = serde_json::from_str::<DatabaseEntry>(&content) {
            entry.vector
        } else {
            content.trim().to_string()
        }
    } else {
        input.to_string()
    };

    let bytes = hex_decode(&hex_str)?;
    if bytes.len() != HYPERVECTOR_BYTES {
        return Err(format!(
            "Invalid vector size: {} bytes (expected {})",
            bytes.len(),
            HYPERVECTOR_BYTES
        ).into());
    }

    Ok(Hypervector::from_bytes(&bytes))
}

fn load_vectors(path: &PathBuf) -> Result<Vec<(Option<String>, Hypervector)>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;

    // Try JSON array first
    if let Ok(entries) = serde_json::from_str::<Vec<DatabaseEntry>>(&content) {
        return entries.iter()
            .map(|e| {
                let bytes = hex_decode(&e.vector)?;
                Ok((Some(e.id.clone()), Hypervector::from_bytes(&bytes)))
            })
            .collect();
    }

    // Fall back to one hex per line
    content.lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, line)| {
            let bytes = hex_decode(line.trim())?;
            Ok((None, Hypervector::from_bytes(&bytes)))
        })
        .collect()
}

fn load_database(path: &PathBuf) -> Result<Vec<DatabaseEntry>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let entries: Vec<DatabaseEntry> = serde_json::from_str(&content)?;
    Ok(entries)
}

fn compute_similarity(
    v1: &Hypervector,
    v2: &Hypervector,
    metric: &str,
    with_confidence: bool,
) -> Result<SimilarityResult, Box<dyn std::error::Error>> {
    let similarity = match metric {
        "cosine" => v1.normalized_cosine_similarity(v2),
        "hamming" => v1.hamming_similarity(v2),
        "jaccard" => v1.jaccard_similarity(v2),
        _ => return Err(format!("Unknown metric: {}. Valid: cosine, hamming, jaccard", metric).into()),
    };

    let confidence = if with_confidence {
        let conf = SimilarityWithConfidence::calculate(v1, v2);
        Some(ConfidenceResult {
            level: format!("{:?}", conf.confidence),
            bits_above_random: conf.bits_above_random,
            z_score: conf.z_score,
            is_significant: conf.is_significant(),
        })
    } else {
        None
    };

    Ok(SimilarityResult {
        similarity,
        metric: metric.to_string(),
        confidence,
    })
}

fn search_database(
    query: &Hypervector,
    database: &[DatabaseEntry],
    top_k: usize,
    threshold: Option<f64>,
    metric: &str,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
    let mut results: Vec<SearchResult> = database.iter()
        .filter_map(|entry| {
            let bytes = hex_decode(&entry.vector).ok()?;
            let vec = Hypervector::from_bytes(&bytes);

            let similarity = match metric {
                "cosine" => query.normalized_cosine_similarity(&vec),
                "hamming" => query.hamming_similarity(&vec),
                "jaccard" => query.jaccard_similarity(&vec),
                _ => return None,
            };

            if let Some(thresh) = threshold {
                if similarity < thresh {
                    return None;
                }
            }

            Some(SearchResult {
                id: entry.id.clone(),
                similarity,
                metric: metric.to_string(),
                confidence: None,
                metadata: entry.metadata.clone(),
            })
        })
        .collect();

    // Sort by similarity descending
    results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
    results.truncate(top_k);

    Ok(results)
}

fn batch_search(
    queries: &[(Option<String>, Hypervector)],
    database: &[DatabaseEntry],
    top_k: usize,
    metric: &str,
) -> Result<Vec<BatchSearchResult>, Box<dyn std::error::Error>> {
    queries.iter()
        .enumerate()
        .map(|(i, (id, query))| {
            let results = search_database(query, database, top_k, None, metric)?;
            Ok(BatchSearchResult {
                query_index: i,
                query_id: id.clone(),
                results,
            })
        })
        .collect()
}

fn build_index(
    vectors_path: &PathBuf,
    output_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let vectors = load_vectors(vectors_path)?;

    // For now, just copy the database format - real indexing would use HdcIndex
    let entries: Vec<DatabaseEntry> = vectors.iter()
        .enumerate()
        .map(|(i, (id, vec))| {
            DatabaseEntry {
                id: id.clone().unwrap_or_else(|| format!("vec_{}", i)),
                vector: hex_encode(vec.as_bytes()),
                metadata: serde_json::json!({}),
            }
        })
        .collect();

    let output = serde_json::to_string_pretty(&entries)?;
    fs::write(output_path, output)?;

    eprintln!("Indexed {} vectors", entries.len());
    Ok(())
}

fn compute_matrix(
    vectors: &[(Option<String>, Hypervector)],
    metric: &str,
    threshold: Option<f64>,
) -> Result<Vec<MatrixEntry>, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();

    for i in 0..vectors.len() {
        for j in (i + 1)..vectors.len() {
            let similarity = match metric {
                "cosine" => vectors[i].1.normalized_cosine_similarity(&vectors[j].1),
                "hamming" => vectors[i].1.hamming_similarity(&vectors[j].1),
                "jaccard" => vectors[i].1.jaccard_similarity(&vectors[j].1),
                _ => continue,
            };

            if let Some(thresh) = threshold {
                if similarity < thresh {
                    continue;
                }
            }

            entries.push(MatrixEntry {
                i,
                j,
                id_i: vectors[i].0.clone(),
                id_j: vectors[j].0.clone(),
                similarity,
            });
        }
    }

    // Sort by similarity descending
    entries.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

    Ok(entries)
}

fn analyze_vector(vector: &Hypervector) -> Result<VectorAnalysis, Box<dyn std::error::Error>> {
    let popcount = vector.popcount();
    let density = popcount as f64 / HYPERVECTOR_DIM as f64;

    // Estimate entropy (ideal random vector has ~50% density)
    let p = density;
    let entropy = if p > 0.0 && p < 1.0 {
        -p * p.log2() - (1.0 - p) * (1.0 - p).log2()
    } else {
        0.0
    };

    Ok(VectorAnalysis {
        dimension: HYPERVECTOR_DIM,
        bytes: HYPERVECTOR_BYTES,
        popcount,
        density,
        entropy_estimate: entropy,
        is_valid: true,
    })
}

fn result_to_csv(value: &serde_json::Value) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = String::new();

    if let Some(arr) = value.as_array() {
        if let Some(first) = arr.first() {
            if let Some(obj) = first.as_object() {
                // Header
                let headers: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
                output.push_str(&headers.join(","));
                output.push('\n');

                // Rows
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        let row: Vec<String> = headers.iter()
                            .map(|h| {
                                obj.get(*h)
                                    .map(|v| v.to_string().trim_matches('"').to_string())
                                    .unwrap_or_default()
                            })
                            .collect();
                        output.push_str(&row.join(","));
                        output.push('\n');
                    }
                }
            }
        }
    } else {
        output = serde_json::to_string(value)?;
    }

    Ok(output)
}

fn result_to_table(value: &serde_json::Value) -> Result<String, Box<dyn std::error::Error>> {
    // Simple table format
    let json_str = serde_json::to_string_pretty(value)?;
    Ok(json_str)
}

// Hex helpers
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err("Hex string must have even length".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string().into()))
        .collect()
}
