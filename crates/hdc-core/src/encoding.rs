//! Genetic data encoding
//!
//! Functions for encoding DNA sequences, SNP panels, and HLA types
//! as hypervectors.

use crate::{bundle, HdcError, Hypervector, Seed};

/// Valid DNA nucleotides
pub const NUCLEOTIDES: &[char] = &['A', 'C', 'G', 'T'];

/// DNA sequence encoder
pub struct DnaEncoder {
    seed: Seed,
    kmer_length: u8,
}

impl DnaEncoder {
    /// Create a new DNA encoder
    pub fn new(seed: Seed, kmer_length: u8) -> Self {
        DnaEncoder { seed, kmer_length }
    }

    /// Get the k-mer length
    pub fn kmer_length(&self) -> u8 {
        self.kmer_length
    }

    /// Encode a DNA sequence as a hypervector
    ///
    /// Uses positional k-mer encoding:
    /// 1. Extract all k-mers from sequence
    /// 2. For each k-mer, bind its item vector with a position vector
    /// 3. Bundle all position-bound k-mer vectors
    pub fn encode_sequence(&self, sequence: &str) -> Result<EncodedSequence, HdcError> {
        let seq = sequence.to_uppercase();
        let k = self.kmer_length as usize;

        if seq.len() < k {
            return Err(HdcError::SequenceTooShort {
                length: seq.len(),
                kmer_length: self.kmer_length,
            });
        }

        // Validate sequence
        for c in seq.chars() {
            if !NUCLEOTIDES.contains(&c) {
                return Err(HdcError::InvalidNucleotide(c));
            }
        }

        let mut kmer_vectors: Vec<Hypervector> = Vec::new();
        let mut kmer_count = 0u32;

        for i in 0..=(seq.len() - k) {
            let kmer = &seq[i..i + k];

            // Generate item vector for this k-mer
            let item_vec = Hypervector::random(&self.seed, kmer);

            // Permute by position (positional encoding)
            let position_vec = item_vec.permute(i);

            kmer_vectors.push(position_vec);
            kmer_count += 1;
        }

        if kmer_vectors.is_empty() {
            return Err(HdcError::EmptyInput);
        }

        // Bundle all k-mer vectors
        let refs: Vec<&Hypervector> = kmer_vectors.iter().collect();
        let vector = bundle(&refs);

        Ok(EncodedSequence {
            vector,
            kmer_count,
            kmer_length: self.kmer_length,
            sequence_length: seq.len(),
        })
    }

    /// Encode multiple sequences and return their vectors
    pub fn encode_batch(&self, sequences: &[&str]) -> Vec<Result<EncodedSequence, HdcError>> {
        sequences.iter().map(|seq| self.encode_sequence(seq)).collect()
    }

    /// Encode multiple sequences in parallel (requires "parallel" feature)
    #[cfg(feature = "parallel")]
    pub fn encode_batch_parallel(&self, sequences: &[&str]) -> Vec<Result<EncodedSequence, HdcError>> {
        use rayon::prelude::*;
        sequences.par_iter().map(|seq| self.encode_sequence(seq)).collect()
    }

    /// Encode a DNA sequence with pre-computed k-mer codebook for faster encoding
    /// This is more efficient when encoding many sequences with the same parameters
    pub fn encode_with_codebook(&self, sequence: &str, codebook: &KmerCodebook) -> Result<EncodedSequence, HdcError> {
        let seq = sequence.to_uppercase();
        let k = self.kmer_length as usize;

        if seq.len() < k {
            return Err(HdcError::SequenceTooShort {
                length: seq.len(),
                kmer_length: self.kmer_length,
            });
        }

        // Validate sequence
        for c in seq.chars() {
            if !NUCLEOTIDES.contains(&c) {
                return Err(HdcError::InvalidNucleotide(c));
            }
        }

        let mut kmer_vectors: Vec<Hypervector> = Vec::new();
        let mut kmer_count = 0u32;

        for i in 0..=(seq.len() - k) {
            let kmer = &seq[i..i + k];

            // Look up pre-computed item vector
            if let Some(item_vec) = codebook.get(kmer) {
                // Permute by position
                let position_vec = item_vec.permute(i);
                kmer_vectors.push(position_vec);
                kmer_count += 1;
            }
        }

        if kmer_vectors.is_empty() {
            return Err(HdcError::EmptyInput);
        }

        let refs: Vec<&Hypervector> = kmer_vectors.iter().collect();
        let vector = bundle(&refs);

        Ok(EncodedSequence {
            vector,
            kmer_count,
            kmer_length: self.kmer_length,
            sequence_length: seq.len(),
        })
    }

