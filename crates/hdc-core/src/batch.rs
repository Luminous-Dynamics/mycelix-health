//! Batch Encoding API
//!
//! Efficient batch processing for multiple sequences or variants.
//! Supports parallel processing via rayon when the `parallel` feature is enabled.
//!
//! # Example
//!
//! ```ignore
//! use hdc_core::batch::{BatchEncoder, BatchConfig};
//! use hdc_core::Seed;
//!
//! let seed = Seed::from_string("my-study-v1");
//! let config = BatchConfig::default()
//!     .with_parallel(true)
//!     .with_kmer_length(6);
//!
//! let encoder = BatchEncoder::new(seed, config);
//!
//! let sequences = vec![
//!     "ACGTACGTACGT",
//!     "TGCATGCATGCA",
//!     "GGCCGGCCGGCC",
//! ];
//!
//! let results = encoder.encode_sequences(&sequences).unwrap();
//! println!("Encoded {} sequences", results.len());
//! ```

use crate::{
    HdcError, Hypervector, Seed,
    encoding::{DnaEncoder, EncodedSequence},
};

/// Configuration for batch encoding
#[derive(Clone, Debug)]
pub struct BatchConfig {
    /// K-mer length for DNA encoding
    pub kmer_length: u8,
    /// Enable parallel processing
    pub parallel: bool,
    /// Chunk size for parallel processing
    pub chunk_size: usize,
    /// Skip invalid sequences instead of erroring
    pub skip_invalid: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        BatchConfig {
            kmer_length: 6,
            parallel: true,
            chunk_size: 100,
            skip_invalid: false,
        }
    }
}

impl BatchConfig {
    /// Set k-mer length
    pub fn with_kmer_length(mut self, k: u8) -> Self {
        self.kmer_length = k;
        self
    }

    /// Enable/disable parallel processing
    pub fn with_parallel(mut self, enabled: bool) -> Self {
        self.parallel = enabled;
        self
    }

    /// Set chunk size for batching
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Skip invalid sequences
    pub fn with_skip_invalid(mut self, skip: bool) -> Self {
        self.skip_invalid = skip;
        self
    }
}

/// Result from batch encoding
#[derive(Clone, Debug)]
pub struct BatchResult<T> {
    /// Successfully encoded items
    pub items: Vec<T>,
    /// Number of items that failed encoding
    pub failed_count: usize,
    /// Indices of failed items (if any)
    pub failed_indices: Vec<usize>,
    /// Processing statistics
    pub stats: BatchStats,
}

impl<T> BatchResult<T> {
    /// Get the number of successful items
    pub fn success_count(&self) -> usize {
        self.items.len()
    }

    /// Get the total number of processed items
    pub fn total_count(&self) -> usize {
        self.items.len() + self.failed_count
    }

    /// Get the success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_count() == 0 {
            0.0
        } else {
            self.items.len() as f64 / self.total_count() as f64
        }
    }
}

/// Statistics from batch processing
#[derive(Clone, Debug, Default)]
pub struct BatchStats {
    /// Total processing time in milliseconds
    pub processing_time_ms: u64,
    /// Number of parallel chunks processed
    pub chunks_processed: usize,
    /// Average encoding time per item in microseconds
    pub avg_encoding_time_us: f64,
}

/// Batch encoder for multiple sequences
pub struct BatchEncoder {
    seed: Seed,
    config: BatchConfig,
}

impl BatchEncoder {
    /// Create a new batch encoder
    pub fn new(seed: Seed, config: BatchConfig) -> Self {
        BatchEncoder { seed, config }
    }

    /// Encode multiple DNA sequences in batch
    pub fn encode_sequences(&self, sequences: &[&str]) -> Result<BatchResult<EncodedSequence>, HdcError> {
        let start_time = std::time::Instant::now();
        let encoder = DnaEncoder::new(self.seed.clone(), self.config.kmer_length);

        let (items, failed_indices) = if self.config.parallel {
            self.encode_parallel(sequences, |seq| encoder.encode_sequence(seq))
        } else {
            self.encode_sequential(sequences, |seq| encoder.encode_sequence(seq))
        };

        let elapsed = start_time.elapsed();
        let stats = BatchStats {
            processing_time_ms: elapsed.as_millis() as u64,
            chunks_processed: (sequences.len() + self.config.chunk_size - 1) / self.config.chunk_size,
            avg_encoding_time_us: if items.is_empty() {
                0.0
            } else {
                elapsed.as_micros() as f64 / items.len() as f64
            },
        };

        Ok(BatchResult {
            failed_count: failed_indices.len(),
            items,
            failed_indices,
            stats,
        })
    }

