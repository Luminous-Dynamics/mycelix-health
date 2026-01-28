//! Practical CLI tools for HDC genomics
//!
//! Provides commands for:
//! - VCF encoding with optional differential privacy
//! - Pharmacogenomic predictions
//! - Patient similarity search
//! - Database building

use colored::*;
use hdc_core::{
    DpHypervector, DpParams, Hypervector, Seed,
    StarAlleleEncoder, MetabolizerPhenotype, DrugRecommendation,
    VcfReader, VcfEncoder,
    similarity::HdcIndex,
};

#[cfg(feature = "gpu")]
use hdc_core::gpu;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::Path;

// =============================================================================
// VCF Encoding
// =============================================================================

#[derive(Serialize, Deserialize)]
struct EncodedVcfOutput {
    source_file: String,
    variant_count: usize,
    vector_hex: String,
    dp_applied: bool,
    dp_epsilon: Option<f64>,
    seed: String,
}

pub fn encode_vcf(
    input: &Path,
    output: Option<&Path>,
    dp_epsilon: Option<f64>,
    format: &str,
    seed_str: &str,
) {
    println!("{}", "─".repeat(60));
    println!("{}", "VCF ENCODING".green().bold());
    println!("{}", "─".repeat(60));

    // Read VCF file
    let file = match File::open(input) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{} Failed to open VCF file: {}", "Error:".red().bold(), e);
            return;
        }
    };

    let mut reader = match VcfReader::new(BufReader::new(file)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Failed to parse VCF: {}", "Error:".red().bold(), e);
            return;
        }
    };

    let variants = match reader.read_variants() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} Failed to read variants: {}", "Error:".red().bold(), e);
            return;
        }
    };

    println!("  Input: {}", input.display());
    println!("  Variants parsed: {}", variants.len().to_string().cyan());

    // Encode
    let seed = Seed::from_string(seed_str);
    let encoder = VcfEncoder::new(seed);

    let encoded = match encoder.encode_variants(&variants) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} Failed to encode: {}", "Error:".red().bold(), e);
            return;
        }
    };

    println!("  Encoded to: {} dimensions", "10,000".cyan());

    // Apply DP if requested
    let (final_vector, dp_applied) = if let Some(epsilon) = dp_epsilon {
        let dp_params = DpParams::pure(epsilon);
        let dp_vec = DpHypervector::from_vector(&encoded, dp_params, None);
        println!("  Differential privacy: {} (ε={})", "applied".yellow(), epsilon);
        println!("    Expected similarity retention: {:.1}%",
                 dp_params.expected_similarity_retention() * 100.0);
        (dp_vec.vector, true)
    } else {
        (encoded, false)
    };

    // Output
    let hex_vector = final_vector.as_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    match format {
        "json" => {
            let output_data = EncodedVcfOutput {
                source_file: input.display().to_string(),
                variant_count: variants.len(),
                vector_hex: hex_vector.clone(),
                dp_applied,
                dp_epsilon,
                seed: seed_str.to_string(),
            };

            let json = serde_json::to_string_pretty(&output_data).unwrap();

            if let Some(out_path) = output {
                fs::write(out_path, &json).expect("Failed to write output");
                println!("\n  Output saved to: {}", out_path.display().to_string().green());
            } else {
                println!("\n{}", json);
            }
        }
        "hex" => {
            if let Some(out_path) = output {
                fs::write(out_path, &hex_vector).expect("Failed to write output");
                println!("\n  Output saved to: {}", out_path.display().to_string().green());
            } else {
                println!("\n{}", hex_vector);
            }
        }
        "binary" => {
            if let Some(out_path) = output {
                fs::write(out_path, final_vector.as_bytes()).expect("Failed to write output");
                println!("\n  Output saved to: {}", out_path.display().to_string().green());
            } else {
                eprintln!("{} Binary format requires output file path", "Error:".red().bold());
            }
        }
        _ => {
            eprintln!("{} Unknown format: {}", "Error:".red().bold(), format);
        }
    }

    println!();
    println!("{}", "Encoding complete!".green().bold());
}