    /// Create a k-mer codebook for fast encoding
    pub fn create_codebook(&self) -> KmerCodebook {
        KmerCodebook::new(&self.seed, self.kmer_length)
    }
}

/// Pre-computed k-mer to hypervector mapping for fast encoding
pub struct KmerCodebook {
    vectors: std::collections::HashMap<String, Hypervector>,
}

impl KmerCodebook {
    /// Generate all possible k-mer vectors
    pub fn new(seed: &Seed, k: u8) -> Self {
        let mut vectors = std::collections::HashMap::new();
        let kmers = Self::generate_all_kmers(k as usize);

        for kmer in kmers {
            let vec = Hypervector::random(seed, &kmer);
            vectors.insert(kmer, vec);
        }

        vectors.into()
    }

    fn generate_all_kmers(k: usize) -> Vec<String> {
        if k == 0 {
            return vec![String::new()];
        }

        let smaller = Self::generate_all_kmers(k - 1);
        let mut result = Vec::with_capacity(4usize.pow(k as u32));

        for base in NUCLEOTIDES {
            for kmer in &smaller {
                result.push(format!("{}{}", base, kmer));
            }
        }

        result
    }

    /// Get the hypervector for a k-mer
    pub fn get(&self, kmer: &str) -> Option<&Hypervector> {
        self.vectors.get(kmer)
    }

    /// Number of k-mers in the codebook
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Check if codebook is empty
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }
}

impl From<std::collections::HashMap<String, Hypervector>> for KmerCodebook {
    fn from(vectors: std::collections::HashMap<String, Hypervector>) -> Self {
        Self { vectors }
    }
}

/// Result of encoding a DNA sequence
#[derive(Clone, Debug)]
pub struct EncodedSequence {
    /// The hypervector
    pub vector: Hypervector,
    /// Number of k-mers encoded
    pub kmer_count: u32,
    /// K-mer length used
    pub kmer_length: u8,
    /// Original sequence length
    pub sequence_length: usize,
}

/// HLA typing encoder for transplant matching
pub struct HlaEncoder {
    seed: Seed,
}

impl HlaEncoder {
    /// Create a new HLA encoder
    pub fn new(seed: Seed) -> Self {
        HlaEncoder { seed }
    }

    /// Encode a set of HLA types as a hypervector
    ///
    /// HLA types should be in standard format, e.g., "A*02:01", "B*07:02"
    pub fn encode_typing(&self, hla_types: &[&str]) -> Result<Hypervector, HdcError> {
        if hla_types.is_empty() {
            return Err(HdcError::EmptyInput);
        }

        let mut hla_vectors: Vec<Hypervector> = Vec::new();

        for hla in hla_types {
            let hla_key = format!("HLA:{}", hla);
            let hla_vec = Hypervector::random(&self.seed, &hla_key);
            hla_vectors.push(hla_vec);
        }

        let refs: Vec<&Hypervector> = hla_vectors.iter().collect();
        Ok(bundle(&refs))
    }