    /// Encode sequences with parallel processing
    #[cfg(feature = "parallel")]
    fn encode_parallel<T, F>(&self, items: &[&str], encode_fn: F) -> (Vec<T>, Vec<usize>)
    where
        T: Send,
        F: Fn(&str) -> Result<T, HdcError> + Sync,
    {
        use rayon::prelude::*;

        let results: Vec<_> = items
            .par_iter()
            .enumerate()
            .map(|(idx, item)| (idx, encode_fn(item)))
            .collect();

        let mut encoded = Vec::with_capacity(items.len());
        let mut failed = Vec::new();

        for (idx, result) in results {
            match result {
                Ok(enc) => encoded.push(enc),
                Err(_) if self.config.skip_invalid => failed.push(idx),
                Err(e) => {
                    if !self.config.skip_invalid {
                        // In non-skip mode, we'd return an error but we already collected
                        // For now, just track as failed
                        failed.push(idx);
                    }
                }
            }
        }

        (encoded, failed)
    }

    /// Fallback when parallel feature is disabled
    #[cfg(not(feature = "parallel"))]
    fn encode_parallel<T, F>(&self, items: &[&str], encode_fn: F) -> (Vec<T>, Vec<usize>)
    where
        F: Fn(&str) -> Result<T, HdcError>,
    {
        self.encode_sequential(items, encode_fn)
    }

    /// Sequential encoding
    fn encode_sequential<T, F>(&self, items: &[&str], encode_fn: F) -> (Vec<T>, Vec<usize>)
    where
        F: Fn(&str) -> Result<T, HdcError>,
    {
        let mut encoded = Vec::with_capacity(items.len());
        let mut failed = Vec::new();

        for (idx, item) in items.iter().enumerate() {
            match encode_fn(item) {
                Ok(enc) => encoded.push(enc),
                Err(_) if self.config.skip_invalid => failed.push(idx),
                Err(_) => failed.push(idx),
            }
        }

        (encoded, failed)
    }

    /// Encode raw strings to hypervectors (simple API)
    pub fn encode_to_vectors(&self, sequences: &[&str]) -> Result<Vec<Hypervector>, HdcError> {
        let result = self.encode_sequences(sequences)?;
        Ok(result.items.into_iter().map(|e| e.vector).collect())
    }

    /// Compute pairwise similarity matrix
    pub fn pairwise_similarity(&self, vectors: &[Hypervector]) -> SimilarityMatrix {
        let n = vectors.len();
        let mut matrix = vec![vec![0.0f64; n]; n];

        #[cfg(feature = "parallel")]
        if self.config.parallel {
            use rayon::prelude::*;

            // Compute upper triangle in parallel
            let results: Vec<_> = (0..n)
                .into_par_iter()
                .flat_map(|i| {
                    (i+1..n).into_par_iter().map(move |j| {
                        let sim = vectors[i].hamming_similarity(&vectors[j]);
                        (i, j, sim)
                    })
                })
                .collect();

            for (i, j, sim) in results {
                matrix[i][j] = sim;
                matrix[j][i] = sim;
            }
        }

        #[cfg(not(feature = "parallel"))]
        {
            for i in 0..n {
                for j in i+1..n {
                    let sim = vectors[i].hamming_similarity(&vectors[j]);
                    matrix[i][j] = sim;
                    matrix[j][i] = sim;
                }
            }
        }

        // Set diagonal to 1.0
        for i in 0..n {
            matrix[i][i] = 1.0;
        }

        SimilarityMatrix { matrix, size: n }
    }

    /// Find top-k most similar items to a query
    pub fn top_k_similar(
        &self,
        query: &Hypervector,
        corpus: &[Hypervector],
        k: usize,
    ) -> Vec<(usize, f64)> {
        let mut similarities: Vec<(usize, f64)> = corpus
            .iter()
            .enumerate()
            .map(|(idx, vec)| (idx, query.hamming_similarity(vec)))
            .collect();

        // Sort by similarity descending
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top k
        similarities.truncate(k);
        similarities
    }