// =============================================================================
// Pharmacogenomics
// =============================================================================

#[derive(Serialize)]
struct PgxOutput {
    diplotypes: Vec<DiplotypeInfo>,
    drug_predictions: Vec<DrugPrediction>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct DiplotypeInfo {
    gene: String,
    allele1: String,
    allele2: String,
    activity_score: f64,
    phenotype: String,
}

#[derive(Serialize)]
struct DrugPrediction {
    drug: String,
    gene: String,
    phenotype: String,
    recommendation: String,
    activity_score: f64,
}

pub fn pharmacogenomics(
    diplotype_strs: &[String],
    drug: Option<&str>,
    all_drugs: bool,
    format: &str,
) {
    println!("{}", "─".repeat(60));
    println!("{}", "PHARMACOGENOMICS ANALYSIS".green().bold());
    println!("{}", "─".repeat(60));

    let seed = Seed::from_string("hdc-pgx-v1");
    let encoder = StarAlleleEncoder::new(seed);

    // Parse diplotypes (format: GENE:*ALLELE1/*ALLELE2)
    let mut diplotypes: Vec<(&str, &str, &str)> = Vec::new();
    let mut parse_errors = Vec::new();

    for dt_str in diplotype_strs {
        let parts: Vec<&str> = dt_str.split(':').collect();
        if parts.len() != 2 {
            parse_errors.push(format!("Invalid format '{}': expected GENE:*A/*B", dt_str));
            continue;
        }

        let gene = parts[0];
        let alleles: Vec<&str> = parts[1].split('/').collect();
        if alleles.len() != 2 {
            parse_errors.push(format!("Invalid alleles in '{}': expected *A/*B", dt_str));
            continue;
        }

        diplotypes.push((gene, alleles[0], alleles[1]));
    }

    if !parse_errors.is_empty() {
        for err in &parse_errors {
            eprintln!("{} {}", "Warning:".yellow().bold(), err);
        }
    }

    if diplotypes.is_empty() {
        eprintln!("{} No valid diplotypes provided", "Error:".red().bold());
        eprintln!("  Format: GENE:*ALLELE1/*ALLELE2");
        eprintln!("  Example: CYP2D6:*1/*4 CYP2C19:*1/*2");
        return;
    }

    // Encode profile
    let profile = match encoder.encode_profile(&diplotypes) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} Failed to encode profile: {}", "Error:".red().bold(), e);
            return;
        }
    };

    // Collect results
    let mut diplotype_infos: Vec<DiplotypeInfo> = Vec::new();
    let mut drug_predictions: Vec<DrugPrediction> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    println!("\n{}", "Patient Metabolizer Profile:".cyan().bold());
    println!("{}", "─".repeat(50));

    for dt in &profile.diplotypes {
        let phenotype_str = format!("{}", dt.phenotype);
        let color = match dt.phenotype {
            MetabolizerPhenotype::Poor => "red",
            MetabolizerPhenotype::Intermediate => "yellow",
            MetabolizerPhenotype::Ultrarapid => "magenta",
            _ => "green",
        };

        println!("  {} {}/{}",
                 format!("{:10}", dt.gene).bold(),
                 dt.allele1,
                 dt.allele2);
        println!("    Activity Score: {:.2}", dt.activity_score);
        println!("    Phenotype: {}",
                 match color {
                     "red" => phenotype_str.red().bold(),
                     "yellow" => phenotype_str.yellow(),
                     "magenta" => phenotype_str.magenta(),
                     _ => phenotype_str.green(),
                 });

        if dt.phenotype == MetabolizerPhenotype::Poor {
            let warning = format!("{} is a POOR METABOLIZER - clinical action may be required", dt.gene);
            warnings.push(warning.clone());
        }

        diplotype_infos.push(DiplotypeInfo {
            gene: dt.gene.clone(),
            allele1: dt.allele1.clone(),
            allele2: dt.allele2.clone(),
            activity_score: dt.activity_score,
            phenotype: phenotype_str,
        });

        println!();
    }

    // Drug interactions
    let drugs_to_check: Vec<&str> = if all_drugs {
        vec!["codeine", "tramadol", "clopidogrel", "omeprazole",
             "warfarin", "azathioprine", "fluorouracil", "simvastatin"]
    } else if let Some(d) = drug {
        vec![d]
    } else {
        vec![]
    };

    if !drugs_to_check.is_empty() {
        println!("{}", "Drug Interaction Predictions:".cyan().bold());
        println!("{}", "─".repeat(50));

        for drug_name in drugs_to_check {
            if let Some(pred) = encoder.predict_drug_interaction(&profile, drug_name) {
                let rec_str = format!("{}", pred.recommendation);
                let rec_color = match pred.recommendation {
                    DrugRecommendation::Avoid => "red",
                    DrugRecommendation::ReducedDose | DrugRecommendation::UseWithCaution => "yellow",
                    DrugRecommendation::ConsiderAlternative => "magenta",
                    _ => "green",
                };

                println!("  {} (via {})", drug_name.bold(), pred.gene);
                println!("    Phenotype: {}", pred.phenotype);
                println!("    Recommendation: {}",
                         match rec_color {
                             "red" => rec_str.red().bold(),
                             "yellow" => rec_str.yellow(),
                             "magenta" => rec_str.magenta(),
                             _ => rec_str.green(),
                         });

                if pred.recommendation == DrugRecommendation::Avoid {
                    warnings.push(format!("AVOID {} - {} poor metabolizer", drug_name.to_uppercase(), pred.gene));
                }

                drug_predictions.push(DrugPrediction {
                    drug: drug_name.to_string(),
                    gene: pred.gene,
                    phenotype: format!("{}", pred.phenotype),
                    recommendation: rec_str,
                    activity_score: pred.activity_score,
                });

                println!();
            }
        }
    }

    // Warnings
    if !warnings.is_empty() {
        println!("{}", "⚠️  CLINICAL WARNINGS:".red().bold());
        println!("{}", "─".repeat(50));
        for warning in &warnings {
            println!("  • {}", warning.red());
        }
        println!();
    }

    // JSON output
    if format == "json" {
        let output = PgxOutput {
            diplotypes: diplotype_infos,
            drug_predictions,
            warnings,
        };
        println!("\n{}", serde_json::to_string_pretty(&output).unwrap());
    }

    println!("{}", "Analysis complete!".green().bold());
}