    /// Calculate HLA match score between two typings
    ///
    /// Uses normalized cosine similarity, returning 0.0-1.0
    pub fn match_score(&self, typing1: &[&str], typing2: &[&str]) -> Result<f64, HdcError> {
        let hv1 = self.encode_typing(typing1)?;
        let hv2 = self.encode_typing(typing2)?;
        Ok(hv1.normalized_cosine_similarity(&hv2))
    }

    /// Find best matches from a list of potential donors
    pub fn find_best_matches(
        &self,
        recipient: &[&str],
        donors: &[(&str, &[&str])], // (donor_id, hla_types)
        top_k: usize,
    ) -> Result<Vec<HlaMatch>, HdcError> {
        let recipient_hv = self.encode_typing(recipient)?;

        let mut matches: Vec<HlaMatch> = donors
            .iter()
            .filter_map(|(id, types)| {
                let donor_hv = self.encode_typing(types).ok()?;
                let score = recipient_hv.normalized_cosine_similarity(&donor_hv);
                Some(HlaMatch {
                    donor_id: id.to_string(),
                    score,
                })
            })
            .collect();

        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        matches.truncate(top_k);

        Ok(matches)
    }
}

/// Result of HLA matching
#[derive(Clone, Debug)]
pub struct HlaMatch {
    /// Donor identifier
    pub donor_id: String,
    /// Match score (0.0-1.0)
    pub score: f64,
}

/// Locus-weighted HLA encoder for clinically accurate matching
///
/// Encodes each HLA locus separately and combines scores with clinical weights.
/// Class II (DRB1, DQB1) mismatches are weighted higher than Class I (A, B, C).
pub struct LocusWeightedHlaEncoder {
    seed: Seed,
    /// Weights per locus: (A, B, C, DRB1, DQB1)
    /// Default: Class II weighted 2x Class I
    weights: [f64; 5],
}

impl LocusWeightedHlaEncoder {
    /// Create encoder with default clinical weights
    /// DRB1 and DQB1 weighted 2x higher than A, B, C
    pub fn new(seed: Seed) -> Self {
        LocusWeightedHlaEncoder {
            seed,
            weights: [1.0, 1.0, 1.0, 2.0, 2.0], // A, B, C, DRB1, DQB1
        }
    }

    /// Create encoder with custom locus weights
    pub fn with_weights(seed: Seed, weights: [f64; 5]) -> Self {
        LocusWeightedHlaEncoder { seed, weights }
    }

    /// Encode a complete HLA typing with per-locus vectors
    ///
    /// Expects 10 alleles in order: A1, A2, B1, B2, C1, C2, DRB1-1, DRB1-2, DQB1-1, DQB1-2
    pub fn encode_typing(&self, hla_types: &[&str]) -> Result<LocusEncodedHla, HdcError> {
        if hla_types.len() != 10 {
            return Err(HdcError::Other(format!(
                "Expected 10 HLA alleles (2 per locus), got {}",
                hla_types.len()
            )));
        }

        let mut locus_vectors = Vec::with_capacity(5);

        // Encode each locus pair as a bundled vector
        for locus_idx in 0..5 {
            let allele1 = hla_types[locus_idx * 2];
            let allele2 = hla_types[locus_idx * 2 + 1];

            let locus_name = match locus_idx {
                0 => "A",
                1 => "B",
                2 => "C",
                3 => "DRB1",
                4 => "DQB1",
                _ => "UNK",
            };

            // Generate vectors for each allele with locus-specific prefix
            let key1 = format!("HLA-{}:{}", locus_name, allele1);
            let key2 = format!("HLA-{}:{}", locus_name, allele2);

            let vec1 = Hypervector::random(&self.seed, &key1);
            let vec2 = Hypervector::random(&self.seed, &key2);

            // Bundle the two alleles for this locus
            let locus_vec = bundle(&[&vec1, &vec2]);
            locus_vectors.push(locus_vec);
        }

        Ok(LocusEncodedHla {
            locus_vectors,
            weights: self.weights,
        })
    }

