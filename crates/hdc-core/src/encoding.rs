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

    /// Encode a DNA sequence using learned (pre-trained) k-mer embeddings
    ///
    /// This uses embeddings trained via contrastive learning and fine-tuning
    /// for improved classification accuracy compared to random embeddings.
    ///
    /// # Arguments
    ///
    /// * `sequence` - The DNA sequence to encode
    /// * `codebook` - A learned codebook loaded from trained embeddings
    ///
    /// # Returns
    ///
    /// Returns an `EncodedSequence` with the bundled hypervector, or an error
    /// if the sequence is invalid or k-mers are not found in the codebook.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hdc_core::encoding::{DnaEncoder, LearnedKmerCodebook};
    /// use hdc_core::Seed;
    ///
    /// let codebook = LearnedKmerCodebook::load("models/learned_6mers.json")?;
    /// let encoder = DnaEncoder::new(Seed::from_string("dna"), 6);
    /// let encoded = encoder.encode_with_learned_codebook("ACGTACGTACGT", &codebook)?;
    /// # Ok::<(), hdc_core::HdcError>(())
    /// ```
    #[cfg(feature = "learned")]
    pub fn encode_with_learned_codebook(
        &self,
        sequence: &str,
        codebook: &LearnedKmerCodebook,
    ) -> Result<EncodedSequence, HdcError> {
        let seq = sequence.to_uppercase();
        let k = self.kmer_length as usize;

        if seq.len() < k {
            return Err(HdcError::SequenceTooShort {
                length: seq.len(),
                kmer_length: self.kmer_length,
            });
        }

        // Validate k-mer length matches codebook
        if self.kmer_length != codebook.kmer_length() {
            return Err(HdcError::InvalidConfig {
                parameter: "kmer_length",
                value: format!("encoder: {}, codebook: {}", self.kmer_length, codebook.kmer_length()),
                reason: "k-mer length must match between encoder and learned codebook".to_string(),
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
        let mut missing_kmers = Vec::new();

        for i in 0..=(seq.len() - k) {
            let kmer = &seq[i..i + k];

            // Look up learned embedding
            if let Some(item_vec) = codebook.get(kmer) {
                // Permute by position (positional encoding)
                let position_vec = item_vec.permute(i);
                kmer_vectors.push(position_vec);
                kmer_count += 1;
            } else {
                missing_kmers.push(kmer.to_string());
            }
        }

        // Warn if some k-mers were missing (but continue if we have any)
        if !missing_kmers.is_empty() && kmer_vectors.is_empty() {
            return Err(HdcError::EmptyInput);
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

/// Learned k-mer codebook with pre-trained embeddings
///
/// This codebook loads embeddings that were trained using contrastive learning
/// and fine-tuning (hybrid HDC approach) for improved classification accuracy.
///
/// The learned embeddings capture semantic relationships between k-mers that
/// random embeddings cannot, resulting in 10-50% accuracy improvements on
/// classification tasks.
///
/// # File Format
///
/// The codebook is stored as JSON with the following structure:
/// ```json
/// {
///   "kmer_length": 6,
///   "dimension": 1000,
///   "embeddings": {
///     "ACGTAC": [0.5, -0.3, ...],
///     "CGTACG": [-0.2, 0.8, ...]
///   }
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// use hdc_core::encoding::{DnaEncoder, LearnedKmerCodebook};
/// use hdc_core::Seed;
///
/// let codebook = LearnedKmerCodebook::load("models/learned_kmers.json")?;
/// let seed = Seed::from_string("dna");
/// let encoder = DnaEncoder::new(seed, 6);
/// let encoded = encoder.encode_with_learned_codebook("ACGTACGT", &codebook)?;
/// ```
#[cfg(feature = "learned")]
pub struct LearnedKmerCodebook {
    vectors: std::collections::HashMap<String, Hypervector>,
    kmer_length: u8,
    source_dimension: usize,
}

/// Serializable format for learned embeddings
#[cfg(feature = "learned")]
#[derive(serde::Deserialize, serde::Serialize)]
struct LearnedCodebookFile {
    kmer_length: u8,
    dimension: usize,
    embeddings: std::collections::HashMap<String, Vec<f32>>,
}

#[cfg(feature = "learned")]
impl LearnedKmerCodebook {
    /// Load learned embeddings from a JSON file
    ///
    /// The file should contain float vectors that will be binarized
    /// using threshold 0.0 (bipolar to binary conversion).
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self, HdcError> {
        let file = std::fs::File::open(path.as_ref()).map_err(|e| {
            HdcError::IoError {
                operation: "open",
                message: format!("Failed to open codebook file: {}", e),
            }
        })?;

        let reader = std::io::BufReader::new(file);
        let data: LearnedCodebookFile = serde_json::from_reader(reader).map_err(|e| {
            HdcError::IoError {
                operation: "parse",
                message: format!("Failed to parse codebook JSON: {}", e),
            }
        })?;

        let mut vectors = std::collections::HashMap::new();

        for (kmer, float_vec) in data.embeddings {
            let hv = Self::float_to_hypervector(&float_vec)?;
            vectors.insert(kmer, hv);
        }

        Ok(LearnedKmerCodebook {
            vectors,
            kmer_length: data.kmer_length,
            source_dimension: data.dimension,
        })
    }

    /// Convert a float vector to a binary hypervector
    ///
    /// Uses threshold at 0.0 for bipolar-to-binary conversion:
    /// - Values >= 0.0 become 1
    /// - Values < 0.0 become 0
    fn float_to_hypervector(floats: &[f32]) -> Result<Hypervector, HdcError> {
        use crate::HYPERVECTOR_DIM;

        // Handle dimension mismatch by padding or truncating
        let target_dim = HYPERVECTOR_DIM;
        let source_dim = floats.len();

        let mut bytes = vec![0u8; crate::HYPERVECTOR_BYTES];

        for i in 0..target_dim {
            // If source is smaller, cycle through it
            let value = if source_dim > 0 {
                floats[i % source_dim]
            } else {
                0.0
            };

            // Bipolar to binary: >= 0 becomes 1
            if value >= 0.0 {
                let byte_idx = i / 8;
                let bit_idx = i % 8;
                bytes[byte_idx] |= 1 << bit_idx;
            }
        }

        Hypervector::from_bytes(bytes)
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

    /// Get the k-mer length
    pub fn kmer_length(&self) -> u8 {
        self.kmer_length
    }

    /// Get the source dimension (before binarization)
    pub fn source_dimension(&self) -> usize {
        self.source_dimension
    }

    /// Create a codebook from pre-computed float vectors
    ///
    /// This is useful for directly passing embeddings from Python training
    /// without going through a file.
    pub fn from_embeddings(
        embeddings: std::collections::HashMap<String, Vec<f32>>,
        kmer_length: u8,
    ) -> Result<Self, HdcError> {
        let source_dimension = embeddings.values().next().map(|v| v.len()).unwrap_or(0);
        let mut vectors = std::collections::HashMap::new();

        for (kmer, float_vec) in embeddings {
            let hv = Self::float_to_hypervector(&float_vec)?;
            vectors.insert(kmer, hv);
        }

        Ok(LearnedKmerCodebook {
            vectors,
            kmer_length,
            source_dimension,
        })
    }

    /// Save the codebook to a JSON file
    ///
    /// Note: This saves the binarized vectors, not the original floats.
    /// For lossless round-trip, save the original float embeddings.
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), HdcError> {
        // Convert binary back to float (lossy - all values will be 0.0 or 1.0)
        let embeddings: std::collections::HashMap<String, Vec<f32>> = self.vectors
            .iter()
            .map(|(kmer, hv)| {
                let floats: Vec<f32> = (0..crate::HYPERVECTOR_DIM)
                    .map(|i| if hv.get_bit(i) { 1.0 } else { -1.0 })
                    .collect();
                (kmer.clone(), floats)
            })
            .collect();

        let data = LearnedCodebookFile {
            kmer_length: self.kmer_length,
            dimension: crate::HYPERVECTOR_DIM,
            embeddings,
        };

        let file = std::fs::File::create(path.as_ref()).map_err(|e| {
            HdcError::IoError {
                operation: "create",
                message: format!("Failed to create codebook file: {}", e),
            }
        })?;

        serde_json::to_writer_pretty(file, &data).map_err(|e| {
            HdcError::IoError {
                operation: "write",
                message: format!("Failed to write codebook JSON: {}", e),
            }
        })?;

        Ok(())
    }
}

/// MLP Classifier for learned HDC
///
/// A simple two-layer neural network that classifies encoded sequences.
/// Trained in Python and loaded for inference in Rust.
///
/// Architecture:
/// - Input: encoded hypervector (16,384 bits → float via popcount)
/// - Hidden: ReLU activation
/// - Output: softmax for classification
///
/// # Example
///
/// ```ignore
/// let classifier = LearnedClassifier::load("models/learned_6mers_mlp.json")?;
/// let codebook = LearnedKmerCodebook::load("models/learned_6mers.json")?;
/// let encoder = DnaEncoder::new(seed, 6);
///
/// let encoded = encoder.encode_with_learned_codebook("ACGTACGT", &codebook)?;
/// let prediction = classifier.predict(&encoded.vector);
/// println!("Class: {}, Confidence: {:.2}%", prediction.class, prediction.confidence * 100.0);
/// ```
#[cfg(feature = "learned")]
pub struct LearnedClassifier {
    /// First layer weights: (input_dim, hidden_dim)
    w1: Vec<Vec<f32>>,
    /// First layer bias: (hidden_dim,)
    b1: Vec<f32>,
    /// Second layer weights: (hidden_dim, output_dim)
    w2: Vec<Vec<f32>>,
    /// Second layer bias: (output_dim,)
    b2: Vec<f32>,
    /// Input dimension (should match embedding dimension)
    input_dim: usize,
    /// Hidden layer dimension
    hidden_dim: usize,
    /// Output dimension (number of classes)
    output_dim: usize,
}

/// Serializable format for MLP classifier
#[cfg(feature = "learned")]
#[derive(serde::Deserialize, serde::Serialize)]
struct LearnedClassifierFile {
    input_dim: usize,
    hidden_dim: usize,
    output_dim: usize,
    #[serde(rename = "W1")]
    w1: Vec<Vec<f32>>,
    b1: Vec<f32>,
    #[serde(rename = "W2")]
    w2: Vec<Vec<f32>>,
    b2: Vec<f32>,
}

/// Classification result
#[cfg(feature = "learned")]
#[derive(Clone, Debug)]
pub struct ClassificationResult {
    /// Predicted class (0-indexed)
    pub class: usize,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Probabilities for each class
    pub probabilities: Vec<f64>,
}

#[cfg(feature = "learned")]
impl LearnedClassifier {
    /// Load classifier from a JSON file
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self, HdcError> {
        let file = std::fs::File::open(path.as_ref()).map_err(|e| {
            HdcError::IoError {
                operation: "open",
                message: format!("Failed to open classifier file: {}", e),
            }
        })?;

        let reader = std::io::BufReader::new(file);
        let data: LearnedClassifierFile = serde_json::from_reader(reader).map_err(|e| {
            HdcError::IoError {
                operation: "parse",
                message: format!("Failed to parse classifier JSON: {}", e),
            }
        })?;

        Ok(LearnedClassifier {
            w1: data.w1,
            b1: data.b1,
            w2: data.w2,
            b2: data.b2,
            input_dim: data.input_dim,
            hidden_dim: data.hidden_dim,
            output_dim: data.output_dim,
        })
    }

    /// Classify an encoded sequence
    ///
    /// The hypervector is converted to a float vector by counting set bits
    /// in each chunk, normalized to [-1, 1] range.
    pub fn predict(&self, vector: &Hypervector) -> ClassificationResult {
        // Convert binary hypervector to float encoding
        let input = self.hypervector_to_float(vector);

        // Forward pass through MLP
        let hidden = self.forward_layer(&input, &self.w1, &self.b1, true);
        let logits = self.forward_layer(&hidden, &self.w2, &self.b2, false);

        // Softmax
        let probabilities = self.softmax(&logits);

        // Find argmax
        let (class, confidence) = probabilities
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, &p)| (i, p))
            .unwrap_or((0, 0.0));

        ClassificationResult {
            class,
            confidence,
            probabilities,
        }
    }

    /// Batch prediction for multiple sequences
    pub fn predict_batch(&self, vectors: &[&Hypervector]) -> Vec<ClassificationResult> {
        vectors.iter().map(|v| self.predict(v)).collect()
    }

    /// Convert hypervector to float representation
    fn hypervector_to_float(&self, vector: &Hypervector) -> Vec<f32> {
        // We need to map the binary vector to the input dimension
        // Strategy: chunk the hypervector and compute normalized popcount
        let bytes = vector.as_bytes();
        let bits_per_chunk = crate::HYPERVECTOR_DIM / self.input_dim;

        let mut result = Vec::with_capacity(self.input_dim);

        for chunk_idx in 0..self.input_dim {
            let start_bit = chunk_idx * bits_per_chunk;
            let mut popcount = 0;

            for bit_offset in 0..bits_per_chunk {
                let bit_idx = start_bit + bit_offset;
                if bit_idx < crate::HYPERVECTOR_DIM {
                    let byte_idx = bit_idx / 8;
                    let bit_in_byte = bit_idx % 8;
                    if bytes[byte_idx] & (1 << bit_in_byte) != 0 {
                        popcount += 1;
                    }
                }
            }

            // Normalize to [-1, 1] range
            let normalized = (2.0 * popcount as f32 / bits_per_chunk as f32) - 1.0;
            result.push(normalized);
        }

        result
    }

    /// Forward pass through a single layer
    fn forward_layer(
        &self,
        input: &[f32],
        weights: &[Vec<f32>],
        bias: &[f32],
        apply_relu: bool,
    ) -> Vec<f32> {
        let out_dim = bias.len();
        let mut output = vec![0.0f32; out_dim];

        // Matrix multiply: output = input @ weights + bias
        for (j, out_val) in output.iter_mut().enumerate() {
            let mut sum = bias[j];
            for (i, &inp) in input.iter().enumerate() {
                if i < weights.len() && j < weights[i].len() {
                    sum += inp * weights[i][j];
                }
            }

            // Apply ReLU if requested
            *out_val = if apply_relu { sum.max(0.0) } else { sum };
        }

        output
    }

    /// Softmax activation
    fn softmax(&self, logits: &[f32]) -> Vec<f64> {
        let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_logits: Vec<f64> = logits
            .iter()
            .map(|&x| ((x - max_logit) as f64).exp())
            .collect();
        let sum: f64 = exp_logits.iter().sum();

        exp_logits.iter().map(|&x| x / sum).collect()
    }

    /// Get input dimension
    pub fn input_dim(&self) -> usize {
        self.input_dim
    }

    /// Get number of output classes
    pub fn num_classes(&self) -> usize {
        self.output_dim
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

/// Multi-scale DNA sequence encoder
///
/// Encodes sequences using multiple k-mer lengths simultaneously to capture
/// both local and global patterns. Default scales are k=4 (local), k=6 (medium),
/// and k=8 (global).
///
/// The multi-scale encoding:
/// - k=4: Captures 256 possible 4-mers (local nucleotide patterns)
/// - k=6: Captures 4096 possible 6-mers (medium-range motifs)
/// - k=8: Captures 65536 possible 8-mers (longer motifs like TATA-box)
///
/// Encodings are combined by bundling (majority vote) the per-scale vectors.
///
/// # Example
///
/// ```
/// use hdc_core::encoding::MultiScaleEncoder;
/// use hdc_core::Seed;
///
/// let seed = Seed::from_string("multi-scale");
/// let encoder = MultiScaleEncoder::new(seed);
///
/// let encoded = encoder.encode_sequence("ACGTACGTACGTACGTACGT").unwrap();
/// println!("Multi-scale k-mers: {:?}", encoded.kmer_counts);
/// ```
pub struct MultiScaleEncoder {
    seed: Seed,
    /// K-mer lengths to use (default: [4, 6, 8])
    scales: Vec<u8>,
    /// Per-scale encoders
    encoders: Vec<DnaEncoder>,
}

/// Result of multi-scale encoding
#[derive(Clone, Debug)]
pub struct MultiScaleEncodedSequence {
    /// The combined hypervector
    pub vector: Hypervector,
    /// K-mer counts per scale
    pub kmer_counts: Vec<(u8, u32)>,
    /// Original sequence length
    pub sequence_length: usize,
    /// Per-scale vectors (for analysis)
    pub scale_vectors: Vec<Hypervector>,
}

impl MultiScaleEncoder {
    /// Create a new multi-scale encoder with default scales (4, 6, 8)
    pub fn new(seed: Seed) -> Self {
        Self::with_scales(seed, vec![4, 6, 8])
    }

    /// Create a multi-scale encoder with custom scales
    pub fn with_scales(seed: Seed, scales: Vec<u8>) -> Self {
        let encoders = scales
            .iter()
            .map(|&k| DnaEncoder::new(seed.clone(), k))
            .collect();

        MultiScaleEncoder {
            seed,
            scales,
            encoders,
        }
    }

    /// Encode a sequence using all scales
    pub fn encode_sequence(&self, sequence: &str) -> Result<MultiScaleEncodedSequence, HdcError> {
        let seq_len = sequence.len();
        let mut scale_vectors = Vec::with_capacity(self.scales.len());
        let mut kmer_counts = Vec::with_capacity(self.scales.len());

        for (encoder, &k) in self.encoders.iter().zip(self.scales.iter()) {
            // Skip scales that are too long for the sequence
            if seq_len < k as usize {
                continue;
            }

            match encoder.encode_sequence(sequence) {
                Ok(encoded) => {
                    scale_vectors.push(encoded.vector);
                    kmer_counts.push((k, encoded.kmer_count));
                }
                Err(HdcError::SequenceTooShort { .. }) => {
                    // Sequence too short for this scale, skip it
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        if scale_vectors.is_empty() {
            return Err(HdcError::SequenceTooShort {
                length: seq_len,
                kmer_length: *self.scales.iter().min().unwrap_or(&4),
            });
        }

        // Combine scale vectors by bundling (majority vote)
        let refs: Vec<&Hypervector> = scale_vectors.iter().collect();
        let combined = bundle(&refs);

        Ok(MultiScaleEncodedSequence {
            vector: combined,
            kmer_counts,
            sequence_length: seq_len,
            scale_vectors,
        })
    }

    /// Encode multiple sequences
    pub fn encode_batch(&self, sequences: &[&str]) -> Vec<Result<MultiScaleEncodedSequence, HdcError>> {
        sequences.iter().map(|seq| self.encode_sequence(seq)).collect()
    }

    /// Encode in parallel (requires "parallel" feature)
    #[cfg(feature = "parallel")]
    pub fn encode_batch_parallel(&self, sequences: &[&str]) -> Vec<Result<MultiScaleEncodedSequence, HdcError>> {
        use rayon::prelude::*;
        sequences.par_iter().map(|seq| self.encode_sequence(seq)).collect()
    }

    /// Get the scales used
    pub fn scales(&self) -> &[u8] {
        &self.scales
    }
}

impl MultiScaleEncodedSequence {
    /// Get similarity between two multi-scale encodings
    pub fn similarity(&self, other: &Self) -> f64 {
        self.vector.hamming_similarity(&other.vector)
    }

    /// Get per-scale similarities (for debugging/analysis)
    pub fn per_scale_similarity(&self, other: &Self) -> Vec<(u8, f64)> {
        self.kmer_counts
            .iter()
            .zip(self.scale_vectors.iter())
            .zip(other.scale_vectors.iter())
            .map(|(((k, _), v1), v2)| (*k, v1.hamming_similarity(v2)))
            .collect()
    }
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
            return Err(HdcError::InvalidConfig {
                parameter: "hla_alleles",
                value: format!("{} alleles", hla_types.len()),
                reason: "expected exactly 10 HLA alleles (2 per locus for A, B, C, DRB1, DQB1)".to_string()
            });
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
            return Err(HdcError::InvalidConfig {
                parameter: "hla_alleles",
                value: format!("{} alleles", hla_types.len()),
                reason: "expected exactly 10 HLA alleles (2 per locus for A, B, C, DRB1, DQB1)".to_string()
            });
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

/// Star allele encoder for pharmacogenomics
///
/// Encodes pharmacogene star alleles (*1, *2, *4, etc.) with function scores
/// for drug metabolism prediction. Supports CYP2D6, CYP2C19, CYP2C9, etc.
///
/// # Example
///
/// ```
/// use hdc_core::{StarAlleleEncoder, MetabolizerPhenotype, Seed};
///
/// let encoder = StarAlleleEncoder::new(Seed::from_string("pharmaco"));
///
/// // Patient with CYP2D6 *1/*4 (intermediate metabolizer)
/// let diplotype = encoder.encode_diplotype("CYP2D6", "*1", "*4").unwrap();
///
/// // Check metabolizer status
/// assert_eq!(diplotype.activity_score, 1.0); // *1 (1.0) + *4 (0.0)
/// assert_eq!(diplotype.phenotype, MetabolizerPhenotype::Intermediate);
/// ```
pub struct StarAlleleEncoder {
    seed: Seed,
    /// Activity score reference: gene -> allele -> score
    activity_scores: std::collections::HashMap<String, std::collections::HashMap<String, f64>>,
}

impl StarAlleleEncoder {
    /// Create a new star allele encoder with standard activity scores
    pub fn new(seed: Seed) -> Self {
        let mut encoder = StarAlleleEncoder {
            seed,
            activity_scores: std::collections::HashMap::new(),
        };
        encoder.load_default_activity_scores();
        encoder
    }

    /// Load standard activity scores from CPIC guidelines
    /// Reference: https://cpicpgx.org/guidelines/
    fn load_default_activity_scores(&mut self) {
        // CYP2D6 activity scores (CPIC 2019 Guideline)
        // Reference: https://cpicpgx.org/guidelines/guideline-for-codeine-and-cyp2d6/
        let mut cyp2d6 = std::collections::HashMap::new();
        // Normal function alleles (activity value = 1.0)
        for allele in ["*1", "*2", "*27", "*33", "*35", "*39", "*45", "*46"] {
            cyp2d6.insert(allele.to_string(), 1.0);
        }
        // Decreased function alleles (activity value = 0.5)
        for allele in ["*9", "*17", "*29", "*41", "*43", "*49"] {
            cyp2d6.insert(allele.to_string(), 0.5);
        }
        // Decreased function alleles (activity value = 0.25) - very reduced
        for allele in ["*10", "*14", "*21", "*44"] {
            cyp2d6.insert(allele.to_string(), 0.25);
        }
        // No function alleles (activity value = 0.0)
        for allele in [
            "*3", "*4", "*5", "*6", "*7", "*8", "*11", "*12", "*13", "*15", "*16",
            "*18", "*19", "*20", "*31", "*36", "*38", "*40", "*42", "*47", "*51",
            "*56", "*57", "*62", "*68", "*69", "*92", "*100",
        ] {
            cyp2d6.insert(allele.to_string(), 0.0);
        }
        // Gene duplication increases function (represented as >2.0 when detected)
        cyp2d6.insert("*1xN".to_string(), 2.0);  // Gene duplication
        cyp2d6.insert("*2xN".to_string(), 2.0);  // Gene duplication
        self.activity_scores.insert("CYP2D6".to_string(), cyp2d6);

        // CYP2C19 activity scores (CPIC 2022 Guideline)
        // Reference: https://cpicpgx.org/guidelines/guideline-for-clopidogrel-and-cyp2c19/
        let mut cyp2c19 = std::collections::HashMap::new();
        // Normal function (activity value = 1.0)
        for allele in ["*1", "*27", "*28"] {
            cyp2c19.insert(allele.to_string(), 1.0);
        }
        // Increased function (activity value = 1.5) - rapid metabolizers
        cyp2c19.insert("*17".to_string(), 1.5);
        // No function (activity value = 0.0)
        for allele in [
            "*2", "*3", "*4", "*5", "*6", "*7", "*8", "*9", "*10", "*22", "*24", "*26",
        ] {
            cyp2c19.insert(allele.to_string(), 0.0);
        }
        self.activity_scores.insert("CYP2C19".to_string(), cyp2c19);

        // CYP2C9 activity scores (CPIC 2020 Guideline for Warfarin)
        // Reference: https://cpicpgx.org/guidelines/guideline-for-warfarin-and-cyp2c9-and-vkorc1/
        let mut cyp2c9 = std::collections::HashMap::new();
        // Normal function
        cyp2c9.insert("*1".to_string(), 1.0);
        // Decreased function (0.5)
        for allele in ["*2", "*8", "*11"] {
            cyp2c9.insert(allele.to_string(), 0.5);
        }
        // No/minimal function (0.0)
        for allele in ["*3", "*5", "*6", "*13"] {
            cyp2c9.insert(allele.to_string(), 0.0);
        }
        self.activity_scores.insert("CYP2C9".to_string(), cyp2c9);

        // CYP3A5 activity scores (CPIC 2022 Tacrolimus Guideline)
        // Reference: https://cpicpgx.org/guidelines/guideline-for-tacrolimus-and-cyp3a5/
        let mut cyp3a5 = std::collections::HashMap::new();
        cyp3a5.insert("*1".to_string(), 1.0);   // Normal function (expresser)
        cyp3a5.insert("*3".to_string(), 0.0);   // No function (non-expresser)
        cyp3a5.insert("*6".to_string(), 0.0);   // No function
        cyp3a5.insert("*7".to_string(), 0.0);   // No function
        self.activity_scores.insert("CYP3A5".to_string(), cyp3a5);

        // TPMT activity scores (CPIC 2018 Thiopurine Guideline)
        // Reference: https://cpicpgx.org/guidelines/guideline-for-thiopurines-and-tpmt/
        let mut tpmt = std::collections::HashMap::new();
        tpmt.insert("*1".to_string(), 1.0);   // Normal function
        // No function alleles
        for allele in ["*2", "*3A", "*3B", "*3C", "*4", "*5", "*6", "*7", "*8", "*9", "*10",
                       "*11", "*12", "*13", "*14", "*15", "*16", "*17", "*18", "*19", "*20",
                       "*21", "*22", "*23", "*24", "*25", "*26", "*27", "*28"] {
            tpmt.insert(allele.to_string(), 0.0);
        }
        self.activity_scores.insert("TPMT".to_string(), tpmt);

        // NUDT15 activity scores (CPIC 2019 Thiopurine Guideline)
        // Important for Asian populations
        let mut nudt15 = std::collections::HashMap::new();
        nudt15.insert("*1".to_string(), 1.0);   // Normal function
        nudt15.insert("*2".to_string(), 0.0);   // No function
        nudt15.insert("*3".to_string(), 0.0);   // No function
        nudt15.insert("*4".to_string(), 0.0);   // No function
        nudt15.insert("*5".to_string(), 0.0);   // No function
        nudt15.insert("*6".to_string(), 0.0);   // No function
        self.activity_scores.insert("NUDT15".to_string(), nudt15);

        // DPYD activity scores (CPIC 2017 Fluoropyrimidine Guideline)
        // Reference: https://cpicpgx.org/guidelines/guideline-for-fluoropyrimidines-and-dpyd/
        let mut dpyd = std::collections::HashMap::new();
        dpyd.insert("*1".to_string(), 1.0);          // Normal function
        dpyd.insert("*2A".to_string(), 0.0);         // No function (c.1905+1G>A, IVS14+1G>A)
        dpyd.insert("*13".to_string(), 0.0);         // No function (c.1679T>G)
        dpyd.insert("c.2846A>T".to_string(), 0.5);   // Decreased function (D949V)
        dpyd.insert("c.1129-5923C>G".to_string(), 0.5); // Decreased function (HapB3)
        dpyd.insert("c.1236G>A".to_string(), 0.5);   // Decreased function
        self.activity_scores.insert("DPYD".to_string(), dpyd);

        // SLCO1B1 activity scores (CPIC 2014 Simvastatin Guideline)
        // Reference: https://cpicpgx.org/guidelines/guideline-for-simvastatin-and-slco1b1/
        let mut slco1b1 = std::collections::HashMap::new();
        slco1b1.insert("*1A".to_string(), 1.0);  // Normal function
        slco1b1.insert("*1B".to_string(), 1.0);  // Normal function
        // Decreased function alleles
        for allele in ["*5", "*15", "*17", "*21", "*31"] {
            slco1b1.insert(allele.to_string(), 0.5);
        }
        // Poor function
        slco1b1.insert("*45".to_string(), 0.0);
        self.activity_scores.insert("SLCO1B1".to_string(), slco1b1);

        // UGT1A1 activity scores (CPIC Irinotecan Guideline)
        let mut ugt1a1 = std::collections::HashMap::new();
        ugt1a1.insert("*1".to_string(), 1.0);    // Normal function
        ugt1a1.insert("*6".to_string(), 0.5);    // Decreased function
        ugt1a1.insert("*28".to_string(), 0.5);   // Decreased function (TA repeat)
        ugt1a1.insert("*37".to_string(), 0.0);   // Poor function
        self.activity_scores.insert("UGT1A1".to_string(), ugt1a1);

        // VKORC1 (CPIC Warfarin Guideline) - uses haplotype groups
        let mut vkorc1 = std::collections::HashMap::new();
        vkorc1.insert("A".to_string(), 0.5);     // Low dose required
        vkorc1.insert("B".to_string(), 1.0);     // Normal dose
        vkorc1.insert("-1639G>A".to_string(), 0.5); // rs9923231 - low dose
        self.activity_scores.insert("VKORC1".to_string(), vkorc1);

        // CYP2B6 (CPIC Efavirenz Guideline)
        let mut cyp2b6 = std::collections::HashMap::new();
        cyp2b6.insert("*1".to_string(), 1.0);    // Normal function
        cyp2b6.insert("*6".to_string(), 0.5);    // Decreased function
        cyp2b6.insert("*18".to_string(), 0.0);   // No function
        self.activity_scores.insert("CYP2B6".to_string(), cyp2b6);
    }

    /// Add or update activity score for an allele
    pub fn set_activity_score(&mut self, gene: &str, allele: &str, score: f64) {
        self.activity_scores
            .entry(gene.to_string())
            .or_insert_with(std::collections::HashMap::new)
            .insert(allele.to_string(), score);
    }

    /// Get activity score for an allele (returns 1.0 for unknown alleles)
    pub fn get_activity_score(&self, gene: &str, allele: &str) -> f64 {
        self.activity_scores
            .get(gene)
            .and_then(|alleles| alleles.get(allele))
            .copied()
            .unwrap_or(1.0) // Unknown alleles assumed normal function
    }

    /// Encode a single star allele as a hypervector
    pub fn encode_allele(&self, gene: &str, allele: &str) -> Hypervector {
        let key = format!("STAR:{}:{}", gene, allele);
        Hypervector::random(&self.seed, &key)
    }

    /// Encode a diplotype (two alleles for a gene)
    pub fn encode_diplotype(
        &self,
        gene: &str,
        allele1: &str,
        allele2: &str,
    ) -> Result<EncodedDiplotype, HdcError> {
        let vec1 = self.encode_allele(gene, allele1);
        let vec2 = self.encode_allele(gene, allele2);

        // Bundle the two alleles
        let vector = bundle(&[&vec1, &vec2]);

        // Calculate activity score
        let score1 = self.get_activity_score(gene, allele1);
        let score2 = self.get_activity_score(gene, allele2);
        let activity_score = score1 + score2;

        // Determine phenotype from activity score
        let phenotype = MetabolizerPhenotype::from_activity_score(gene, activity_score);

        Ok(EncodedDiplotype {
            gene: gene.to_string(),
            allele1: allele1.to_string(),
            allele2: allele2.to_string(),
            vector,
            activity_score,
            phenotype,
        })
    }

    /// Encode a full pharmacogenomic profile (multiple genes)
    pub fn encode_profile(
        &self,
        diplotypes: &[(&str, &str, &str)], // (gene, allele1, allele2)
    ) -> Result<EncodedPgxProfile, HdcError> {
        if diplotypes.is_empty() {
            return Err(HdcError::EmptyInput);
        }

        let mut encoded_diplotypes = Vec::with_capacity(diplotypes.len());
        let mut all_vectors: Vec<Hypervector> = Vec::with_capacity(diplotypes.len());

        for (gene, allele1, allele2) in diplotypes {
            let diplotype = self.encode_diplotype(gene, allele1, allele2)?;
            all_vectors.push(diplotype.vector.clone());
            encoded_diplotypes.push(diplotype);
        }

        // Create composite profile vector
        let refs: Vec<&Hypervector> = all_vectors.iter().collect();
        let profile_vector = bundle(&refs);

        Ok(EncodedPgxProfile {
            diplotypes: encoded_diplotypes,
            profile_vector,
        })
    }

    /// Calculate similarity between two pharmacogenomic profiles
    pub fn profile_similarity(
        &self,
        profile1: &EncodedPgxProfile,
        profile2: &EncodedPgxProfile,
    ) -> f64 {
        profile1.profile_vector.normalized_cosine_similarity(&profile2.profile_vector)
    }

    /// Calculate per-gene similarity between profiles
    pub fn per_gene_similarity(
        &self,
        profile1: &EncodedPgxProfile,
        profile2: &EncodedPgxProfile,
    ) -> std::collections::HashMap<String, f64> {
        let mut similarities = std::collections::HashMap::new();

        for d1 in &profile1.diplotypes {
            if let Some(d2) = profile2.diplotypes.iter().find(|d| d.gene == d1.gene) {
                let sim = d1.vector.normalized_cosine_similarity(&d2.vector);
                similarities.insert(d1.gene.clone(), sim);
            }
        }

        similarities
    }

    /// Get list of all drugs with CPIC guidelines
    pub fn get_supported_drugs(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            // CYP2D6 substrates
            ("codeine", "CYP2D6"),
            ("tramadol", "CYP2D6"),
            ("oxycodone", "CYP2D6"),
            ("hydrocodone", "CYP2D6"),
            ("tamoxifen", "CYP2D6"),
            ("ondansetron", "CYP2D6"),
            ("tropisetron", "CYP2D6"),
            ("paroxetine", "CYP2D6"),
            ("fluvoxamine", "CYP2D6"),
            ("atomoxetine", "CYP2D6"),
            ("nortriptyline", "CYP2D6"),
            ("amitriptyline", "CYP2D6"),
            ("clomipramine", "CYP2D6"),
            ("desipramine", "CYP2D6"),
            ("doxepin", "CYP2D6"),
            ("imipramine", "CYP2D6"),
            ("trimipramine", "CYP2D6"),
            // CYP2C19 substrates
            ("clopidogrel", "CYP2C19"),
            ("omeprazole", "CYP2C19"),
            ("lansoprazole", "CYP2C19"),
            ("pantoprazole", "CYP2C19"),
            ("esomeprazole", "CYP2C19"),
            ("citalopram", "CYP2C19"),
            ("escitalopram", "CYP2C19"),
            ("sertraline", "CYP2C19"),
            ("voriconazole", "CYP2C19"),
            // CYP2C9 substrates
            ("warfarin", "CYP2C9"),
            ("phenytoin", "CYP2C9"),
            ("celecoxib", "CYP2C9"),
            ("flurbiprofen", "CYP2C9"),
            ("siponimod", "CYP2C9"),
            // CYP3A5 substrates
            ("tacrolimus", "CYP3A5"),
            // TPMT/NUDT15 substrates
            ("azathioprine", "TPMT"),
            ("mercaptopurine", "TPMT"),
            ("thioguanine", "TPMT"),
            // DPYD substrates
            ("fluorouracil", "DPYD"),
            ("5-fluorouracil", "DPYD"),
            ("5-fu", "DPYD"),
            ("capecitabine", "DPYD"),
            ("tegafur", "DPYD"),
            // SLCO1B1 substrates
            ("simvastatin", "SLCO1B1"),
            ("atorvastatin", "SLCO1B1"),
            ("rosuvastatin", "SLCO1B1"),
            ("pravastatin", "SLCO1B1"),
            // UGT1A1 substrates
            ("irinotecan", "UGT1A1"),
            ("atazanavir", "UGT1A1"),
            // CYP2B6 substrates
            ("efavirenz", "CYP2B6"),
        ]
    }

    /// Predict drug interaction based on metabolizer phenotypes
    /// Based on CPIC guidelines: https://cpicpgx.org/guidelines/
    pub fn predict_drug_interaction(
        &self,
        profile: &EncodedPgxProfile,
        drug: &str,
    ) -> Option<DrugInteractionPrediction> {
        // Comprehensive CPIC drug-gene associations
        let gene = match drug.to_lowercase().as_str() {
            // CYP2D6 substrates (opioids, antidepressants, tamoxifen)
            "codeine" | "tramadol" | "oxycodone" | "hydrocodone" => "CYP2D6",
            "tamoxifen" => "CYP2D6",
            "ondansetron" | "tropisetron" => "CYP2D6",
            "paroxetine" | "fluvoxamine" => "CYP2D6",
            "atomoxetine" => "CYP2D6",
            "nortriptyline" | "amitriptyline" | "clomipramine" | "desipramine" |
            "doxepin" | "imipramine" | "trimipramine" => "CYP2D6",

            // CYP2C19 substrates (clopidogrel, PPIs, antidepressants, antifungals)
            "clopidogrel" => "CYP2C19",
            "omeprazole" | "lansoprazole" | "pantoprazole" | "esomeprazole" => "CYP2C19",
            "citalopram" | "escitalopram" | "sertraline" => "CYP2C19",
            "voriconazole" => "CYP2C19",

            // CYP2C9 substrates (warfarin, phenytoin, NSAIDs)
            "warfarin" => "CYP2C9",
            "phenytoin" => "CYP2C9",
            "celecoxib" | "flurbiprofen" => "CYP2C9",
            "siponimod" => "CYP2C9",

            // CYP3A5 substrates (tacrolimus)
            "tacrolimus" => "CYP3A5",

            // TPMT substrates (thiopurines)
            "azathioprine" | "mercaptopurine" | "thioguanine" => "TPMT",

            // DPYD substrates (fluoropyrimidines)
            "fluorouracil" | "5-fluorouracil" | "5-fu" | "capecitabine" | "tegafur" => "DPYD",

            // SLCO1B1 substrates (statins)
            "simvastatin" | "atorvastatin" | "rosuvastatin" | "pravastatin" => "SLCO1B1",

            // UGT1A1 substrates
            "irinotecan" | "atazanavir" => "UGT1A1",

            // CYP2B6 substrates
            "efavirenz" => "CYP2B6",

            _ => return None,
        };

        // Find the relevant diplotype
        let diplotype = profile.diplotypes.iter().find(|d| d.gene == gene)?;

        // Get recommendation based on gene and phenotype (CPIC guidelines)
        let recommendation = match gene {
            "CYP2D6" => DrugRecommendation::from_cyp2d6_phenotype(&diplotype.phenotype),
            "CYP2C19" => DrugRecommendation::from_cyp2c19_phenotype(&diplotype.phenotype),
            "CYP2C9" => DrugRecommendation::from_cyp2c9_phenotype(&diplotype.phenotype),
            "CYP3A5" => DrugRecommendation::from_cyp3a5_phenotype(&diplotype.phenotype),
            "TPMT" | "NUDT15" => DrugRecommendation::from_tpmt_phenotype(&diplotype.phenotype),
            "DPYD" => DrugRecommendation::from_dpyd_phenotype(&diplotype.phenotype),
            "SLCO1B1" => DrugRecommendation::from_slco1b1_phenotype(&diplotype.phenotype),
            "UGT1A1" => DrugRecommendation::from_ugt1a1_phenotype(&diplotype.phenotype),
            "CYP2B6" => DrugRecommendation::from_cyp2b6_phenotype(&diplotype.phenotype),
            "VKORC1" => DrugRecommendation::from_vkorc1_phenotype(&diplotype.phenotype),
            _ => DrugRecommendation::InsufficientEvidence,
        };

        Some(DrugInteractionPrediction {
            drug: drug.to_string(),
            gene: gene.to_string(),
            phenotype: diplotype.phenotype.clone(),
            activity_score: diplotype.activity_score,
            recommendation,
        })
    }
}

/// Metabolizer phenotype classification
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetabolizerPhenotype {
    /// Very high enzyme activity (>2.0 activity score)
    Ultrarapid,
    /// Higher than normal activity (1.5-2.0)
    RapidToNormal,
    /// Normal enzyme activity (1.0-1.5)
    Normal,
    /// Intermediate activity (0.5-1.0)
    Intermediate,
    /// Poor/no enzyme activity (<0.5)
    Poor,
    /// Indeterminate (unknown alleles)
    Indeterminate,
}

impl MetabolizerPhenotype {
    /// Determine phenotype from activity score
    pub fn from_activity_score(gene: &str, score: f64) -> Self {
        // CYP2D6 uses standard thresholds
        match gene {
            "CYP2D6" => {
                if score > 2.0 {
                    MetabolizerPhenotype::Ultrarapid
                } else if score >= 1.25 {
                    MetabolizerPhenotype::Normal
                } else if score >= 0.25 {
                    MetabolizerPhenotype::Intermediate
                } else {
                    MetabolizerPhenotype::Poor
                }
            }
            "CYP2C19" => {
                if score > 2.0 {
                    MetabolizerPhenotype::Ultrarapid
                } else if score >= 1.5 {
                    MetabolizerPhenotype::RapidToNormal
                } else if score >= 1.0 {
                    MetabolizerPhenotype::Normal
                } else if score > 0.0 {
                    MetabolizerPhenotype::Intermediate
                } else {
                    MetabolizerPhenotype::Poor
                }
            }
            _ => {
                // Generic thresholds for other genes
                if score >= 1.5 {
                    MetabolizerPhenotype::Normal
                } else if score >= 0.5 {
                    MetabolizerPhenotype::Intermediate
                } else {
                    MetabolizerPhenotype::Poor
                }
            }
        }
    }
}

impl std::fmt::Display for MetabolizerPhenotype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetabolizerPhenotype::Ultrarapid => write!(f, "Ultrarapid Metabolizer"),
            MetabolizerPhenotype::RapidToNormal => write!(f, "Rapid to Normal Metabolizer"),
            MetabolizerPhenotype::Normal => write!(f, "Normal Metabolizer"),
            MetabolizerPhenotype::Intermediate => write!(f, "Intermediate Metabolizer"),
            MetabolizerPhenotype::Poor => write!(f, "Poor Metabolizer"),
            MetabolizerPhenotype::Indeterminate => write!(f, "Indeterminate"),
        }
    }
}

/// Encoded diplotype (two alleles for a gene)
#[derive(Clone, Debug)]
pub struct EncodedDiplotype {
    /// Gene name (e.g., "CYP2D6")
    pub gene: String,
    /// First allele (e.g., "*1")
    pub allele1: String,
    /// Second allele (e.g., "*4")
    pub allele2: String,
    /// Combined hypervector
    pub vector: Hypervector,
    /// Combined activity score
    pub activity_score: f64,
    /// Phenotype classification
    pub phenotype: MetabolizerPhenotype,
}

impl EncodedDiplotype {
    /// Format as standard diplotype notation (e.g., "CYP2D6 *1/*4")
    pub fn to_notation(&self) -> String {
        format!("{} {}/{}", self.gene, self.allele1, self.allele2)
    }
}

/// Encoded pharmacogenomic profile
#[derive(Clone, Debug)]
pub struct EncodedPgxProfile {
    /// Individual diplotypes
    pub diplotypes: Vec<EncodedDiplotype>,
    /// Composite profile vector
    pub profile_vector: Hypervector,
}

impl EncodedPgxProfile {
    /// Get diplotype for a specific gene
    pub fn get_diplotype(&self, gene: &str) -> Option<&EncodedDiplotype> {
        self.diplotypes.iter().find(|d| d.gene == gene)
    }

    /// Get all poor metabolizer genes
    pub fn get_poor_metabolizer_genes(&self) -> Vec<&str> {
        self.diplotypes
            .iter()
            .filter(|d| d.phenotype == MetabolizerPhenotype::Poor)
            .map(|d| d.gene.as_str())
            .collect()
    }

    /// Get summary of all phenotypes
    pub fn summary(&self) -> String {
        self.diplotypes
            .iter()
            .map(|d| format!("{}: {} (AS={})", d.to_notation(), d.phenotype, d.activity_score))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Drug dosing recommendation
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DrugRecommendation {
    /// Use standard dose
    StandardDose,
    /// Consider increased dose or alternative
    ConsiderAlternative,
    /// Use reduced dose
    ReducedDose,
    /// Avoid this drug
    Avoid,
    /// Use with caution
    UseWithCaution,
    /// Insufficient evidence
    InsufficientEvidence,
}

impl DrugRecommendation {
    fn from_cyp2d6_phenotype(phenotype: &MetabolizerPhenotype) -> Self {
        match phenotype {
            MetabolizerPhenotype::Ultrarapid => DrugRecommendation::ConsiderAlternative,
            MetabolizerPhenotype::Normal | MetabolizerPhenotype::RapidToNormal => {
                DrugRecommendation::StandardDose
            }
            MetabolizerPhenotype::Intermediate => DrugRecommendation::UseWithCaution,
            MetabolizerPhenotype::Poor => DrugRecommendation::Avoid,
            MetabolizerPhenotype::Indeterminate => DrugRecommendation::InsufficientEvidence,
        }
    }

    fn from_cyp2c19_phenotype(phenotype: &MetabolizerPhenotype) -> Self {
        match phenotype {
            MetabolizerPhenotype::Ultrarapid | MetabolizerPhenotype::RapidToNormal => {
                DrugRecommendation::ConsiderAlternative
            }
            MetabolizerPhenotype::Normal => DrugRecommendation::StandardDose,
            MetabolizerPhenotype::Intermediate => DrugRecommendation::UseWithCaution,
            MetabolizerPhenotype::Poor => DrugRecommendation::Avoid,
            MetabolizerPhenotype::Indeterminate => DrugRecommendation::InsufficientEvidence,
        }
    }

    fn from_cyp2c9_phenotype(phenotype: &MetabolizerPhenotype) -> Self {
        match phenotype {
            MetabolizerPhenotype::Normal | MetabolizerPhenotype::RapidToNormal => {
                DrugRecommendation::StandardDose
            }
            MetabolizerPhenotype::Intermediate => DrugRecommendation::ReducedDose,
            MetabolizerPhenotype::Poor => DrugRecommendation::Avoid,
            _ => DrugRecommendation::InsufficientEvidence,
        }
    }

    fn from_tpmt_phenotype(phenotype: &MetabolizerPhenotype) -> Self {
        match phenotype {
            MetabolizerPhenotype::Normal => DrugRecommendation::StandardDose,
            MetabolizerPhenotype::Intermediate => DrugRecommendation::ReducedDose,
            MetabolizerPhenotype::Poor => DrugRecommendation::Avoid,
            _ => DrugRecommendation::InsufficientEvidence,
        }
    }

    fn from_dpyd_phenotype(phenotype: &MetabolizerPhenotype) -> Self {
        match phenotype {
            MetabolizerPhenotype::Normal => DrugRecommendation::StandardDose,
            MetabolizerPhenotype::Intermediate => DrugRecommendation::ReducedDose,
            MetabolizerPhenotype::Poor => DrugRecommendation::Avoid,
            _ => DrugRecommendation::InsufficientEvidence,
        }
    }

    fn from_slco1b1_phenotype(phenotype: &MetabolizerPhenotype) -> Self {
        match phenotype {
            MetabolizerPhenotype::Normal => DrugRecommendation::StandardDose,
            MetabolizerPhenotype::Intermediate => DrugRecommendation::ReducedDose,
            MetabolizerPhenotype::Poor => DrugRecommendation::Avoid,
            _ => DrugRecommendation::InsufficientEvidence,
        }
    }

    fn from_cyp3a5_phenotype(phenotype: &MetabolizerPhenotype) -> Self {
        // CYP3A5 for tacrolimus - expressers need higher dose
        match phenotype {
            MetabolizerPhenotype::Normal => DrugRecommendation::ConsiderAlternative, // Higher dose needed
            MetabolizerPhenotype::Intermediate => DrugRecommendation::UseWithCaution,
            MetabolizerPhenotype::Poor => DrugRecommendation::StandardDose, // Non-expressers use standard
            _ => DrugRecommendation::InsufficientEvidence,
        }
    }

    fn from_nudt15_phenotype(phenotype: &MetabolizerPhenotype) -> Self {
        // NUDT15 for thiopurines - similar to TPMT
        match phenotype {
            MetabolizerPhenotype::Normal => DrugRecommendation::StandardDose,
            MetabolizerPhenotype::Intermediate => DrugRecommendation::ReducedDose,
            MetabolizerPhenotype::Poor => DrugRecommendation::Avoid,
            _ => DrugRecommendation::InsufficientEvidence,
        }
    }

    fn from_ugt1a1_phenotype(phenotype: &MetabolizerPhenotype) -> Self {
        // UGT1A1 for irinotecan
        match phenotype {
            MetabolizerPhenotype::Normal => DrugRecommendation::StandardDose,
            MetabolizerPhenotype::Intermediate => DrugRecommendation::ReducedDose,
            MetabolizerPhenotype::Poor => DrugRecommendation::Avoid,
            _ => DrugRecommendation::InsufficientEvidence,
        }
    }

    fn from_cyp2b6_phenotype(phenotype: &MetabolizerPhenotype) -> Self {
        // CYP2B6 for efavirenz
        match phenotype {
            MetabolizerPhenotype::Normal => DrugRecommendation::StandardDose,
            MetabolizerPhenotype::Intermediate => DrugRecommendation::ReducedDose,
            MetabolizerPhenotype::Poor => DrugRecommendation::ReducedDose, // Significantly reduced
            MetabolizerPhenotype::Ultrarapid => DrugRecommendation::ConsiderAlternative,
            _ => DrugRecommendation::InsufficientEvidence,
        }
    }

    fn from_vkorc1_phenotype(phenotype: &MetabolizerPhenotype) -> Self {
        // VKORC1 for warfarin
        match phenotype {
            MetabolizerPhenotype::Normal => DrugRecommendation::StandardDose,
            MetabolizerPhenotype::Intermediate | MetabolizerPhenotype::Poor => {
                DrugRecommendation::ReducedDose
            }
            _ => DrugRecommendation::InsufficientEvidence,
        }
    }
}

impl std::fmt::Display for DrugRecommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DrugRecommendation::StandardDose => write!(f, "Standard dose recommended"),
            DrugRecommendation::ConsiderAlternative => {
                write!(f, "Consider alternative or increased dose")
            }
            DrugRecommendation::ReducedDose => write!(f, "Reduced dose recommended"),
            DrugRecommendation::Avoid => write!(f, "Avoid - use alternative drug"),
            DrugRecommendation::UseWithCaution => write!(f, "Use with caution"),
            DrugRecommendation::InsufficientEvidence => write!(f, "Insufficient evidence"),
        }
    }
}

/// Drug interaction prediction result
#[derive(Clone, Debug)]
pub struct DrugInteractionPrediction {
    /// Drug name
    pub drug: String,
    /// Relevant gene
    pub gene: String,
    /// Patient phenotype for this gene
    pub phenotype: MetabolizerPhenotype,
    /// Activity score
    pub activity_score: f64,
    /// Dosing recommendation
    pub recommendation: DrugRecommendation,
}

// =============================================================================
// Ancestry-Informed Pharmacogenomics
// =============================================================================

/// Biogeographic ancestry groups based on PharmGKB/CPIC classifications
/// Reference: https://www.pharmgkb.org/page/biogeographicalGroups
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ancestry {
    /// African or African American
    African,
    /// American (Indigenous peoples of the Americas)
    American,
    /// Central/South Asian
    CentralSouthAsian,
    /// East Asian
    EastAsian,
    /// European (Caucasian)
    European,
    /// Latino/Hispanic
    Latino,
    /// Near Eastern (Middle Eastern, North African)
    NearEastern,
    /// Oceanian (Pacific Islander, Aboriginal Australian)
    Oceanian,
    /// Mixed or multi-ethnic
    Mixed(Vec<Ancestry>),
    /// Unknown/unspecified ancestry
    Unknown,
}

impl std::fmt::Display for Ancestry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ancestry::African => write!(f, "African"),
            Ancestry::American => write!(f, "American"),
            Ancestry::CentralSouthAsian => write!(f, "Central/South Asian"),
            Ancestry::EastAsian => write!(f, "East Asian"),
            Ancestry::European => write!(f, "European"),
            Ancestry::Latino => write!(f, "Latino"),
            Ancestry::NearEastern => write!(f, "Near Eastern"),
            Ancestry::Oceanian => write!(f, "Oceanian"),
            Ancestry::Mixed(groups) => {
                let names: Vec<_> = groups.iter().map(|g| g.to_string()).collect();
                write!(f, "Mixed ({})", names.join(", "))
            }
            Ancestry::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Population-specific allele frequency data
/// Sources: gnomAD, PharmGKB, CPIC guidelines
#[derive(Clone, Debug)]
pub struct AlleleFrequencies {
    /// Gene name
    pub gene: String,
    /// Allele name
    pub allele: String,
    /// Frequencies by ancestry
    pub frequencies: std::collections::HashMap<Ancestry, f64>,
}

/// Ancestry-informed pharmacogenomics encoder
/// Enhances predictions using population-specific allele frequencies
pub struct AncestryInformedEncoder {
    base_encoder: StarAlleleEncoder,
    /// Population allele frequencies
    frequencies: Vec<AlleleFrequencies>,
}

impl AncestryInformedEncoder {
    /// Create a new ancestry-informed encoder
    pub fn new(seed: Seed) -> Self {
        let base_encoder = StarAlleleEncoder::new(seed);
        let frequencies = Self::load_population_frequencies();

        AncestryInformedEncoder {
            base_encoder,
            frequencies,
        }
    }

    /// Load population-specific allele frequencies from CPIC/PharmGKB data
    /// Reference frequencies from gnomAD v2.1 and published literature
    fn load_population_frequencies() -> Vec<AlleleFrequencies> {
        let mut freqs = Vec::new();
        let mut map = std::collections::HashMap::new;

        // CYP2D6 allele frequencies by ancestry
        // Reference: Gaedigk et al., Clin Pharmacol Ther 2017
        freqs.push(AlleleFrequencies {
            gene: "CYP2D6".to_string(),
            allele: "*4".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::European, 0.20);     // ~20% in Europeans
                m.insert(Ancestry::African, 0.06);      // ~6% in Africans
                m.insert(Ancestry::EastAsian, 0.01);    // ~1% in East Asians
                m.insert(Ancestry::Latino, 0.10);       // ~10% in Latinos
                m.insert(Ancestry::CentralSouthAsian, 0.05); // ~5%
                m
            },
        });

        freqs.push(AlleleFrequencies {
            gene: "CYP2D6".to_string(),
            allele: "*10".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::EastAsian, 0.40);    // ~40% in East Asians
                m.insert(Ancestry::European, 0.02);     // ~2% in Europeans
                m.insert(Ancestry::African, 0.05);      // ~5%
                m.insert(Ancestry::Latino, 0.04);       // ~4%
                m
            },
        });

        freqs.push(AlleleFrequencies {
            gene: "CYP2D6".to_string(),
            allele: "*17".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::African, 0.20);      // ~20% in Africans
                m.insert(Ancestry::European, 0.001);    // Very rare in Europeans
                m.insert(Ancestry::EastAsian, 0.001);
                m.insert(Ancestry::Latino, 0.03);
                m
            },
        });

        freqs.push(AlleleFrequencies {
            gene: "CYP2D6".to_string(),
            allele: "*29".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::African, 0.10);      // ~10% in Africans
                m.insert(Ancestry::European, 0.001);
                m.insert(Ancestry::EastAsian, 0.001);
                m
            },
        });

        // CYP2C19 frequencies
        // Reference: Scott et al., Clin Pharmacol Ther 2012
        freqs.push(AlleleFrequencies {
            gene: "CYP2C19".to_string(),
            allele: "*2".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::EastAsian, 0.30);    // ~30% in East Asians
                m.insert(Ancestry::European, 0.15);     // ~15% in Europeans
                m.insert(Ancestry::African, 0.15);      // ~15%
                m.insert(Ancestry::Latino, 0.12);
                m
            },
        });

        freqs.push(AlleleFrequencies {
            gene: "CYP2C19".to_string(),
            allele: "*3".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::EastAsian, 0.08);    // ~8% in East Asians
                m.insert(Ancestry::European, 0.001);    // Very rare in Europeans
                m.insert(Ancestry::African, 0.001);
                m.insert(Ancestry::Latino, 0.001);
                m
            },
        });

        freqs.push(AlleleFrequencies {
            gene: "CYP2C19".to_string(),
            allele: "*17".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::European, 0.21);     // ~21% in Europeans
                m.insert(Ancestry::African, 0.16);      // ~16%
                m.insert(Ancestry::EastAsian, 0.01);    // ~1% in East Asians
                m.insert(Ancestry::Latino, 0.15);
                m
            },
        });

        // CYP2C9 frequencies
        freqs.push(AlleleFrequencies {
            gene: "CYP2C9".to_string(),
            allele: "*2".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::European, 0.13);     // ~13% in Europeans
                m.insert(Ancestry::African, 0.02);      // ~2%
                m.insert(Ancestry::EastAsian, 0.001);   // Very rare
                m.insert(Ancestry::Latino, 0.06);
                m
            },
        });

        freqs.push(AlleleFrequencies {
            gene: "CYP2C9".to_string(),
            allele: "*3".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::European, 0.07);     // ~7% in Europeans
                m.insert(Ancestry::African, 0.01);      // ~1%
                m.insert(Ancestry::EastAsian, 0.04);    // ~4%
                m.insert(Ancestry::Latino, 0.03);
                m
            },
        });

        // CYP3A5 frequencies (important for tacrolimus)
        freqs.push(AlleleFrequencies {
            gene: "CYP3A5".to_string(),
            allele: "*3".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::European, 0.85);     // ~85% in Europeans (loss of function)
                m.insert(Ancestry::African, 0.30);      // ~30% in Africans
                m.insert(Ancestry::EastAsian, 0.70);    // ~70%
                m.insert(Ancestry::Latino, 0.65);
                m
            },
        });

        // DPYD frequencies (5-FU toxicity)
        freqs.push(AlleleFrequencies {
            gene: "DPYD".to_string(),
            allele: "*2A".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::European, 0.01);     // ~1% in Europeans
                m.insert(Ancestry::African, 0.001);
                m.insert(Ancestry::EastAsian, 0.001);
                m
            },
        });

        // SLCO1B1 frequencies (statin myopathy)
        freqs.push(AlleleFrequencies {
            gene: "SLCO1B1".to_string(),
            allele: "*5".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::European, 0.15);     // ~15% in Europeans
                m.insert(Ancestry::EastAsian, 0.10);
                m.insert(Ancestry::African, 0.02);      // Lower in Africans
                m.insert(Ancestry::Latino, 0.08);
                m
            },
        });

        // TPMT frequencies (thiopurine toxicity)
        freqs.push(AlleleFrequencies {
            gene: "TPMT".to_string(),
            allele: "*3A".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::European, 0.05);     // ~5% in Europeans
                m.insert(Ancestry::African, 0.01);      // Lower in Africans
                m.insert(Ancestry::EastAsian, 0.001);   // Very rare
                m.insert(Ancestry::Latino, 0.03);
                m
            },
        });

        freqs.push(AlleleFrequencies {
            gene: "TPMT".to_string(),
            allele: "*3C".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::EastAsian, 0.02);    // ~2% in East Asians
                m.insert(Ancestry::African, 0.05);      // ~5% in Africans
                m.insert(Ancestry::European, 0.005);
                m
            },
        });

        // UGT1A1 frequencies (irinotecan toxicity)
        freqs.push(AlleleFrequencies {
            gene: "UGT1A1".to_string(),
            allele: "*28".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::European, 0.32);     // ~32% in Europeans
                m.insert(Ancestry::African, 0.42);      // ~42% in Africans
                m.insert(Ancestry::EastAsian, 0.15);    // ~15% in East Asians
                m.insert(Ancestry::Latino, 0.28);
                m
            },
        });

        // NUDT15 frequencies (thiopurine toxicity - especially important in Asians)
        freqs.push(AlleleFrequencies {
            gene: "NUDT15".to_string(),
            allele: "*3".to_string(),
            frequencies: {
                let mut m = map();
                m.insert(Ancestry::EastAsian, 0.10);    // ~10% in East Asians
                m.insert(Ancestry::Latino, 0.03);
                m.insert(Ancestry::European, 0.002);    // Very rare
                m.insert(Ancestry::African, 0.001);
                m
            },
        });

        freqs
    }

    /// Get allele frequency for a specific ancestry
    pub fn get_allele_frequency(
        &self,
        gene: &str,
        allele: &str,
        ancestry: &Ancestry,
    ) -> Option<f64> {
        self.frequencies
            .iter()
            .find(|f| f.gene == gene && f.allele == allele)
            .and_then(|f| f.frequencies.get(ancestry).copied())
    }

    /// Calculate prior probability of a phenotype given ancestry
    pub fn phenotype_prior(
        &self,
        gene: &str,
        phenotype: &MetabolizerPhenotype,
        ancestry: &Ancestry,
    ) -> f64 {
        match (gene, phenotype, ancestry) {
            // CYP2D6 phenotype priors
            ("CYP2D6", MetabolizerPhenotype::Poor, Ancestry::European) => 0.07,     // ~7%
            ("CYP2D6", MetabolizerPhenotype::Poor, Ancestry::African) => 0.02,       // ~2%
            ("CYP2D6", MetabolizerPhenotype::Poor, Ancestry::EastAsian) => 0.01,     // ~1%
            ("CYP2D6", MetabolizerPhenotype::Intermediate, Ancestry::EastAsian) => 0.35, // High due to *10
            ("CYP2D6", MetabolizerPhenotype::Ultrarapid, Ancestry::African) => 0.10,  // Gene duplications

            // CYP2C19 phenotype priors
            ("CYP2C19", MetabolizerPhenotype::Poor, Ancestry::EastAsian) => 0.15,    // ~15%
            ("CYP2C19", MetabolizerPhenotype::Poor, Ancestry::European) => 0.02,     // ~2%
            ("CYP2C19", MetabolizerPhenotype::Ultrarapid, Ancestry::European) => 0.05, // *17/*17

            // CYP3A5 phenotype priors (for tacrolimus dosing)
            ("CYP3A5", MetabolizerPhenotype::Poor, Ancestry::European) => 0.85,      // Most are *3/*3
            ("CYP3A5", MetabolizerPhenotype::Poor, Ancestry::African) => 0.10,       // More expressers

            // NUDT15 - critical for Asians on thiopurines
            ("NUDT15", MetabolizerPhenotype::Poor, Ancestry::EastAsian) => 0.01,     // ~1%
            ("NUDT15", MetabolizerPhenotype::Intermediate, Ancestry::EastAsian) => 0.18, // ~18%

            // Default priors when no specific data
            _ => match phenotype {
                MetabolizerPhenotype::Normal => 0.70,
                MetabolizerPhenotype::Intermediate => 0.20,
                MetabolizerPhenotype::Poor => 0.05,
                MetabolizerPhenotype::Ultrarapid => 0.03,
                MetabolizerPhenotype::RapidToNormal => 0.02,
                MetabolizerPhenotype::Indeterminate => 0.00,
            },
        }
    }

    /// Encode a diplotype with ancestry context
    pub fn encode_diplotype_with_ancestry(
        &self,
        gene: &str,
        allele1: &str,
        allele2: &str,
        ancestry: &Ancestry,
    ) -> Result<AncestryEncodedDiplotype, HdcError> {
        let base = self.base_encoder.encode_diplotype(gene, allele1, allele2)?;

        // Get allele frequencies for this ancestry
        let freq1 = self.get_allele_frequency(gene, allele1, ancestry);
        let freq2 = self.get_allele_frequency(gene, allele2, ancestry);

        // Calculate diplotype frequency (Hardy-Weinberg assumption)
        let diplotype_frequency = match (freq1, freq2) {
            (Some(f1), Some(f2)) if allele1 == allele2 => f1 * f1, // homozygous
            (Some(f1), Some(f2)) => 2.0 * f1 * f2,                  // heterozygous
            _ => None.or(freq1).or(freq2).unwrap_or(0.01),          // fallback
        };

        // Get phenotype prior for context
        let phenotype_prior = self.phenotype_prior(gene, &base.phenotype, ancestry);

        // Calculate ancestry-specific risk flags
        let ancestry_notes = self.get_ancestry_notes(gene, allele1, allele2, ancestry);

        Ok(AncestryEncodedDiplotype {
            base,
            ancestry: ancestry.clone(),
            diplotype_frequency,
            phenotype_prior,
            ancestry_notes,
        })
    }

    /// Get ancestry-specific clinical notes
    fn get_ancestry_notes(
        &self,
        gene: &str,
        allele1: &str,
        allele2: &str,
        ancestry: &Ancestry,
    ) -> Vec<String> {
        let mut notes = Vec::new();

        match (gene, ancestry) {
            ("CYP2D6", Ancestry::African) => {
                if allele1.contains("*17") || allele2.contains("*17") {
                    notes.push("CYP2D6*17 is common in African ancestry - reduced function allele".to_string());
                }
                if allele1.contains("*29") || allele2.contains("*29") {
                    notes.push("CYP2D6*29 is primarily found in African populations".to_string());
                }
            }
            ("CYP2D6", Ancestry::EastAsian) => {
                if allele1.contains("*10") || allele2.contains("*10") {
                    notes.push("CYP2D6*10 is very common in East Asian ancestry (~40%)".to_string());
                    notes.push("Consider starting at lower doses for CYP2D6 substrates".to_string());
                }
            }
            ("CYP2C19", Ancestry::EastAsian) => {
                if allele1.contains("*3") || allele2.contains("*3") {
                    notes.push("CYP2C19*3 is primarily found in East Asian populations".to_string());
                }
                notes.push("Higher prevalence of CYP2C19 poor metabolizers in East Asian ancestry".to_string());
            }
            ("CYP3A5", Ancestry::African) => {
                notes.push("CYP3A5 expression is more common in African ancestry".to_string());
                notes.push("May require higher tacrolimus doses compared to other populations".to_string());
            }
            ("NUDT15", Ancestry::EastAsian) => {
                notes.push("NUDT15 variants are more common in East Asian ancestry".to_string());
                notes.push("Consider NUDT15 testing before thiopurine therapy".to_string());
            }
            ("UGT1A1", Ancestry::African) => {
                notes.push("UGT1A1*28 is common in African ancestry (~42%)".to_string());
                notes.push("Higher risk of irinotecan toxicity".to_string());
            }
            _ => {}
        }

        notes
    }

    /// Encode a complete pharmacogenomic profile with ancestry
    pub fn encode_profile_with_ancestry(
        &self,
        diplotypes: &[(&str, &str, &str)],
        ancestry: &Ancestry,
    ) -> Result<AncestryEncodedProfile, HdcError> {
        let mut encoded_diplotypes = Vec::new();
        let mut all_notes = Vec::new();

        for (gene, allele1, allele2) in diplotypes {
            let encoded = self.encode_diplotype_with_ancestry(gene, allele1, allele2, ancestry)?;
            all_notes.extend(encoded.ancestry_notes.clone());
            encoded_diplotypes.push(encoded);
        }

        // Create composite vector
        let vectors: Vec<&Hypervector> = encoded_diplotypes
            .iter()
            .map(|d| &d.base.vector)
            .collect();
        let profile_vector = bundle(&vectors);

        // De-duplicate notes
        all_notes.sort();
        all_notes.dedup();

        Ok(AncestryEncodedProfile {
            diplotypes: encoded_diplotypes,
            profile_vector,
            ancestry: ancestry.clone(),
            ancestry_notes: all_notes,
        })
    }

    /// Predict drug interaction with ancestry context
    pub fn predict_drug_interaction_with_ancestry(
        &self,
        profile: &AncestryEncodedProfile,
        drug: &str,
    ) -> Option<AncestryDrugPrediction> {
        let base_prediction = self.base_encoder.predict_drug_interaction(
            &EncodedPgxProfile {
                diplotypes: profile.diplotypes.iter().map(|d| d.base.clone()).collect(),
                profile_vector: profile.profile_vector.clone(),
            },
            drug,
        )?;

        // Get ancestry-specific considerations
        let ancestry_considerations = self.get_drug_ancestry_considerations(
            drug,
            &base_prediction.gene,
            &profile.ancestry,
        );

        // Adjust confidence based on ancestry data availability
        let ancestry_confidence = match &profile.ancestry {
            Ancestry::European | Ancestry::EastAsian | Ancestry::African => 0.9,
            Ancestry::Latino | Ancestry::CentralSouthAsian => 0.7,
            Ancestry::NearEastern | Ancestry::American | Ancestry::Oceanian => 0.5,
            Ancestry::Mixed(_) => 0.6,
            Ancestry::Unknown => 0.4,
        };

        Some(AncestryDrugPrediction {
            base: base_prediction,
            ancestry: profile.ancestry.clone(),
            ancestry_confidence,
            ancestry_considerations,
        })
    }

    /// Get ancestry-specific drug considerations
    fn get_drug_ancestry_considerations(
        &self,
        drug: &str,
        gene: &str,
        ancestry: &Ancestry,
    ) -> Vec<String> {
        let mut considerations = Vec::new();

        match (drug.to_lowercase().as_str(), gene, ancestry) {
            ("clopidogrel", "CYP2C19", Ancestry::EastAsian) => {
                considerations.push("Higher prevalence of CYP2C19 poor metabolizers in East Asian populations".to_string());
                considerations.push("Consider alternative antiplatelet therapy or genetic testing".to_string());
            }
            ("codeine" | "tramadol", "CYP2D6", Ancestry::EastAsian) => {
                considerations.push("CYP2D6*10 is common - may have reduced conversion to active metabolite".to_string());
            }
            ("codeine" | "tramadol", "CYP2D6", Ancestry::African) => {
                considerations.push("Gene duplications more common - risk of ultrarapid metabolism".to_string());
            }
            ("tacrolimus", "CYP3A5", Ancestry::African) => {
                considerations.push("CYP3A5 expressers more common in African ancestry".to_string());
                considerations.push("May require higher tacrolimus doses for target trough levels".to_string());
            }
            ("azathioprine" | "mercaptopurine", _, Ancestry::EastAsian) => {
                considerations.push("NUDT15 variants common in East Asian ancestry".to_string());
                considerations.push("Consider reduced starting dose or NUDT15 testing".to_string());
            }
            ("warfarin", "CYP2C9", Ancestry::European) => {
                considerations.push("CYP2C9*2 and *3 common in Europeans - may require dose reduction".to_string());
            }
            ("simvastatin", "SLCO1B1", Ancestry::European) => {
                considerations.push("SLCO1B1*5 common in Europeans - increased myopathy risk".to_string());
            }
            _ => {}
        }

        considerations
    }

    /// Get population-specific dosing guidance
    pub fn get_dosing_guidance(
        &self,
        drug: &str,
        profile: &AncestryEncodedProfile,
    ) -> Option<DosingGuidance> {
        let prediction = self.predict_drug_interaction_with_ancestry(profile, drug)?;

        // Determine dose adjustment based on recommendation and ancestry
        let (dose_adjustment, reasoning) = match (&prediction.base.recommendation, &profile.ancestry) {
            (DrugRecommendation::Avoid, _) => {
                (DoseAdjustment::Contraindicated, "Alternative therapy recommended".to_string())
            }
            (DrugRecommendation::StandardDose, _) => {
                (DoseAdjustment::Standard, "Standard dosing appropriate".to_string())
            }
            (DrugRecommendation::ReducedDose, Ancestry::EastAsian) if prediction.base.gene == "CYP2D6" => {
                // Additional reduction for East Asians with *10 allele (common)
                (DoseAdjustment::Reduce(50), "Reduce dose by 50% (adjusted for East Asian CYP2D6*10 prevalence)".to_string())
            }
            (DrugRecommendation::ReducedDose, _) => {
                (DoseAdjustment::Reduce(25), "Reduce dose by 25%".to_string())
            }
            (DrugRecommendation::ConsiderAlternative, Ancestry::African) if prediction.base.gene == "CYP3A5" => {
                // CYP3A5 expressers in African ancestry may need higher doses
                (DoseAdjustment::Increase(50), "Increase dose by 50% (CYP3A5 expresser, common in African ancestry)".to_string())
            }
            (DrugRecommendation::ConsiderAlternative, _) => {
                // For ultrarapid metabolizers, may need dose increase
                match &prediction.base.phenotype {
                    MetabolizerPhenotype::Ultrarapid | MetabolizerPhenotype::RapidToNormal => {
                        (DoseAdjustment::Increase(25), "Consider increased dose or alternative therapy".to_string())
                    }
                    _ => {
                        (DoseAdjustment::CautionNeeded, "Consider alternative therapy".to_string())
                    }
                }
            }
            (DrugRecommendation::UseWithCaution, _) => {
                (DoseAdjustment::CautionNeeded, "Use with caution - enhanced monitoring recommended".to_string())
            }
            (DrugRecommendation::InsufficientEvidence, _) => {
                (DoseAdjustment::CautionNeeded, "Clinical monitoring recommended - limited evidence".to_string())
            }
        };

        Some(DosingGuidance {
            drug: drug.to_string(),
            adjustment: dose_adjustment,
            reasoning,
            confidence: prediction.ancestry_confidence,
            considerations: prediction.ancestry_considerations,
        })
    }
}