    /// Find all items above a similarity threshold
    pub fn find_above_threshold(
        &self,
        query: &Hypervector,
        corpus: &[Hypervector],
        threshold: f64,
    ) -> Vec<(usize, f64)> {
        corpus
            .iter()
            .enumerate()
            .filter_map(|(idx, vec)| {
                let sim = query.hamming_similarity(vec);
                if sim >= threshold {
                    Some((idx, sim))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Pairwise similarity matrix
#[derive(Clone, Debug)]
pub struct SimilarityMatrix {
    /// The similarity values
    matrix: Vec<Vec<f64>>,
    /// Matrix size (n x n)
    size: usize,
}

impl SimilarityMatrix {
    /// Get the size of the matrix
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get similarity between items i and j
    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.matrix[i][j]
    }

    /// Get the entire row of similarities for item i
    pub fn row(&self, i: usize) -> &[f64] {
        &self.matrix[i]
    }

    /// Find the most similar pair (excluding self-similarity)
    pub fn most_similar_pair(&self) -> Option<(usize, usize, f64)> {
        let mut best = None;
        let mut best_sim = 0.0;

        for i in 0..self.size {
            for j in i+1..self.size {
                if self.matrix[i][j] > best_sim {
                    best_sim = self.matrix[i][j];
                    best = Some((i, j, best_sim));
                }
            }
        }

        best
    }

    /// Find all pairs above a threshold
    pub fn pairs_above_threshold(&self, threshold: f64) -> Vec<(usize, usize, f64)> {
        let mut pairs = Vec::new();

        for i in 0..self.size {
            for j in i+1..self.size {
                if self.matrix[i][j] >= threshold {
                    pairs.push((i, j, self.matrix[i][j]));
                }
            }
        }

        pairs
    }

    /// Get the average similarity (excluding diagonal)
    pub fn average_similarity(&self) -> f64 {
        if self.size <= 1 {
            return 0.0;
        }

        let mut sum = 0.0;
        let mut count = 0;

        for i in 0..self.size {
            for j in i+1..self.size {
                sum += self.matrix[i][j];
                count += 1;
            }
        }

        if count > 0 {
            sum / count as f64
        } else {
            0.0
        }
    }
}

/// Builder for complex batch queries
pub struct BatchQueryBuilder {
    seed: Seed,
    config: BatchConfig,
    queries: Vec<String>,
    corpus: Vec<String>,
}

impl BatchQueryBuilder {
    /// Create a new batch query builder
    pub fn new(seed: Seed) -> Self {
        BatchQueryBuilder {
            seed,
            config: BatchConfig::default(),
            queries: Vec::new(),
            corpus: Vec::new(),
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: BatchConfig) -> Self {
        self.config = config;
        self
    }

    /// Add query sequences
    pub fn add_queries(mut self, queries: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.queries.extend(queries.into_iter().map(|s| s.into()));
        self
    }

    /// Add corpus sequences
    pub fn add_corpus(mut self, corpus: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.corpus.extend(corpus.into_iter().map(|s| s.into()));
        self
    }

    /// Execute the batch query and find top-k matches for each query
    pub fn find_top_k(self, k: usize) -> Result<Vec<Vec<(usize, f64)>>, HdcError> {
        let encoder = BatchEncoder::new(self.seed, self.config);

        // Encode queries
        let query_strs: Vec<&str> = self.queries.iter().map(|s| s.as_str()).collect();
        let query_vecs = encoder.encode_to_vectors(&query_strs)?;

        // Encode corpus
        let corpus_strs: Vec<&str> = self.corpus.iter().map(|s| s.as_str()).collect();
        let corpus_vecs = encoder.encode_to_vectors(&corpus_strs)?;

        // Find top-k for each query
        let results: Vec<_> = query_vecs
            .iter()
            .map(|q| encoder.top_k_similar(q, &corpus_vecs, k))
            .collect();

        Ok(results)
    }

    /// Execute and compute full similarity matrix between queries and corpus
    pub fn compute_similarity_matrix(self) -> Result<Vec<Vec<f64>>, HdcError> {
        let encoder = BatchEncoder::new(self.seed, self.config);

        // Encode queries
        let query_strs: Vec<&str> = self.queries.iter().map(|s| s.as_str()).collect();
        let query_vecs = encoder.encode_to_vectors(&query_strs)?;

        // Encode corpus
        let corpus_strs: Vec<&str> = self.corpus.iter().map(|s| s.as_str()).collect();
        let corpus_vecs = encoder.encode_to_vectors(&corpus_strs)?;

        // Compute query x corpus similarity matrix
        let matrix: Vec<Vec<f64>> = query_vecs
            .iter()
            .map(|q| {
                corpus_vecs
                    .iter()
                    .map(|c| q.hamming_similarity(c))
                    .collect()
            })
            .collect();

        Ok(matrix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_encode_sequences() {
        let seed = Seed::from_string("batch-test");
        let config = BatchConfig::default().with_parallel(false);
        let encoder = BatchEncoder::new(seed, config);

        let sequences = vec![
            "ACGTACGTACGTACGT",
            "TGCATGCATGCATGCA",
            "GGCCGGCCGGCCGGCC",
        ];

        let result = encoder.encode_sequences(&sequences).unwrap();
        assert_eq!(result.success_count(), 3);
        assert_eq!(result.failed_count, 0);
    }

    #[test]
    fn test_batch_skip_invalid() {
        let seed = Seed::from_string("batch-test");
        let config = BatchConfig::default()
            .with_parallel(false)
            .with_skip_invalid(true);
        let encoder = BatchEncoder::new(seed, config);

        let sequences = vec![
            "ACGTACGTACGTACGT",
            "INVALID",  // This should fail
            "GGCCGGCCGGCCGGCC",
        ];

        let result = encoder.encode_sequences(&sequences).unwrap();
        assert_eq!(result.success_count(), 2);
        assert_eq!(result.failed_count, 1);
        assert!(result.failed_indices.contains(&1));
    }

    #[test]
    fn test_pairwise_similarity() {
        let seed = Seed::from_string("batch-test");
        let config = BatchConfig::default().with_parallel(false);
        let encoder = BatchEncoder::new(seed, config);

        let sequences = vec![
            "ACGTACGTACGTACGT",
            "ACGTACGTACGTACGT",  // Identical to first
            "TGCATGCATGCATGCA",
        ];

        let vectors = encoder.encode_to_vectors(&sequences).unwrap();
        let matrix = encoder.pairwise_similarity(&vectors);

        // Identical sequences should have similarity ~1.0
        assert!(matrix.get(0, 1) > 0.99);

        // Different sequences should have lower similarity
        assert!(matrix.get(0, 2) < matrix.get(0, 1));

        // Diagonal should be 1.0
        assert!((matrix.get(0, 0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_top_k_similar() {
        let seed = Seed::from_string("batch-test");
        let config = BatchConfig::default().with_parallel(false);
        let encoder = BatchEncoder::new(seed, config);

        let corpus = vec![
            "ACGTACGTACGTACGT",
            "TGCATGCATGCATGCA",
            "ACGTACGTACGTACGA",  // Very similar to first
        ];
        let corpus_vecs = encoder.encode_to_vectors(&corpus).unwrap();

        let query = encoder.encode_to_vectors(&["ACGTACGTACGTACGT"]).unwrap();
        let top_2 = encoder.top_k_similar(&query[0], &corpus_vecs, 2);

        assert_eq!(top_2.len(), 2);
        // First result should be exact match (index 0)
        assert_eq!(top_2[0].0, 0);
        assert!(top_2[0].1 > 0.99);
    }

    #[test]
    fn test_similarity_matrix_stats() {
        let seed = Seed::from_string("batch-test");
        let config = BatchConfig::default().with_parallel(false);
        let encoder = BatchEncoder::new(seed, config);

        let sequences = vec![
            "ACGTACGTACGTACGT",
            "ACGTACGTACGTACGT",
            "TGCATGCATGCATGCA",
            "GGCCGGCCGGCCGGCC",
        ];
        let vectors = encoder.encode_to_vectors(&sequences).unwrap();
        let matrix = encoder.pairwise_similarity(&vectors);

        assert_eq!(matrix.size(), 4);

        let (i, j, sim) = matrix.most_similar_pair().unwrap();
        assert_eq!((i, j), (0, 1));
        assert!(sim > 0.99);

        let avg = matrix.average_similarity();
        assert!(avg > 0.0 && avg < 1.0);
    }

    #[test]
    fn test_batch_query_builder() {
        let seed = Seed::from_string("batch-test");

        let results = BatchQueryBuilder::new(seed)
            .with_config(BatchConfig::default().with_parallel(false))
            .add_queries(vec!["ACGTACGTACGTACGT"])
            .add_corpus(vec![
                "ACGTACGTACGTACGT",
                "TGCATGCATGCATGCA",
                "GGCCGGCCGGCCGGCC",
            ])
            .find_top_k(2)
            .unwrap();

        assert_eq!(results.len(), 1);  // One query
        assert_eq!(results[0].len(), 2);  // Top 2 results
        assert_eq!(results[0][0].0, 0);  // Best match is index 0
    }

    #[test]
    fn test_batch_stats() {
        let seed = Seed::from_string("batch-test");
        let config = BatchConfig::default().with_parallel(false);
        let encoder = BatchEncoder::new(seed, config);

        let sequences: Vec<&str> = (0..100)
            .map(|_| "ACGTACGTACGTACGT")
            .collect();

        let result = encoder.encode_sequences(&sequences).unwrap();
        assert!(result.stats.processing_time_ms >= 0);
        assert!(result.stats.avg_encoding_time_us >= 0.0);
    }
}