    /// Calculate weighted match score between two HLA typings
    pub fn match_score(&self, typing1: &[&str], typing2: &[&str]) -> Result<f64, HdcError> {
        let enc1 = self.encode_typing(typing1)?;
        let enc2 = self.encode_typing(typing2)?;
        Ok(enc1.weighted_similarity(&enc2))
    }

    /// Find best matches from a donor pool using locus-weighted scoring
    pub fn find_best_matches(
        &self,
        recipient: &[&str],
        donors: &[(&str, &[&str])],
        top_k: usize,
    ) -> Result<Vec<HlaMatch>, HdcError> {
        let recipient_enc = self.encode_typing(recipient)?;

        let mut matches: Vec<HlaMatch> = donors
            .iter()
            .filter_map(|(id, types)| {
                let donor_enc = self.encode_typing(types).ok()?;
                let score = recipient_enc.weighted_similarity(&donor_enc);
                Some(HlaMatch {
                    donor_id: id.to_string(),
                    score,
                })
            })
            .collect();

        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        matches.truncate(top_k);

        Ok(matches)
    }
}

/// Per-locus encoded HLA typing
#[derive(Clone, Debug)]
pub struct LocusEncodedHla {
    /// Vectors for each locus: A, B, C, DRB1, DQB1
    pub locus_vectors: Vec<Hypervector>,
    /// Weights for each locus
    pub weights: [f64; 5],
}

impl LocusEncodedHla {
    /// Calculate weighted similarity to another HLA encoding
    pub fn weighted_similarity(&self, other: &LocusEncodedHla) -> f64 {
        let total_weight: f64 = self.weights.iter().sum();
        let mut weighted_sum = 0.0;

        for i in 0..5 {
            let sim = self.locus_vectors[i].normalized_cosine_similarity(&other.locus_vectors[i]);
            weighted_sum += sim * self.weights[i];
        }

        weighted_sum / total_weight
    }

    /// Get per-locus similarity breakdown
    pub fn per_locus_similarity(&self, other: &LocusEncodedHla) -> [f64; 5] {
        let mut sims = [0.0; 5];
        for i in 0..5 {
            sims[i] = self.locus_vectors[i].normalized_cosine_similarity(&other.locus_vectors[i]);
        }
        sims
    }
}

/// Allele-level HLA encoder for maximum precision
///
/// Encodes each allele as a separate hypervector, enabling exact
/// allele matching comparison rather than bundled similarity.
pub struct AlleleHlaEncoder {
    seed: Seed,
    /// Weights per locus: (A, B, C, DRB1, DQB1)
    weights: [f64; 5],
}

impl AlleleHlaEncoder {
    /// Create encoder with default clinical weights
    pub fn new(seed: Seed) -> Self {
        AlleleHlaEncoder {
            seed,
            // Clinical weights: DRB1 most important, then DQB1, then Class I
            weights: [1.0, 1.0, 0.5, 2.0, 1.5], // A, B, C, DRB1, DQB1
        }
    }

    /// Encode a complete HLA typing with per-allele vectors
    ///
    /// Expects 10 alleles in order: A1, A2, B1, B2, C1, C2, DRB1-1, DRB1-2, DQB1-1, DQB1-2
    pub fn encode_typing(&self, hla_types: &[&str]) -> Result<AlleleEncodedHla, HdcError> {
        if hla_types.len() != 10 {
            return Err(HdcError::Other(format!(
                "Expected 10 HLA alleles, got {}",
                hla_types.len()
            )));
        }

        let mut allele_vectors = Vec::with_capacity(10);

        for (i, allele) in hla_types.iter().enumerate() {
            let locus_idx = i / 2;
            let locus_name = match locus_idx {
                0 => "A", 1 => "B", 2 => "C", 3 => "DRB1", 4 => "DQB1",
                _ => "UNK",
            };

            // Key includes locus to prevent cross-locus matches
            let key = format!("ALLELE:{}:{}", locus_name, allele);
            let vec = Hypervector::random(&self.seed, &key);
            allele_vectors.push(vec);
        }

        Ok(AlleleEncodedHla {
            allele_vectors,
            weights: self.weights,
        })
    }