// =============================================================================
// Patient Search
// =============================================================================

pub fn search_patients(
    query_path: &Path,
    database_path: &Path,
    top_k: usize,
    threshold: f64,
    dp_epsilon: Option<f64>,
    seed_str: &str,
    use_gpu: bool,
) {
    println!("{}", "─".repeat(60));
    println!("{}", "PATIENT SIMILARITY SEARCH".green().bold());
    println!("{}", "─".repeat(60));

    let seed = Seed::from_string(seed_str);
    let encoder = VcfEncoder::new(seed);

    // Load database
    println!("  Loading database from: {}", database_path.display());

    let db_index_path = database_path.join("index.json");
    if !db_index_path.exists() {
        eprintln!("{} Database index not found. Run 'build-db' first.", "Error:".red().bold());
        return;
    }

    let index_data: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&db_index_path).unwrap()
    ).unwrap();

    let patients = index_data["patients"].as_array().unwrap();

    // Load patient vectors and IDs
    let mut patient_ids: Vec<String> = Vec::new();
    let mut patient_vectors: Vec<Hypervector> = Vec::new();
    let mut index = HdcIndex::new();

    for patient in patients {
        let id = patient["id"].as_str().unwrap();
        let hex = patient["vector_hex"].as_str().unwrap();
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i+2], 16).unwrap())
            .collect();
        let vector = Hypervector::from_bytes(bytes).unwrap();

        patient_ids.push(id.to_string());
        patient_vectors.push(vector.clone());
        index.add(id.to_string(), vector);
    }

    println!("  Database loaded: {} patients", patient_ids.len().to_string().cyan());

    // Encode query
    println!("  Encoding query: {}", query_path.display());

    let file = File::open(query_path).expect("Failed to open query VCF");
    let mut reader = VcfReader::new(BufReader::new(file)).expect("Failed to parse VCF");
    let variants = reader.read_variants().expect("Failed to read variants");
    let query_encoded = encoder.encode_variants(&variants).expect("Failed to encode");

    // Apply DP if requested
    let query_vector = if let Some(epsilon) = dp_epsilon {
        let dp_params = DpParams::pure(epsilon);
        let dp_vec = DpHypervector::from_vector(&query_encoded, dp_params, None);
        println!("  Differential privacy applied: ε={}", epsilon);
        dp_vec.vector
    } else {
        query_encoded
    };

    // Search (GPU or CPU)
    let start_time = std::time::Instant::now();

    #[cfg(feature = "gpu")]
    let results = if use_gpu {
        println!("  Using {} for similarity computation...", "GPU".magenta().bold());
        search_with_gpu(&query_vector, &patient_vectors, &patient_ids, top_k)
    } else {
        println!("  Using {} for similarity computation...", "CPU".cyan());
        search_with_cpu(&index, &query_vector, top_k)
    };

    #[cfg(not(feature = "gpu"))]
    let results = {
        if use_gpu {
            eprintln!("{} GPU feature not enabled. Using CPU instead.", "Warning:".yellow());
            eprintln!("  Build with: cargo build --features gpu");
        }
        println!("  Using {} for similarity computation...", "CPU".cyan());
        search_with_cpu(&index, &query_vector, top_k)
    };

    let elapsed = start_time.elapsed();

    // Display results
    println!("\n{}", "Search Results:".cyan().bold());
    println!("{}", "─".repeat(50));

    let mut found = 0;
    for (rank, (id, similarity)) in results.iter().enumerate() {
        if *similarity >= threshold {
            found += 1;
            let sim_pct = *similarity * 100.0;
            let sim_color = if sim_pct > 90.0 { "green" }
                           else if sim_pct > 70.0 { "yellow" }
                           else { "white" };

            println!("  {}. {} - {:.1}%",
                     rank + 1,
                     id.bold(),
                     match sim_color {
                         "green" => format!("{:.1}", sim_pct).green(),
                         "yellow" => format!("{:.1}", sim_pct).yellow(),
                         _ => format!("{:.1}", sim_pct).normal(),
                     });
        }
    }

    if found == 0 {
        println!("  No matches found above threshold ({:.0}%)", threshold * 100.0);
    }

    println!();
    println!("  Search time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("\n{}", "Search complete!".green().bold());
}