/// Diplotype encoding with ancestry context
#[derive(Clone, Debug)]
pub struct AncestryEncodedDiplotype {
    /// Base diplotype encoding
    pub base: EncodedDiplotype,
    /// Patient ancestry
    pub ancestry: Ancestry,
    /// Diplotype frequency in this population
    pub diplotype_frequency: f64,
    /// Prior probability of this phenotype in this population
    pub phenotype_prior: f64,
    /// Ancestry-specific clinical notes
    pub ancestry_notes: Vec<String>,
}

/// Complete pharmacogenomic profile with ancestry
#[derive(Clone, Debug)]
pub struct AncestryEncodedProfile {
    /// Individual diplotype encodings with ancestry context
    pub diplotypes: Vec<AncestryEncodedDiplotype>,
    /// Composite profile vector
    pub profile_vector: Hypervector,
    /// Patient ancestry
    pub ancestry: Ancestry,
    /// Combined ancestry-specific notes
    pub ancestry_notes: Vec<String>,
}

/// Drug prediction with ancestry context
#[derive(Clone, Debug)]
pub struct AncestryDrugPrediction {
    /// Base drug interaction prediction
    pub base: DrugInteractionPrediction,
    /// Patient ancestry
    pub ancestry: Ancestry,
    /// Confidence in prediction given ancestry data
    pub ancestry_confidence: f64,
    /// Ancestry-specific considerations
    pub ancestry_considerations: Vec<String>,
}