    /// Find best matches using allele-level comparison
    pub fn find_best_matches(
        &self,
        recipient: &[&str],
        donors: &[(&str, &[&str])],
        top_k: usize,
    ) -> Result<Vec<HlaMatch>, HdcError> {
        let recipient_enc = self.encode_typing(recipient)?;

        let mut matches: Vec<HlaMatch> = donors
            .iter()
            .filter_map(|(id, types)| {
                let donor_enc = self.encode_typing(types).ok()?;
                let score = recipient_enc.match_score(&donor_enc);
                Some(HlaMatch {
                    donor_id: id.to_string(),
                    score,
                })
            })
            .collect();

        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        matches.truncate(top_k);
        Ok(matches)
    }
}

/// Allele-level encoded HLA typing
#[derive(Clone, Debug)]
pub struct AlleleEncodedHla {
    /// Vector for each allele (10 total: 2 per locus)
    pub allele_vectors: Vec<Hypervector>,
    /// Weights per locus (applied to each allele pair)
    pub weights: [f64; 5],
}

impl AlleleEncodedHla {
    /// Calculate match score using allele-level comparison
    ///
    /// For each locus, finds the best allele matches (handling heterozygosity)
    /// and weights by clinical importance.
    pub fn match_score(&self, other: &AlleleEncodedHla) -> f64 {
        let mut total_score = 0.0;
        let mut total_weight = 0.0;

        // Process each locus (2 alleles each)
        for locus_idx in 0..5 {
            let weight = self.weights[locus_idx];
            let base = locus_idx * 2;

            // Get allele vectors for this locus
            let self_a1 = &self.allele_vectors[base];
            let self_a2 = &self.allele_vectors[base + 1];
            let other_a1 = &other.allele_vectors[base];
            let other_a2 = &other.allele_vectors[base + 1];

            // Find best matching: each self allele should match one other allele
            // Similarity > 0.9 indicates same allele (due to deterministic encoding)
            let sim_11 = self_a1.normalized_cosine_similarity(other_a1);
            let sim_12 = self_a1.normalized_cosine_similarity(other_a2);
            let sim_21 = self_a2.normalized_cosine_similarity(other_a1);
            let sim_22 = self_a2.normalized_cosine_similarity(other_a2);

            // Count matches: similarity > 0.99 = same allele (deterministic vectors)
            let threshold = 0.999;
            let mut locus_matches = 0.0;

            // Best matching for allele 1
            if sim_11 > threshold || sim_12 > threshold {
                locus_matches += 1.0;
            }
            // Best matching for allele 2 (must match different allele if allele 1 matched)
            if sim_11 > threshold && sim_22 > threshold {
                locus_matches += 1.0;
            } else if sim_12 > threshold && sim_21 > threshold {
                locus_matches += 1.0;
            } else if sim_21 > threshold || sim_22 > threshold {
                // Only count if allele 1 didn't already take this match
                if !(sim_11 > threshold && sim_21 > threshold) && !(sim_12 > threshold && sim_22 > threshold) {
                    locus_matches += 1.0;
                }
            }

            // Normalize: 2 matches per locus = 1.0 score for that locus
            total_score += (locus_matches / 2.0) * weight;
            total_weight += weight;
        }

        total_score / total_weight
    }
}

/// SNP panel encoder
pub struct SnpEncoder {
    seed: Seed,
}

impl SnpEncoder {
    /// Create a new SNP encoder
    pub fn new(seed: Seed) -> Self {
        SnpEncoder { seed }
    }