/// CPU-based similarity search
fn search_with_cpu(index: &HdcIndex, query: &Hypervector, top_k: usize) -> Vec<(String, f64)> {
    index.search(query, top_k)
        .into_iter()
        .map(|r| (r.id, r.similarity))
        .collect()
}

/// GPU-accelerated similarity search
#[cfg(feature = "gpu")]
fn search_with_gpu(
    query: &Hypervector,
    database: &[Hypervector],
    patient_ids: &[String],
    top_k: usize,
) -> Vec<(String, f64)> {
    // Initialize GPU engine
    let engine = match gpu::sync::create_engine() {
        Ok(e) => {
            println!("  {}", "GPU initialized successfully".green());
            e
        }
        Err(e) => {
            eprintln!("{} GPU initialization failed: {}. Falling back to CPU.", "Warning:".yellow(), e);
            // Fallback to CPU
            let mut index = HdcIndex::new();
            for (id, vec) in patient_ids.iter().zip(database.iter()) {
                index.add(id.clone(), vec.clone());
            }
            return search_with_cpu(&index, query, top_k);
        }
    };

    // Run GPU batch similarity
    let queries = vec![query.clone()];
    let top_k_results = match gpu::sync::top_k_similarity(&engine, &queries, database, top_k) {
        Ok(results) => results,
        Err(e) => {
            eprintln!("{} GPU computation failed: {}. Falling back to CPU.", "Warning:".yellow(), e);
            let mut index = HdcIndex::new();
            for (id, vec) in patient_ids.iter().zip(database.iter()) {
                index.add(id.clone(), vec.clone());
            }
            return search_with_cpu(&index, query, top_k);
        }
    };

    // Convert GPU results to output format
    top_k_results.into_iter()
        .next()
        .map(|results| {
            results.into_iter()
                .map(|(idx, sim)| (patient_ids[idx].clone(), sim as f64))
                .collect()
        })
        .unwrap_or_default()
}