/// Dose adjustment type
#[derive(Clone, Debug, PartialEq)]
pub enum DoseAdjustment {
    /// Standard dose
    Standard,
    /// Reduce by percentage
    Reduce(u8),
    /// Increase by percentage
    Increase(u8),
    /// Drug is contraindicated
    Contraindicated,
    /// Caution needed, no clear guidance
    CautionNeeded,
}

/// Complete dosing guidance
#[derive(Clone, Debug)]
pub struct DosingGuidance {
    /// Drug name
    pub drug: String,
    /// Dose adjustment recommendation
    pub adjustment: DoseAdjustment,
    /// Reasoning for adjustment
    pub reasoning: String,
    /// Confidence in recommendation (0.0-1.0)
    pub confidence: f64,
    /// Additional considerations
    pub considerations: Vec<String>,
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

    #[test]
    fn test_star_allele_encoder_cyp2d6() {
        let seed = Seed::from_string("pharmaco-test");
        let encoder = StarAlleleEncoder::new(seed);

        // Normal metabolizer: *1/*1
        let normal = encoder.encode_diplotype("CYP2D6", "*1", "*1").unwrap();
        assert!((normal.activity_score - 2.0).abs() < 0.01);
        assert_eq!(normal.phenotype, MetabolizerPhenotype::Normal);

        // Poor metabolizer: *4/*4
        let poor = encoder.encode_diplotype("CYP2D6", "*4", "*4").unwrap();
        assert!((poor.activity_score - 0.0).abs() < 0.01);
        assert_eq!(poor.phenotype, MetabolizerPhenotype::Poor);

        // Intermediate metabolizer: *1/*4
        let intermediate = encoder.encode_diplotype("CYP2D6", "*1", "*4").unwrap();
        assert!((intermediate.activity_score - 1.0).abs() < 0.01);
        assert_eq!(intermediate.phenotype, MetabolizerPhenotype::Intermediate);
    }

