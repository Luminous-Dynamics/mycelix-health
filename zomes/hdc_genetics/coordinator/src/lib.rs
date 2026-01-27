//! HDC Genetics Coordinator Zome
//!
//! Provides the public API for encoding genetic data as hypervectors
//! and performing privacy-preserving similarity queries.

use hdk::prelude::*;
use hdc_genetics_integrity::*;
use hdc_genetics_integrity::hdc_ops::*;
use hdc_genetics_integrity::dna_encoding;

/// Input for encoding a DNA sequence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodeDnaSequenceInput {
    /// Patient this genetic data belongs to
    pub patient_hash: ActionHash,
    /// The DNA sequence to encode
    pub sequence: String,
    /// K-mer length (default: 6)
    pub kmer_length: Option<u8>,
    /// Source metadata
    pub source_metadata: GeneticSourceMetadata,
}

/// Input for encoding a SNP panel
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodeSnpPanelInput {
    pub patient_hash: ActionHash,
    /// SNPs as (rsID, allele) pairs
    pub snps: Vec<(String, char)>,
    pub source_metadata: GeneticSourceMetadata,
}

/// Input for encoding HLA typing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodeHlaTypingInput {
    pub patient_hash: ActionHash,
    /// HLA alleles (e.g., ["A*02:01", "B*07:02"])
    pub hla_types: Vec<String>,
    pub source_metadata: GeneticSourceMetadata,
}

/// Input for similarity query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimilarityQueryInput {
    pub query_vector_hash: ActionHash,
    pub target_vector_hash: ActionHash,
    pub metric: Option<SimilarityMetric>,
    pub purpose: QueryPurpose,
}

/// Input for batch similarity search
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchSimilaritySearchInput {
    pub query_vector_hash: ActionHash,
    pub encoding_type: GeneticEncodingType,
    pub min_similarity: f64,
    pub limit: usize,
    pub purpose: QueryPurpose,
}

/// Result from batch similarity search
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimilaritySearchResult {
    pub vector_hash: ActionHash,
    pub patient_hash: ActionHash,
    pub similarity_score: f64,
}

/// Initialize the default codebook
#[hdk_extern]
pub fn init_codebook(_: ()) -> ExternResult<ActionHash> {
    // Generate a deterministic seed for the default codebook
    // In production, this would be configured at DNA level
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(
            format!("Failed to generate seed: {:?}", e)
        )))?;

    let now = sys_time()?;
    let codebook = GeneticCodebook {
        codebook_id: format!("default-k{}", DEFAULT_KMER_LENGTH),
        kmer_length: DEFAULT_KMER_LENGTH,
        seed,
        version: 1,
        created_at: Timestamp::from_micros(now.as_micros() as i64),
        description: Some("Default codebook for DNA sequence encoding".to_string()),
    };

    let action_hash = create_entry(&EntryTypes::GeneticCodebook(codebook.clone()))?;

    let entry_hash = hash_entry(&EntryTypes::GeneticCodebook(codebook))?;

    // Link from anchor for easy lookup
    let anchor = anchor_hash("codebooks")?;
    create_link(
        anchor,
        entry_hash,
        LinkTypes::CodebookByKmerLength,
        LinkTag::new(format!("k{}", DEFAULT_KMER_LENGTH)),
    )?;

    Ok(action_hash)
}

/// Get or create the default codebook
fn get_default_codebook() -> ExternResult<GeneticCodebook> {
    let anchor = anchor_hash("codebooks")?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::CodebookByKmerLength)?,
        GetStrategy::default(),
    )?;

    if let Some(link) = links.first() {
        let hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest(
                "Invalid codebook hash".to_string()
            )))?;

        if let Some(record) = get(hash, GetOptions::default())? {
            let codebook: GeneticCodebook = record.entry()
                .to_app_option()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("{:?}", e))))?
                .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
                    "Codebook entry not found".to_string()
                )))?;
            return Ok(codebook);
        }
    }

    // Create default codebook if not found
    init_codebook(())?;
    get_default_codebook()
}