// =============================================================================
// Database Building
// =============================================================================

#[derive(Serialize)]
struct DatabaseIndex {
    version: String,
    seed: String,
    dp_epsilon: Option<f64>,
    patient_count: usize,
    patients: Vec<PatientEntry>,
}

#[derive(Serialize)]
struct PatientEntry {
    id: String,
    source_file: String,
    variant_count: usize,
    vector_hex: String,
}

pub fn build_database(
    input_dir: &Path,
    output_dir: &Path,
    dp_epsilon: Option<f64>,
    seed_str: &str,
) {
    println!("{}", "─".repeat(60));
    println!("{}", "BUILDING PATIENT DATABASE".green().bold());
    println!("{}", "─".repeat(60));

    let seed = Seed::from_string(seed_str);
    let encoder = VcfEncoder::new(seed);

    // Create output directory
    fs::create_dir_all(output_dir).expect("Failed to create output directory");

    // Find VCF files
    let vcf_files: Vec<_> = fs::read_dir(input_dir)
        .expect("Failed to read input directory")
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension().map(|ext| ext == "vcf").unwrap_or(false)
        })
        .collect();

    println!("  Input directory: {}", input_dir.display());
    println!("  VCF files found: {}", vcf_files.len().to_string().cyan());
    println!("  Output directory: {}", output_dir.display());

    if let Some(eps) = dp_epsilon {
        println!("  Differential privacy: ε={}", eps);
    }

    let mut patients: Vec<PatientEntry> = Vec::new();
    let mut errors = 0;

    for (idx, entry) in vcf_files.iter().enumerate() {
        let path = entry.path();
        let patient_id = path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("patient_{}", idx));

        print!("  Processing {} [{}/{}]... ",
               patient_id, idx + 1, vcf_files.len());

        // Read and encode
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => {
                println!("{}", "FAILED (read error)".red());
                errors += 1;
                continue;
            }
        };

        let mut reader = match VcfReader::new(BufReader::new(file)) {
            Ok(r) => r,
            Err(_) => {
                println!("{}", "FAILED (parse error)".red());
                errors += 1;
                continue;
            }
        };

        let variants = match reader.read_variants() {
            Ok(v) => v,
            Err(_) => {
                println!("{}", "FAILED (variant error)".red());
                errors += 1;
                continue;
            }
        };

        let encoded = match encoder.encode_variants(&variants) {
            Ok(v) => v,
            Err(_) => {
                println!("{}", "FAILED (encode error)".red());
                errors += 1;
                continue;
            }
        };

        // Apply DP if requested
        let final_vector = if let Some(epsilon) = dp_epsilon {
            let dp_params = DpParams::pure(epsilon);
            DpHypervector::from_vector(&encoded, dp_params, None).vector
        } else {
            encoded
        };

        let hex_vector = final_vector.as_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        patients.push(PatientEntry {
            id: patient_id,
            source_file: path.file_name().unwrap().to_string_lossy().to_string(),
            variant_count: variants.len(),
            vector_hex: hex_vector,
        });

        println!("{} ({} variants)", "OK".green(), variants.len());
    }

    // Write index
    let index = DatabaseIndex {
        version: "1.0".to_string(),
        seed: seed_str.to_string(),
        dp_epsilon,
        patient_count: patients.len(),
        patients,
    };

    let index_path = output_dir.join("index.json");
    let json = serde_json::to_string_pretty(&index).unwrap();
    fs::write(&index_path, json).expect("Failed to write index");

    println!();
    println!("{}", "─".repeat(50));
    println!("  Patients encoded: {}", index.patient_count.to_string().green());
    println!("  Errors: {}", if errors > 0 { errors.to_string().red() } else { "0".to_string().green() });
    println!("  Index saved to: {}", index_path.display());
    println!();
    println!("{}", "Database build complete!".green().bold());
}