    #[test]
    fn test_star_allele_cyp2c19_ultrarapid() {
        let seed = Seed::from_string("pharmaco-test");
        let encoder = StarAlleleEncoder::new(seed);

        // Ultrarapid: *17/*17
        let ultrarapid = encoder.encode_diplotype("CYP2C19", "*17", "*17").unwrap();
        assert!((ultrarapid.activity_score - 3.0).abs() < 0.01);
        assert_eq!(ultrarapid.phenotype, MetabolizerPhenotype::Ultrarapid);
    }

    #[test]
    fn test_pgx_profile() {
        let seed = Seed::from_string("pharmaco-test");
        let encoder = StarAlleleEncoder::new(seed);

        let profile = encoder.encode_profile(&[
            ("CYP2D6", "*1", "*4"),   // Intermediate
            ("CYP2C19", "*1", "*1"),  // Normal
            ("CYP2C9", "*1", "*3"),   // Intermediate (0.5)
        ]).unwrap();

        assert_eq!(profile.diplotypes.len(), 3);

        // Check individual phenotypes
        let cyp2d6 = profile.get_diplotype("CYP2D6").unwrap();
        assert_eq!(cyp2d6.phenotype, MetabolizerPhenotype::Intermediate);

        let cyp2c9 = profile.get_diplotype("CYP2C9").unwrap();
        assert_eq!(cyp2c9.phenotype, MetabolizerPhenotype::Intermediate);
    }