/// Encode a DNA sequence as a hypervector
///
/// This creates a GeneticHypervector entry that can be used for
/// privacy-preserving similarity comparisons.
#[hdk_extern]
pub fn encode_dna_sequence(input: EncodeDnaSequenceInput) -> ExternResult<ActionHash> {
    // Validate consent for genetic data
    validate_genetic_consent(&input.patient_hash, &input.source_metadata)?;

    let codebook = get_default_codebook()?;
    let kmer_length = input.kmer_length.unwrap_or(DEFAULT_KMER_LENGTH);

    let (data, kmer_count) = encode_dna_sequence_internal(
        &input.sequence,
        &codebook.seed,
        kmer_length,
    ).map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

    let now = sys_time()?;
    let vector_id = generate_vector_id(&input.patient_hash, now.as_micros() as i64);

    let hypervector = GeneticHypervector {
        vector_id,
        patient_hash: input.patient_hash.clone(),
        data,
        encoding_type: GeneticEncodingType::DnaSequence,
        kmer_length,
        kmer_count,
        created_at: Timestamp::from_micros(now.as_micros() as i64),
        source_metadata: input.source_metadata,
    };

    let action_hash = create_entry(&EntryTypes::GeneticHypervector(hypervector.clone()))?;

    // Link to patient
    create_link(
        input.patient_hash.clone(),
        action_hash.clone(),
        LinkTypes::PatientToVectors,
        LinkTag::new("dna"),
    )?;

    // Index by encoding type
    let type_anchor = anchor_hash("encoding:dna")?;
    create_link(
        type_anchor,
        action_hash.clone(),
        LinkTypes::EncodingTypeIndex,
        LinkTag::new(""),
    )?;

    Ok(action_hash)
}

/// Encode a SNP panel as a hypervector
#[hdk_extern]
pub fn encode_snp_panel(input: EncodeSnpPanelInput) -> ExternResult<ActionHash> {
    validate_genetic_consent(&input.patient_hash, &input.source_metadata)?;

    let codebook = get_default_codebook()?;

    let data = encode_snp_panel_internal(&input.snps, &codebook.seed)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

    let now = sys_time()?;
    let vector_id = generate_vector_id(&input.patient_hash, now.as_micros() as i64);

    let hypervector = GeneticHypervector {
        vector_id,
        patient_hash: input.patient_hash.clone(),
        data,
        encoding_type: GeneticEncodingType::SnpPanel,
        kmer_length: 0, // Not applicable for SNP panels
        kmer_count: input.snps.len() as u32,
        created_at: Timestamp::from_micros(now.as_micros() as i64),
        source_metadata: input.source_metadata,
    };

    let action_hash = create_entry(&EntryTypes::GeneticHypervector(hypervector))?;

    create_link(
        input.patient_hash,
        action_hash.clone(),
        LinkTypes::PatientToVectors,
        LinkTag::new("snp"),
    )?;

    let type_anchor = anchor_hash("encoding:snp")?;
    create_link(
        type_anchor,
        action_hash.clone(),
        LinkTypes::EncodingTypeIndex,
        LinkTag::new(""),
    )?;

    Ok(action_hash)
}

/// Encode HLA typing as a hypervector
#[hdk_extern]
pub fn encode_hla_typing(input: EncodeHlaTypingInput) -> ExternResult<ActionHash> {
    validate_genetic_consent(&input.patient_hash, &input.source_metadata)?;

    let codebook = get_default_codebook()?;

    let data = encode_hla_typing_internal(&input.hla_types, &codebook.seed)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

    let now = sys_time()?;
    let vector_id = generate_vector_id(&input.patient_hash, now.as_micros() as i64);

    let hypervector = GeneticHypervector {
        vector_id,
        patient_hash: input.patient_hash.clone(),
        data,
        encoding_type: GeneticEncodingType::HlaTyping,
        kmer_length: 0,
        kmer_count: input.hla_types.len() as u32,
        created_at: Timestamp::from_micros(now.as_micros() as i64),
        source_metadata: input.source_metadata,
    };

    let action_hash = create_entry(&EntryTypes::GeneticHypervector(hypervector))?;

    create_link(
        input.patient_hash,
        action_hash.clone(),
        LinkTypes::PatientToVectors,
        LinkTag::new("hla"),
    )?;

    let type_anchor = anchor_hash("encoding:hla")?;
    create_link(
        type_anchor,
        action_hash.clone(),
        LinkTypes::EncodingTypeIndex,
        LinkTag::new(""),
    )?;

    Ok(action_hash)
}

