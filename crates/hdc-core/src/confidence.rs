//! Confidence Metrics for HDC Similarity
//!
//! Provides confidence scores for similarity matches to support
//! clinical decision-making.

use crate::{Hypervector, HYPERVECTOR_DIM};
use serde::{Deserialize, Serialize};

/// Match confidence level for clinical use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchConfidence {
    /// Very high confidence (>95% likely correct)
    VeryHigh,
    /// High confidence (85-95% likely correct)
    High,
    /// Moderate confidence (70-85% likely correct)
    Moderate,
    /// Low confidence (50-70% likely correct)
    Low,
    /// Very low confidence (<50% likely correct)
    VeryLow,
}

impl MatchConfidence {
    /// Get confidence from similarity score using empirical thresholds
    pub fn from_similarity(similarity: f64) -> Self {
        // Thresholds based on HDC properties:
        // - Random vectors have ~0.5 similarity
        // - Related sequences have 0.55-0.7 similarity
        // - Same-origin sequences have >0.7 similarity
        if similarity >= 0.85 {
            MatchConfidence::VeryHigh
        } else if similarity >= 0.70 {
            MatchConfidence::High
        } else if similarity >= 0.58 {
            MatchConfidence::Moderate
        } else if similarity >= 0.52 {
            MatchConfidence::Low
        } else {
            MatchConfidence::VeryLow
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            MatchConfidence::VeryHigh => "Very high confidence - strong match",
            MatchConfidence::High => "High confidence - likely match",
            MatchConfidence::Moderate => "Moderate confidence - possible match",
            MatchConfidence::Low => "Low confidence - weak match",
            MatchConfidence::VeryLow => "Very low confidence - likely unrelated",
        }
    }

    /// Get numeric probability estimate
    pub fn probability(&self) -> f64 {
        match self {
            MatchConfidence::VeryHigh => 0.97,
            MatchConfidence::High => 0.90,
            MatchConfidence::Moderate => 0.77,
            MatchConfidence::Low => 0.60,
            MatchConfidence::VeryLow => 0.35,
        }
    }
}

/// Similarity score with confidence metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityWithConfidence {
    /// Raw similarity score (0-1)
    pub similarity: f64,
    /// Confidence level
    pub confidence: MatchConfidence,
    /// Z-score relative to random baseline (~0.5)
    pub z_score: f64,
    /// Bits different from expected random match
    pub bits_above_random: i32,
    /// Statistical significance (p-value estimate)
    pub p_value: f64,
}

impl SimilarityWithConfidence {
    /// Calculate comprehensive confidence metrics for a similarity score
    pub fn calculate(similarity: f64) -> Self {
        // Expected random similarity for binary vectors
        let random_mean = 0.5;
        // Standard deviation for random binary vectors
        // SD = sqrt(p(1-p)/n) where p=0.5, n=10000
        let random_std = (0.5 * 0.5 / HYPERVECTOR_DIM as f64).sqrt();

        // Z-score: how many standard deviations above random
        let z_score = (similarity - random_mean) / random_std;

        // Bits above random
        let expected_matching = (HYPERVECTOR_DIM as f64 * random_mean) as i32;
        let actual_matching = (HYPERVECTOR_DIM as f64 * similarity) as i32;
        let bits_above_random = actual_matching - expected_matching;

        // P-value estimate using normal approximation
        // P(X > observed) for standard normal
        let p_value = Self::normal_cdf_complement(z_score);

        let confidence = MatchConfidence::from_similarity(similarity);

        SimilarityWithConfidence {
            similarity,
            confidence,
            z_score,
            bits_above_random,
            p_value,
        }
    }

    /// Compare two hypervectors with confidence
    pub fn compare(a: &Hypervector, b: &Hypervector) -> Self {
        let similarity = a.normalized_cosine_similarity(b);
        Self::calculate(similarity)
    }

    /// Check if this match is statistically significant
    pub fn is_significant(&self, alpha: f64) -> bool {
        self.p_value < alpha
    }

    /// Check if suitable for clinical use
    pub fn is_clinical_grade(&self) -> bool {
        matches!(
            self.confidence,
            MatchConfidence::VeryHigh | MatchConfidence::High
        )
    }