    #[test]
    fn test_drug_interaction_prediction() {
        let seed = Seed::from_string("pharmaco-test");
        let encoder = StarAlleleEncoder::new(seed);

        // Patient with CYP2D6 *4/*4 (poor metabolizer)
        let profile = encoder.encode_profile(&[
            ("CYP2D6", "*4", "*4"),
        ]).unwrap();

        // Codeine is metabolized by CYP2D6 to morphine
        let prediction = encoder.predict_drug_interaction(&profile, "codeine").unwrap();
        assert_eq!(prediction.gene, "CYP2D6");
        assert_eq!(prediction.phenotype, MetabolizerPhenotype::Poor);
        assert_eq!(prediction.recommendation, DrugRecommendation::Avoid);
    }

    #[test]
    fn test_profile_similarity() {
        let seed = Seed::from_string("pharmaco-test");
        let encoder = StarAlleleEncoder::new(seed);

        // Identical profiles
        let profile1 = encoder.encode_profile(&[
            ("CYP2D6", "*1", "*1"),
            ("CYP2C19", "*1", "*2"),
        ]).unwrap();

        let profile2 = encoder.encode_profile(&[
            ("CYP2D6", "*1", "*1"),
            ("CYP2C19", "*1", "*2"),
        ]).unwrap();

        let sim_identical = encoder.profile_similarity(&profile1, &profile2);
        assert!(sim_identical > 0.99, "Identical profiles should have sim > 0.99");

        // Different profiles
        let profile3 = encoder.encode_profile(&[
            ("CYP2D6", "*4", "*4"),
            ("CYP2C19", "*17", "*17"),
        ]).unwrap();

        let sim_different = encoder.profile_similarity(&profile1, &profile3);
        assert!(sim_different < sim_identical, "Different profiles should be less similar");
    }

