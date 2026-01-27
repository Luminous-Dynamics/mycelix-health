//! HDC Genetics Integrity Zome
//!
//! Hyperdimensional Computing for genetic data in Mycelix-Health.
//! Enables privacy-preserving genetic similarity searches without
//! exposing raw genetic sequences.
//!
//! ## Key Concepts
//!
//! - **Hypervectors**: High-dimensional (10,000-D) binary/bipolar vectors
//! - **K-mers**: DNA subsequences of length k used as encoding units
//! - **Binding**: XOR operation to combine information
//! - **Bundling**: Majority voting to aggregate multiple vectors
//! - **Similarity**: Cosine/Hamming distance for comparisons
//!
//! This zome uses the shared `hdc-core` library to ensure consistent
//! encoding across all platforms (native experiments, Holochain, WASM).

use hdi::prelude::*;

// Re-export constants from hdc-core for consistency
pub use hdc_core::{HYPERVECTOR_DIM, HYPERVECTOR_BYTES, DEFAULT_KMER_LENGTH};

/// A hypervector representing encoded genetic data
///
/// Uses bipolar representation internally (-1/+1) but stores as bits
/// for efficiency. The 10,000 dimensions are stored in 1,250 bytes.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GeneticHypervector {
    /// Unique identifier
    pub vector_id: String,
    /// Patient this genetic data belongs to
    pub patient_hash: ActionHash,
    /// The hypervector data (packed bits, 10,000 dims = 1,250 bytes)
    pub data: Vec<u8>,
    /// What type of genetic data is encoded
    pub encoding_type: GeneticEncodingType,
    /// K-mer length used for encoding
    pub kmer_length: u8,
    /// Number of k-mers encoded (for normalization)
    pub kmer_count: u32,
    /// Creation timestamp
    pub created_at: Timestamp,
    /// Metadata about the source
    pub source_metadata: GeneticSourceMetadata,
}

/// Types of genetic data that can be encoded
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GeneticEncodingType {
    /// Full DNA sequence (or segment)
    DnaSequence,
    /// Single Nucleotide Polymorphisms
    SnpPanel,
    /// HLA typing for transplant matching
    HlaTyping,
    /// Pharmacogenomic markers (drug response)
    Pharmacogenomics,
    /// Disease risk markers
    DiseaseRisk,
    /// Ancestry markers
    Ancestry,
    /// Custom panel of genes
    GenePanel(String),
}

/// Metadata about the genetic data source
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GeneticSourceMetadata {
    /// Source system (lab, sequencing provider)
    pub source_system: String,
    /// Date of genetic test
    pub test_date: Option<Timestamp>,
    /// Type of sequencing (WGS, WES, SNP array, etc.)
    pub sequencing_method: Option<String>,
    /// Quality score (if available)
    pub quality_score: Option<f64>,
    /// Consent hash authorizing this use
    pub consent_hash: Option<ActionHash>,
}

/// A similarity query result
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GeneticSimilarityResult {
    /// Unique identifier
    pub result_id: String,
    /// The query vector hash
    pub query_vector_hash: ActionHash,
    /// The target vector hash
    pub target_vector_hash: ActionHash,
    /// Similarity score (0.0 to 1.0)
    pub similarity_score: f64,
    /// Type of similarity metric used
    pub similarity_metric: SimilarityMetric,
    /// Query purpose (for audit)
    pub query_purpose: QueryPurpose,
    /// Timestamp of query
    pub queried_at: Timestamp,
    /// Agent who performed query
    pub queried_by: AgentPubKey,
}

/// Similarity metrics for comparing hypervectors
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SimilarityMetric {
    /// Cosine similarity (bipolar interpretation)
    Cosine,
    /// Hamming distance (normalized)
    Hamming,
    /// Jaccard index for binary vectors
    Jaccard,
}

/// Purpose of a genetic similarity query
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum QueryPurpose {
    /// Finding compatible organ donors
    OrganDonorMatching,
    /// HLA matching for transplant
    HlaMatching,
    /// Predicting drug response
    PharmacogenomicPrediction,
    /// Assessing disease risk
    DiseaseRiskAssessment,
    /// Finding genetically similar trial participants
    ClinicalTrialMatching,
    /// Research use with consent
    Research(String),
    /// Other purpose (must specify)
    Other(String),
}

/// An item codebook mapping genetic elements to hypervectors
///
/// This stores the random seed used to generate consistent
/// hypervectors for each k-mer or genetic marker.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GeneticCodebook {
    /// Codebook identifier
    pub codebook_id: String,
    /// K-mer length this codebook is for
    pub kmer_length: u8,
    /// Random seed used for generation (enables reproducibility)
    pub seed: [u8; 32],
    /// Version of the codebook
    pub version: u32,
    /// Creation timestamp
    pub created_at: Timestamp,
    /// Optional description
    pub description: Option<String>,
}