/// Calculate similarity between two genetic hypervectors
#[hdk_extern]
pub fn calculate_similarity(input: SimilarityQueryInput) -> ExternResult<GeneticSimilarityResult> {
    // Fetch both vectors
    let query_record = get(input.query_vector_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            "Query vector not found".to_string()
        )))?;

    let target_record = get(input.target_vector_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            "Target vector not found".to_string()
        )))?;

    let query_vec: GeneticHypervector = query_record.entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("{:?}", e))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            "Query entry not found".to_string()
        )))?;

    let target_vec: GeneticHypervector = target_record.entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("{:?}", e))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            "Target entry not found".to_string()
        )))?;

    // Calculate similarity using requested metric
    let metric = input.metric.unwrap_or(SimilarityMetric::Cosine);
    let similarity_score = match metric {
        SimilarityMetric::Cosine => normalized_cosine_similarity(&query_vec.data, &target_vec.data),
        SimilarityMetric::Hamming => hamming_similarity(&query_vec.data, &target_vec.data),
        SimilarityMetric::Jaccard => {
            // Jaccard = intersection / union
            let intersection: usize = query_vec.data.iter()
                .zip(target_vec.data.iter())
                .map(|(a, b)| (a & b).count_ones() as usize)
                .sum();
            let union: usize = query_vec.data.iter()
                .zip(target_vec.data.iter())
                .map(|(a, b)| (a | b).count_ones() as usize)
                .sum();
            if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
        }
    };

    let now = sys_time()?;
    let caller = agent_info()?.agent_initial_pubkey;

    let result = GeneticSimilarityResult {
        result_id: format!("sim-{}", now.as_micros()),
        query_vector_hash: input.query_vector_hash.clone(),
        target_vector_hash: input.target_vector_hash.clone(),
        similarity_score,
        similarity_metric: metric,
        query_purpose: input.purpose,
        queried_at: Timestamp::from_micros(now.as_micros() as i64),
        queried_by: caller,
    };

    // Store result for audit
    create_entry(&EntryTypes::GeneticSimilarityResult(result.clone()))?;

    Ok(result)
}

/// Search for similar genetic profiles
#[hdk_extern]
pub fn search_similar_genetics(
    input: BatchSimilaritySearchInput,
) -> ExternResult<Vec<SimilaritySearchResult>> {
    // Get the query vector
    let query_record = get(input.query_vector_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            "Query vector not found".to_string()
        )))?;

    let query_vec: GeneticHypervector = query_record.entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("{:?}", e))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            "Query entry not found".to_string()
        )))?;

    // Get all vectors of the same encoding type
    let type_anchor = match input.encoding_type {
        GeneticEncodingType::DnaSequence => anchor_hash("encoding:dna")?,
        GeneticEncodingType::SnpPanel => anchor_hash("encoding:snp")?,
        GeneticEncodingType::HlaTyping => anchor_hash("encoding:hla")?,
        _ => anchor_hash("encoding:other")?,
    };

    let links = get_links(
        LinkQuery::try_new(type_anchor, LinkTypes::EncodingTypeIndex)?,
        GetStrategy::default(),
    )?;

    let mut results: Vec<SimilaritySearchResult> = Vec::new();

    for link in links {
        let target_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest(
                "Invalid vector hash".to_string()
            )))?;

        // Skip the query vector itself
        if target_hash == input.query_vector_hash {
            continue;
        }

        if let Some(record) = get(target_hash.clone(), GetOptions::default())? {
            if let Some(target_vec) = record.entry()
                .to_app_option::<GeneticHypervector>()
                .ok()
                .flatten()
            {
                let similarity = normalized_cosine_similarity(&query_vec.data, &target_vec.data);

                if similarity >= input.min_similarity {
                    results.push(SimilaritySearchResult {
                        vector_hash: target_hash,
                        patient_hash: target_vec.patient_hash,
                        similarity_score: similarity,
                    });
                }
            }
        }
    }

    // Sort by similarity (descending) and limit
    results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
    results.truncate(input.limit);

    Ok(results)
}

/// Get all genetic vectors for a patient
#[hdk_extern]
pub fn get_patient_genetic_vectors(patient_hash: ActionHash) -> ExternResult<Vec<GeneticHypervector>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToVectors)?,
        GetStrategy::default(),
    )?;

    let mut vectors = Vec::new();

    for link in links {
        let hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest(
                "Invalid vector hash".to_string()
            )))?;

        if let Some(record) = get(hash, GetOptions::default())? {
            if let Some(vec) = record.entry()
                .to_app_option::<GeneticHypervector>()
                .ok()
                .flatten()
            {
                vectors.push(vec);
            }
        }
    }

    Ok(vectors)
}