    #[test]
    fn test_diplotype_notation() {
        let seed = Seed::from_string("pharmaco-test");
        let encoder = StarAlleleEncoder::new(seed);

        let diplotype = encoder.encode_diplotype("CYP2D6", "*1", "*4").unwrap();
        assert_eq!(diplotype.to_notation(), "CYP2D6 *1/*4");
    }

    #[test]
    fn test_poor_metabolizer_genes() {
        let seed = Seed::from_string("pharmaco-test");
        let encoder = StarAlleleEncoder::new(seed);

        let profile = encoder.encode_profile(&[
            ("CYP2D6", "*4", "*4"),   // Poor
            ("CYP2C19", "*1", "*1"),  // Normal
            ("CYP2C9", "*3", "*3"),   // Poor
        ]).unwrap();

        let poor_genes = profile.get_poor_metabolizer_genes();
        assert!(poor_genes.contains(&"CYP2D6"));
        assert!(poor_genes.contains(&"CYP2C9"));
        assert!(!poor_genes.contains(&"CYP2C19"));
    }

    // Ancestry-Informed Pharmacogenomics Tests

    #[test]
    fn test_ancestry_encoder_basic() {
        let seed = Seed::from_string("ancestry-test");
        let encoder = AncestryInformedEncoder::new(seed);

        let result = encoder.encode_diplotype_with_ancestry(
            "CYP2D6", "*1", "*4",
            &Ancestry::European,
        );
        assert!(result.is_ok());

        let encoded = result.unwrap();
        assert_eq!(encoded.base.phenotype, MetabolizerPhenotype::Intermediate);
        assert!(encoded.diplotype_frequency > 0.0);
    }

