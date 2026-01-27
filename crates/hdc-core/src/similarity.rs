//! Similarity search and indexing
//!
//! Functions for efficient similarity search over collections of hypervectors.

use crate::Hypervector;

/// A similarity search index
pub struct HdcIndex {
    vectors: Vec<(String, Hypervector)>,
}

impl HdcIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        HdcIndex { vectors: Vec::new() }
    }

    /// Create an index with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        HdcIndex {
            vectors: Vec::with_capacity(capacity),
        }
    }

    /// Add a vector to the index
    pub fn add(&mut self, id: String, vector: Hypervector) {
        self.vectors.push((id, vector));
    }

    /// Number of vectors in the index
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Get a vector by index
    pub fn get(&self, index: usize) -> Option<(&str, &Hypervector)> {
        self.vectors.get(index).map(|(id, hv)| (id.as_str(), hv))
    }

    /// Find top-k most similar vectors to a query
    pub fn search(&self, query: &Hypervector, top_k: usize) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self.vectors
            .iter()
            .map(|(id, vector)| SearchResult {
                id: id.clone(),
                similarity: query.normalized_cosine_similarity(vector),
            })
            .collect();

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results.truncate(top_k);

        results
    }

    /// Find all vectors above a similarity threshold
    pub fn search_threshold(&self, query: &Hypervector, threshold: f64) -> Vec<SearchResult> {
        self.vectors
            .iter()
            .filter_map(|(id, vector)| {
                let sim = query.normalized_cosine_similarity(vector);
                if sim >= threshold {
                    Some(SearchResult {
                        id: id.clone(),
                        similarity: sim,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Compute the memory size of the index in bytes
    pub fn memory_size(&self) -> usize {
        self.vectors.iter()
            .map(|(id, hv)| id.len() + hv.as_bytes().len())
            .sum()
    }

    /// Get an iterator over all vectors
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Hypervector)> {
        self.vectors.iter().map(|(id, hv)| (id.as_str(), hv))
    }
}

impl Default for HdcIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a similarity search
#[derive(Clone, Debug)]
pub struct SearchResult {
    /// ID of the matched vector
    pub id: String,
    /// Similarity score (0.0-1.0)
    pub similarity: f64,
}

/// Statistics about similarity distributions
#[derive(Clone, Debug, Default)]
pub struct SimilarityStats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub std_dev: f64,
    pub count: usize,
}

impl SimilarityStats {
    /// Compute statistics from a slice of values
    pub fn from_values(values: &[f64]) -> Self {
        if values.is_empty() {
            return SimilarityStats::default();
        }

        let count = values.len();
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mean = values.iter().sum::<f64>() / count as f64;
        let variance = values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        SimilarityStats {
            min,
            max,
            mean,
            std_dev,
            count,
        }
    }
}

/// Compute pairwise similarity matrix (upper triangle)
pub fn pairwise_similarities(vectors: &[&Hypervector]) -> Vec<f64> {
    let n = vectors.len();
    let mut sims = Vec::with_capacity(n * (n - 1) / 2);

    for i in 0..n {
        for j in (i + 1)..n {
            sims.push(vectors[i].normalized_cosine_similarity(vectors[j]));
        }
    }

    sims
}

/// Compute k-NN accuracy
///
/// Given a query, ground truth labels, and an index with labels,
/// compute the fraction of top-k neighbors that share the same label.
pub fn knn_accuracy(
    query_label: &str,
    search_results: &[SearchResult],
    label_map: &std::collections::HashMap<String, String>,
) -> f64 {
    if search_results.is_empty() {
        return 0.0;
    }

    let correct = search_results.iter()
        .filter(|r| {
            label_map.get(&r.id)
                .map(|label| label == query_label)
                .unwrap_or(false)
        })
        .count();

    correct as f64 / search_results.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Seed;

    #[test]
    fn test_index_operations() {
        let seed = Seed::from_string("test");
        let mut index = HdcIndex::new();

        index.add("v1".to_string(), Hypervector::random(&seed, "item1"));
        index.add("v2".to_string(), Hypervector::random(&seed, "item2"));
        index.add("v3".to_string(), Hypervector::random(&seed, "item3"));

        assert_eq!(index.len(), 3);
        assert!(!index.is_empty());
    }

    #[test]
    fn test_search_self() {
        let seed = Seed::from_string("test");
        let mut index = HdcIndex::new();

        let hv1 = Hypervector::random(&seed, "item1");
        index.add("v1".to_string(), hv1.clone());
        index.add("v2".to_string(), Hypervector::random(&seed, "item2"));
        index.add("v3".to_string(), Hypervector::random(&seed, "item3"));

        let results = index.search(&hv1, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "v1");
        assert!(results[0].similarity > 0.99);
    }

    #[test]
    fn test_similarity_stats() {
        let values = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let stats = SimilarityStats::from_values(&values);

        assert!((stats.min - 0.1).abs() < 0.001);
        assert!((stats.max - 0.5).abs() < 0.001);
        assert!((stats.mean - 0.3).abs() < 0.001);
        assert_eq!(stats.count, 5);
    }
}