    // Standard normal CDF complement (1 - CDF)
    fn normal_cdf_complement(z: f64) -> f64 {
        if z > 8.0 {
            return 0.0;
        }
        if z < -8.0 {
            return 1.0;
        }

        // Approximation using error function
        let t = 1.0 / (1.0 + 0.2316419 * z.abs());
        let d = 0.3989423 * (-z * z / 2.0).exp();
        let p = d * t * (0.3193815 + t * (-0.3565638 + t * (1.781478 + t * (-1.821256 + t * 1.330274))));

        if z > 0.0 { p } else { 1.0 - p }
    }
}

/// Batch similarity results with statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSimilarityStats {
    /// All comparisons
    pub comparisons: Vec<SimilarityWithConfidence>,
    /// Mean similarity
    pub mean: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Maximum similarity
    pub max: f64,
    /// Minimum similarity
    pub min: f64,
    /// Number of high-confidence matches
    pub high_confidence_count: usize,
    /// Number of clinical-grade matches
    pub clinical_grade_count: usize,
}

impl BatchSimilarityStats {
    /// Calculate statistics for a batch of similarities
    pub fn from_similarities(similarities: &[f64]) -> Self {
        let comparisons: Vec<_> = similarities
            .iter()
            .map(|&s| SimilarityWithConfidence::calculate(s))
            .collect();

        let mean = similarities.iter().sum::<f64>() / similarities.len() as f64;

        let variance = similarities.iter()
            .map(|s| (s - mean).powi(2))
            .sum::<f64>() / similarities.len() as f64;
        let std_dev = variance.sqrt();

        let max = similarities.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = similarities.iter().cloned().fold(f64::INFINITY, f64::min);

        let high_confidence_count = comparisons.iter()
            .filter(|c| matches!(c.confidence, MatchConfidence::High | MatchConfidence::VeryHigh))
            .count();

        let clinical_grade_count = comparisons.iter()
            .filter(|c| c.is_clinical_grade())
            .count();

        BatchSimilarityStats {
            comparisons,
            mean,
            std_dev,
            max,
            min,
            high_confidence_count,
            clinical_grade_count,
        }
    }

    /// Get top N matches sorted by similarity
    pub fn top_matches(&self, n: usize) -> Vec<&SimilarityWithConfidence> {
        let mut sorted: Vec<_> = self.comparisons.iter().collect();
        sorted.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        sorted.into_iter().take(n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Seed;

    #[test]
    fn test_confidence_from_similarity() {
        assert_eq!(MatchConfidence::from_similarity(0.90), MatchConfidence::VeryHigh);
        assert_eq!(MatchConfidence::from_similarity(0.75), MatchConfidence::High);
        assert_eq!(MatchConfidence::from_similarity(0.60), MatchConfidence::Moderate);
        assert_eq!(MatchConfidence::from_similarity(0.53), MatchConfidence::Low);
        assert_eq!(MatchConfidence::from_similarity(0.50), MatchConfidence::VeryLow);
    }

    #[test]
    fn test_similarity_with_confidence() {
        let result = SimilarityWithConfidence::calculate(0.75);

        assert_eq!(result.confidence, MatchConfidence::High);
        assert!(result.z_score > 40.0); // Well above random
        assert!(result.bits_above_random > 2000);
        assert!(result.p_value < 0.001);
        assert!(result.is_clinical_grade());
    }

    #[test]
    fn test_random_similarity() {
        let result = SimilarityWithConfidence::calculate(0.50);

        assert_eq!(result.confidence, MatchConfidence::VeryLow);
        assert!(result.z_score.abs() < 1.0);
        assert!(!result.is_significant(0.05));
    }

    #[test]
    fn test_compare_identical() {
        let seed = Seed::from_string("test");
        let hv = Hypervector::random(&seed, "item");

        let result = SimilarityWithConfidence::compare(&hv, &hv);

        assert_eq!(result.confidence, MatchConfidence::VeryHigh);
        assert!((result.similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_batch_stats() {
        let similarities = vec![0.50, 0.55, 0.60, 0.70, 0.80, 0.90];
        let stats = BatchSimilarityStats::from_similarities(&similarities);

        assert!(stats.mean > 0.65 && stats.mean < 0.70);
        // High confidence: 0.70, 0.80, 0.90 (all >= 0.70)
        assert_eq!(stats.high_confidence_count, 3);
        // Clinical grade: same as high confidence (High or VeryHigh)
        assert_eq!(stats.clinical_grade_count, 3);

        let top = stats.top_matches(2);
        assert_eq!(top.len(), 2);
        assert!(top[0].similarity > top[1].similarity);
    }
}