    #[test]
    fn test_ancestry_specific_frequencies() {
        let seed = Seed::from_string("ancestry-test");
        let encoder = AncestryInformedEncoder::new(seed);

        // CYP2D6*4 is more common in Europeans
        let european_freq = encoder.get_allele_frequency("CYP2D6", "*4", &Ancestry::European);
        let east_asian_freq = encoder.get_allele_frequency("CYP2D6", "*4", &Ancestry::EastAsian);

        assert!(european_freq.is_some());
        assert!(east_asian_freq.is_some());
        assert!(european_freq.unwrap() > east_asian_freq.unwrap());

        // CYP2D6*10 is more common in East Asians
        let european_10 = encoder.get_allele_frequency("CYP2D6", "*10", &Ancestry::European);
        let east_asian_10 = encoder.get_allele_frequency("CYP2D6", "*10", &Ancestry::EastAsian);

        assert!(east_asian_10.unwrap() > european_10.unwrap());
    }

    #[test]
    fn test_phenotype_priors() {
        let seed = Seed::from_string("ancestry-test");
        let encoder = AncestryInformedEncoder::new(seed);

        // CYP2D6 poor metabolizer more common in Europeans
        let european_pm = encoder.phenotype_prior("CYP2D6", &MetabolizerPhenotype::Poor, &Ancestry::European);
        let african_pm = encoder.phenotype_prior("CYP2D6", &MetabolizerPhenotype::Poor, &Ancestry::African);

        assert!(european_pm > african_pm);

        // CYP2C19 poor metabolizer more common in East Asians
        let east_asian_pm = encoder.phenotype_prior("CYP2C19", &MetabolizerPhenotype::Poor, &Ancestry::EastAsian);
        let european_c19_pm = encoder.phenotype_prior("CYP2C19", &MetabolizerPhenotype::Poor, &Ancestry::European);

        assert!(east_asian_pm > european_c19_pm);
    }