/// A bundled (aggregated) hypervector from multiple sources
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct BundledGeneticVector {
    /// Unique identifier
    pub bundle_id: String,
    /// Patient this belongs to
    pub patient_hash: ActionHash,
    /// The bundled hypervector data
    pub data: Vec<u8>,
    /// Hashes of vectors that were bundled
    pub source_vector_hashes: Vec<ActionHash>,
    /// Weights used for bundling (if weighted)
    pub weights: Option<Vec<f64>>,
    /// Bundle timestamp
    pub bundled_at: Timestamp,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    GeneticHypervector(GeneticHypervector),
    GeneticSimilarityResult(GeneticSimilarityResult),
    GeneticCodebook(GeneticCodebook),
    BundledGeneticVector(BundledGeneticVector),
}

#[hdk_link_types]
pub enum LinkTypes {
    /// Patient to their genetic hypervectors
    PatientToVectors,
    /// Codebook lookups
    CodebookByKmerLength,
    /// Similarity query results
    VectorToSimilarityResults,
    /// Bundle components
    BundleToSources,
    /// Index by encoding type
    EncodingTypeIndex,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::GeneticHypervector(v) => validate_hypervector(&v),
                EntryTypes::GeneticSimilarityResult(r) => validate_similarity_result(&r),
                EntryTypes::GeneticCodebook(c) => validate_codebook(&c),
                EntryTypes::BundledGeneticVector(b) => validate_bundled_vector(&b),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_hypervector(v: &GeneticHypervector) -> ExternResult<ValidateCallbackResult> {
    // Validate vector ID
    if v.vector_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Vector ID is required".to_string(),
        ));
    }

    // Validate hypervector size (10,000 bits = 1,250 bytes)
    let expected_bytes = (HYPERVECTOR_DIM + 7) / 8; // Round up to nearest byte
    if v.data.len() != expected_bytes {
        return Ok(ValidateCallbackResult::Invalid(
            format!(
                "Hypervector must be {} bytes ({} dimensions), got {} bytes",
                expected_bytes, HYPERVECTOR_DIM, v.data.len()
            ),
        ));
    }

    // Validate k-mer length (reasonable range: 3-12)
    if v.kmer_length < 3 || v.kmer_length > 12 {
        return Ok(ValidateCallbackResult::Invalid(
            format!("K-mer length must be between 3 and 12, got {}", v.kmer_length),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_similarity_result(r: &GeneticSimilarityResult) -> ExternResult<ValidateCallbackResult> {
    if r.result_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Result ID is required".to_string(),
        ));
    }

    // Similarity must be between 0 and 1
    if r.similarity_score < 0.0 || r.similarity_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Similarity score must be between 0.0 and 1.0".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_codebook(c: &GeneticCodebook) -> ExternResult<ValidateCallbackResult> {
    if c.codebook_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Codebook ID is required".to_string(),
        ));
    }

    if c.kmer_length < 3 || c.kmer_length > 12 {
        return Ok(ValidateCallbackResult::Invalid(
            format!("K-mer length must be between 3 and 12, got {}", c.kmer_length),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

fn validate_bundled_vector(b: &BundledGeneticVector) -> ExternResult<ValidateCallbackResult> {
    if b.bundle_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Bundle ID is required".to_string(),
        ));
    }

    let expected_bytes = (HYPERVECTOR_DIM + 7) / 8;
    if b.data.len() != expected_bytes {
        return Ok(ValidateCallbackResult::Invalid(
            format!(
                "Bundled vector must be {} bytes, got {}",
                expected_bytes, b.data.len()
            ),
        ));
    }

    if b.source_vector_hashes.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one source vector is required for bundling".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// HDC operations module - delegates to hdc-core for consistent encoding
///
/// This module re-exports functions from hdc-core to ensure the same
/// hypervector algorithms are used across:
/// - Native Rust experiments
/// - Holochain WASM zomes
/// - Any other platform
pub mod hdc_ops {
    pub use hdc_core::ops::{
        generate_item_vector,
        bind,
        bundle,
        weighted_bundle,
        permute,
        hamming_similarity,
        cosine_similarity,
        normalized_cosine_similarity,
    };
}

/// DNA encoding functions - delegates to hdc-core for consistent encoding
///
/// These wrapper functions adapt the hdc-core encoding API to the
/// simpler interface expected by the zome coordinator.
pub mod dna_encoding {
    use hdc_core::encoding::{DnaEncoder, HlaEncoder, SnpEncoder};
    use hdc_core::Seed;

    /// Valid DNA nucleotides
    pub const NUCLEOTIDES: &[char] = &['A', 'C', 'G', 'T'];

    /// Encode a DNA sequence as a hypervector
    ///
    /// Uses positional k-mer encoding via hdc-core:
    /// 1. Extract all k-mers from sequence
    /// 2. For each k-mer, bind its item vector with a position vector
    /// 3. Bundle all position-bound k-mer vectors
    pub fn encode_dna_sequence(
        sequence: &str,
        codebook_seed: &[u8; 32],
        kmer_length: u8,
    ) -> Result<(Vec<u8>, u32), String> {
        let seed = Seed::from_bytes(*codebook_seed);
        let encoder = DnaEncoder::new(seed, kmer_length);

        encoder
            .encode_sequence(sequence)
            .map(|enc| (enc.vector.as_bytes().to_vec(), enc.kmer_count))
            .map_err(|e| e.to_string())
    }

    /// Encode a set of SNPs as a hypervector
    ///
    /// SNPs are represented as rsID:allele pairs (e.g., "rs1234:A")
    pub fn encode_snp_panel(
        snps: &[(String, char)], // (rsID, allele)
        codebook_seed: &[u8; 32],
    ) -> Result<Vec<u8>, String> {
        let seed = Seed::from_bytes(*codebook_seed);
        let encoder = SnpEncoder::new(seed);

        // Convert to the format expected by SnpEncoder
        let snp_tuples: Vec<(&str, char)> = snps
            .iter()
            .map(|(rsid, allele)| (rsid.as_str(), *allele))
            .collect();

        encoder
            .encode_panel(&snp_tuples)
            .map(|hv| hv.as_bytes().to_vec())
            .map_err(|e| e.to_string())
    }

    /// Encode HLA types as a hypervector
    ///
    /// HLA types are represented as locus:allele pairs (e.g., "A*02:01")
    pub fn encode_hla_typing(
        hla_types: &[String],
        codebook_seed: &[u8; 32],
    ) -> Result<Vec<u8>, String> {
        let seed = Seed::from_bytes(*codebook_seed);
        let encoder = HlaEncoder::new(seed);

        let hla_refs: Vec<&str> = hla_types.iter().map(|s| s.as_str()).collect();

        encoder
            .encode_typing(&hla_refs)
            .map(|hv| hv.as_bytes().to_vec())
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::hdc_ops::*;
    use super::dna_encoding::*;

    #[test]
    fn test_hypervector_size() {
        let seed = [0u8; 32];
        let vec = generate_item_vector(&seed, "ACGT");
        assert_eq!(vec.len(), HYPERVECTOR_BYTES);
    }

    #[test]
    fn test_bind_is_self_inverse() {
        let seed = [0u8; 32];
        let a = generate_item_vector(&seed, "ACGT");
        let b = generate_item_vector(&seed, "TGCA");

        let bound = bind(&a, &b);
        let unbound = bind(&bound, &b);

        // XOR is self-inverse: (a XOR b) XOR b = a
        assert_eq!(a, unbound);
    }

    #[test]
    fn test_similarity_self() {
        let seed = [0u8; 32];
        let vec = generate_item_vector(&seed, "ACGTACGT");

        let sim = hamming_similarity(&vec, &vec);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_dna_encoding() {
        let seed = [42u8; 32];
        let sequence = "ACGTACGTACGT";

        let result = encode_dna_sequence(sequence, &seed, 6);
        assert!(result.is_ok());

        let (vec, count) = result.unwrap();
        assert_eq!(vec.len(), HYPERVECTOR_BYTES);
        assert_eq!(count, 7); // 12 - 6 + 1 = 7 k-mers
    }

    #[test]
    fn test_similar_sequences_have_high_similarity() {
        let seed = [42u8; 32];
        let seq1 = "ACGTACGTACGTACGTACGT";
        let seq2 = "ACGTACGTACGTACGTACGT"; // Identical
        let seq3 = "TGCATGCATGCATGCATGCA"; // Very different

        let (vec1, _) = encode_dna_sequence(seq1, &seed, 6).unwrap();
        let (vec2, _) = encode_dna_sequence(seq2, &seed, 6).unwrap();
        let (vec3, _) = encode_dna_sequence(seq3, &seed, 6).unwrap();

        let sim_identical = hamming_similarity(&vec1, &vec2);
        let sim_different = hamming_similarity(&vec1, &vec3);

        // Identical sequences should have very high similarity
        assert!(sim_identical > 0.99);
        // Different sequences should have lower similarity
        assert!(sim_different < sim_identical);
    }

    #[test]
    fn test_hdc_core_consistency() {
        // Verify that zome encoding matches hdc-core encoding
        use hdc_core::{encoding::DnaEncoder, Seed as HdcSeed};

        let seed_bytes = [42u8; 32];
        let sequence = "ACGTACGTACGTACGTACGT";

        // Encode via zome wrapper
        let (zome_vec, zome_count) = encode_dna_sequence(sequence, &seed_bytes, 6).unwrap();

        // Encode via hdc-core directly
        let hdc_seed = HdcSeed::from_bytes(seed_bytes);
        let encoder = DnaEncoder::new(hdc_seed, 6);
        let hdc_result = encoder.encode_sequence(sequence).unwrap();

        // They should be identical
        assert_eq!(zome_vec, hdc_result.vector.as_bytes());
        assert_eq!(zome_count, hdc_result.kmer_count);
    }
}