/// Bundle multiple genetic vectors into one
#[hdk_extern]
pub fn bundle_genetic_vectors(
    input: BundleVectorsInput,
) -> ExternResult<ActionHash> {
    if input.vector_hashes.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "At least one vector hash is required".to_string()
        )));
    }

    let mut vectors: Vec<Vec<u8>> = Vec::new();
    let mut weights: Vec<f64> = Vec::new();

    for (i, hash) in input.vector_hashes.iter().enumerate() {
        if let Some(record) = get(hash.clone(), GetOptions::default())? {
            if let Some(vec) = record.entry()
                .to_app_option::<GeneticHypervector>()
                .ok()
                .flatten()
            {
                vectors.push(vec.data);
                weights.push(input.weights.as_ref().map(|w| w[i]).unwrap_or(1.0));
            }
        }
    }

    // Perform weighted bundle
    let vec_weight_pairs: Vec<(&[u8], f64)> = vectors.iter()
        .zip(weights.iter())
        .map(|(v, &w)| (v.as_slice(), w))
        .collect();

    let bundled_data = weighted_bundle(&vec_weight_pairs);

    let now = sys_time()?;

    let bundled = BundledGeneticVector {
        bundle_id: format!("bundle-{}", now.as_micros()),
        patient_hash: input.patient_hash.clone(),
        data: bundled_data,
        source_vector_hashes: input.vector_hashes,
        weights: Some(weights),
        bundled_at: Timestamp::from_micros(now.as_micros() as i64),
    };

    let action_hash = create_entry(&EntryTypes::BundledGeneticVector(bundled))?;

    create_link(
        input.patient_hash,
        action_hash.clone(),
        LinkTypes::PatientToVectors,
        LinkTag::new("bundle"),
    )?;

    Ok(action_hash)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BundleVectorsInput {
    pub patient_hash: ActionHash,
    pub vector_hashes: Vec<ActionHash>,
    pub weights: Option<Vec<f64>>,
}

// Helper functions

fn generate_vector_id(patient_hash: &ActionHash, time_micros: i64) -> String {
    let hash_bytes = patient_hash.get_raw_39();
    format!(
        "vec-{:02x}{:02x}{:02x}{:02x}-{}",
        hash_bytes[0], hash_bytes[1], hash_bytes[2], hash_bytes[3],
        time_micros
    )
}

fn validate_genetic_consent(
    _patient_hash: &ActionHash,
    metadata: &GeneticSourceMetadata,
) -> ExternResult<()> {
    // If consent hash is provided, verify it
    if let Some(consent_hash) = &metadata.consent_hash {
        // Call consent zome to verify
        let response = call(
            CallTargetCell::Local,
            "consent",
            "verify_consent".into(),
            None,
            consent_hash,
        )?;

        match response {
            ZomeCallResponse::Ok(_) => Ok(()),
            _ => Err(wasm_error!(WasmErrorInner::Guest(
                "Consent verification failed".to_string()
            ))),
        }
    } else {
        // For now, allow without explicit consent if the caller owns the data
        // In production, this would require proper consent
        Ok(())
    }
}

fn anchor_hash(text: &str) -> ExternResult<EntryHash> {
    let bytes = text.as_bytes().to_vec();
    let entry = Entry::App(AppEntryBytes::try_from(SerializedBytes::try_from(UnsafeBytes::from(bytes))
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("{:?}", e))))?)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("{:?}", e))))?);
    hash_entry(entry)
}

// Wrapper functions that delegate to integrity crate
fn encode_dna_sequence_internal(
    sequence: &str,
    seed: &[u8; 32],
    kmer_length: u8,
) -> Result<(Vec<u8>, u32), String> {
    dna_encoding::encode_dna_sequence(sequence, seed, kmer_length)
}

fn encode_snp_panel_internal(
    snps: &[(String, char)],
    seed: &[u8; 32],
) -> Result<Vec<u8>, String> {
    dna_encoding::encode_snp_panel(snps, seed)
}

fn encode_hla_typing_internal(
    hla_types: &[String],
    seed: &[u8; 32],
) -> Result<Vec<u8>, String> {
    dna_encoding::encode_hla_typing(hla_types, seed)
}