    #[test]
    fn test_ancestry_profile_encoding() {
        let seed = Seed::from_string("ancestry-test");
        let encoder = AncestryInformedEncoder::new(seed);

        let profile = encoder.encode_profile_with_ancestry(&[
            ("CYP2D6", "*10", "*10"),
            ("CYP2C19", "*2", "*3"),
        ], &Ancestry::EastAsian).unwrap();

        assert_eq!(profile.diplotypes.len(), 2);
        assert_eq!(profile.ancestry, Ancestry::EastAsian);
        assert!(!profile.ancestry_notes.is_empty());
    }

    #[test]
    fn test_ancestry_drug_prediction() {
        let seed = Seed::from_string("ancestry-test");
        let encoder = AncestryInformedEncoder::new(seed);

        // East Asian patient with CYP2C19*2/*3 (poor metabolizer)
        let profile = encoder.encode_profile_with_ancestry(&[
            ("CYP2C19", "*2", "*3"),
        ], &Ancestry::EastAsian).unwrap();

        let prediction = encoder.predict_drug_interaction_with_ancestry(&profile, "clopidogrel");
        assert!(prediction.is_some());

        let pred = prediction.unwrap();
        assert_eq!(pred.base.gene, "CYP2C19");
        assert!(pred.ancestry_confidence > 0.8);
        assert!(!pred.ancestry_considerations.is_empty());
    }

    #[test]
    fn test_dosing_guidance() {
        let seed = Seed::from_string("ancestry-test");
        let encoder = AncestryInformedEncoder::new(seed);

        // African patient with CYP3A5*1/*1 (expresser)
        let profile = encoder.encode_profile_with_ancestry(&[
            ("CYP3A5", "*1", "*1"),
        ], &Ancestry::African).unwrap();

        let guidance = encoder.get_dosing_guidance("tacrolimus", &profile);
        assert!(guidance.is_some());

        let guide = guidance.unwrap();
        assert!(matches!(guide.adjustment, DoseAdjustment::Increase(_)));
    }

    #[test]
    fn test_ancestry_notes_generation() {
        let seed = Seed::from_string("ancestry-test");
        let encoder = AncestryInformedEncoder::new(seed);

        // East Asian with common *10 allele
        let result = encoder.encode_diplotype_with_ancestry(
            "CYP2D6", "*10", "*10",
            &Ancestry::EastAsian,
        ).unwrap();

        // Should have ancestry-specific notes about *10
        assert!(result.ancestry_notes.iter().any(|n| n.contains("*10")));
        assert!(result.ancestry_notes.iter().any(|n| n.contains("East Asian")));
    }

    #[test]
    fn test_nudt15_east_asian_warning() {
        let seed = Seed::from_string("ancestry-test");
        let encoder = AncestryInformedEncoder::new(seed);

        // Include both TPMT (for drug mapping) and NUDT15 (for East Asian warnings)
        let profile = encoder.encode_profile_with_ancestry(&[
            ("TPMT", "*3A", "*3A"),  // Poor metabolizer
            ("NUDT15", "*3", "*3"),
        ], &Ancestry::EastAsian).unwrap();

        // Should have warning about NUDT15 in East Asians
        assert!(profile.ancestry_notes.iter().any(|n| n.contains("NUDT15")));

        // Azathioprine maps to TPMT, and East Asian patients should get considerations
        let prediction = encoder.predict_drug_interaction_with_ancestry(&profile, "azathioprine");
        assert!(prediction.is_some());
        // Should have East Asian-specific thiopurine considerations
        assert!(!prediction.unwrap().ancestry_considerations.is_empty());
    }

    #[test]
    fn test_ancestry_display() {
        assert_eq!(format!("{}", Ancestry::European), "European");
        assert_eq!(format!("{}", Ancestry::EastAsian), "East Asian");
        assert_eq!(format!("{}", Ancestry::African), "African");

        let mixed = Ancestry::Mixed(vec![Ancestry::European, Ancestry::African]);
        assert!(format!("{}", mixed).contains("Mixed"));
    }

    #[test]
    #[cfg(feature = "learned")]
    fn test_learned_kmer_codebook() {
        use std::collections::HashMap;

        // Create synthetic learned embeddings
        let mut embeddings: HashMap<String, Vec<f32>> = HashMap::new();

        // Add some k-mers with distinct embeddings
        // Positive values will become 1, negative become 0
        embeddings.insert("ACGTAC".to_string(), vec![0.5, -0.3, 0.8, -0.1, 0.9, -0.5, 0.2, -0.8]);
        embeddings.insert("CGTACG".to_string(), vec![-0.2, 0.8, -0.4, 0.6, -0.1, 0.9, -0.7, 0.3]);
        embeddings.insert("GTACGT".to_string(), vec![0.1, 0.2, 0.3, -0.4, -0.5, 0.6, 0.7, -0.8]);

        // Create codebook from embeddings
        let codebook = LearnedKmerCodebook::from_embeddings(embeddings, 6).unwrap();

        assert_eq!(codebook.len(), 3);
        assert_eq!(codebook.kmer_length(), 6);
        assert!(!codebook.is_empty());

        // Verify we can look up k-mers
        assert!(codebook.get("ACGTAC").is_some());
        assert!(codebook.get("CGTACG").is_some());
        assert!(codebook.get("GTACGT").is_some());
        assert!(codebook.get("XXXXXX").is_none());
    }

    #[test]
    #[cfg(feature = "learned")]
    fn test_encode_with_learned_codebook() {
        use std::collections::HashMap;

        // Generate all 4096 6-mers with distinct embeddings
        let mut embeddings: HashMap<String, Vec<f32>> = HashMap::new();
        let bases = ['A', 'C', 'G', 'T'];

        for b1 in &bases {
            for b2 in &bases {
                for b3 in &bases {
                    for b4 in &bases {
                        for b5 in &bases {
                            for b6 in &bases {
                                let kmer = format!("{}{}{}{}{}{}", b1, b2, b3, b4, b5, b6);
                                // Create pseudo-random embedding based on k-mer
                                let hash = kmer.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
                                let embedding: Vec<f32> = (0..100).map(|i| {
                                    let v = ((hash.wrapping_add(i)) % 100) as f32 / 50.0 - 1.0;
                                    v
                                }).collect();
                                embeddings.insert(kmer, embedding);
                            }
                        }
                    }
                }
            }
        }

        let codebook = LearnedKmerCodebook::from_embeddings(embeddings, 6).unwrap();

        // Create encoder and encode a sequence
        let seed = Seed::from_string("test");
        let encoder = DnaEncoder::new(seed, 6);

        let result = encoder.encode_with_learned_codebook("ACGTACGTACGT", &codebook);
        assert!(result.is_ok());

        let encoded = result.unwrap();
        assert_eq!(encoded.kmer_count, 7); // 12 - 6 + 1 = 7
        assert_eq!(encoded.kmer_length, 6);
        assert_eq!(encoded.sequence_length, 12);
    }
}