    /// Encode a set of SNPs as a hypervector
    ///
    /// SNPs are (rsID, allele) pairs, e.g., ("rs1234", 'A')
    pub fn encode_panel(&self, snps: &[(&str, char)]) -> Result<Hypervector, HdcError> {
        if snps.is_empty() {
            return Err(HdcError::EmptyInput);
        }

        let mut snp_vectors: Vec<Hypervector> = Vec::new();

        for (rsid, allele) in snps {
            let snp_key = format!("{}:{}", rsid, allele);
            let snp_vec = Hypervector::random(&self.seed, &snp_key);
            snp_vectors.push(snp_vec);
        }

        let refs: Vec<&Hypervector> = snp_vectors.iter().collect();
        Ok(bundle(&refs))
    }
}

/// Generate all possible k-mers for a given k
pub fn generate_all_kmers(k: u8) -> Vec<String> {
    let k = k as usize;
    let total = 4usize.pow(k as u32);
    let mut kmers = Vec::with_capacity(total);

    for i in 0..total {
        let mut kmer = String::with_capacity(k);
        let mut val = i;
        for _ in 0..k {
            kmer.push(NUCLEOTIDES[val % 4]);
            val /= 4;
        }
        kmers.push(kmer);
    }

    kmers
}

/// Calculate GC content of a sequence
pub fn gc_content(sequence: &str) -> f64 {
    let seq = sequence.to_uppercase();
    let gc_count = seq.chars().filter(|&c| c == 'G' || c == 'C').count();
    gc_count as f64 / seq.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dna_encoder() {
        let seed = Seed::from_string("test");
        let encoder = DnaEncoder::new(seed, 6);

        let result = encoder.encode_sequence("ACGTACGTACGT");
        assert!(result.is_ok());

        let encoded = result.unwrap();
        assert_eq!(encoded.kmer_count, 7); // 12 - 6 + 1 = 7
        assert_eq!(encoded.kmer_length, 6);
    }

    #[test]
    fn test_similar_sequences() {
        let seed = Seed::from_string("test");
        let encoder = DnaEncoder::new(seed, 6);

        let seq1 = "ACGTACGTACGTACGTACGT";
        let seq2 = "ACGTACGTACGTACGTACGT"; // identical
        let seq3 = "TGCATGCATGCATGCATGCA"; // different

        let enc1 = encoder.encode_sequence(seq1).unwrap();
        let enc2 = encoder.encode_sequence(seq2).unwrap();
        let enc3 = encoder.encode_sequence(seq3).unwrap();

        let sim_identical = enc1.vector.hamming_similarity(&enc2.vector);
        let sim_different = enc1.vector.hamming_similarity(&enc3.vector);

        assert!(sim_identical > 0.99);
        assert!(sim_different < sim_identical);
    }

    #[test]
    fn test_hla_encoder() {
        let seed = Seed::from_string("hla-test");
        let encoder = HlaEncoder::new(seed);

        let typing1 = vec!["A*02:01", "A*03:01", "B*07:02", "B*08:01"];
        let typing2 = vec!["A*02:01", "A*03:01", "B*07:02", "B*08:01"]; // identical

        let score = encoder.match_score(&typing1, &typing2).unwrap();
        assert!(score > 0.99);
    }

    #[test]
    fn test_snp_encoder() {
        let seed = Seed::from_string("snp-test");
        let encoder = SnpEncoder::new(seed);

        let panel = vec![
            ("rs1234", 'A'),
            ("rs5678", 'G'),
            ("rs9012", 'C'),
        ];

        let result = encoder.encode_panel(&panel);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gc_content() {
        assert!((gc_content("GGCC") - 1.0).abs() < 0.001);
        assert!((gc_content("AATT") - 0.0).abs() < 0.001);
        assert!((gc_content("ACGT") - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_generate_kmers() {
        let kmers = generate_all_kmers(2);
        assert_eq!(kmers.len(), 16); // 4^2 = 16
    }
}
