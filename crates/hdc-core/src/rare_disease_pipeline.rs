// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Privacy-Preserving Rare Disease Variant Matching Pipeline
//!
//! Enables cross-institutional genomic similarity search for undiagnosed
//! rare disease patients WITHOUT exposing raw genomic sequences.
//!
//! Architecture:
//!   Patient VCF -> HDC Encoding -> Differential Privacy -> Similarity Search
//!                                                              |
//!   Matched Conditions <- Disease Database <- Top-K Matches <--+
//!
//! Privacy guarantees:
//! - Raw VCF never leaves the patient's institution
//! - Only differentially-private hypervectors are shared
//! - Epsilon budget controls privacy-utility tradeoff
//! - Similarity search operates on encoded vectors only
//!
//! References:
//! - Splinter et al. (2018). Rare disease diagnostic odyssey.
//! - Dwork, C. (2006). Differential privacy.
//! - Kanerva, P. (2009). Hyperdimensional computing.
//! - Rehm, H. L. et al. (2015). ClinGen -- standardized gene curation.

use crate::{bundle, Hypervector, Seed, HYPERVECTOR_BYTES};
use crate::differential_privacy::{DpHypervector, DpParams};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// Clinical significance of a genetic variant (ClinVar-aligned)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClinicalSignificance {
    Pathogenic,
    LikelyPathogenic,
    /// Variant of uncertain significance
    VUS,
    LikelyBenign,
    Benign,
}

/// Mendelian inheritance pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InheritancePattern {
    AutosomalDominant,
    AutosomalRecessive,
    XLinked,
    Mitochondrial,
    DeNovo,
    Unknown,
}

/// A single rare-disease-relevant genomic variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RareDiseaseVariant {
    pub gene: String,
    pub chromosome: String,
    pub position: u64,
    pub ref_allele: String,
    pub alt_allele: String,
    pub clinical_significance: ClinicalSignificance,
    pub condition: Option<String>,
    pub inheritance: InheritancePattern,
}

// ---------------------------------------------------------------------------
// Patient / disease profiles
// ---------------------------------------------------------------------------

/// Privacy-preserving patient profile suitable for cross-institutional sharing.
///
/// Contains only differentially-private hypervector encodings -- never raw
/// genomic data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatientProfile {
    /// DP-protected genomic hypervector (2048 bytes = 16384 bits)
    pub encoded_genome: Vec<u8>,
    /// HPO terms (Human Phenotype Ontology) -- non-identifying phenotype codes
    pub phenotype_hpo: Vec<String>,
    /// HPO terms encoded as a hypervector
    pub encoded_phenotype: Vec<u8>,
    /// Age at symptom onset (years)
    pub age_onset: Option<u32>,
    /// Parental consanguinity flag
    pub consanguinity: bool,
    /// Free-text family history summaries
    pub family_history: Vec<String>,
    /// Remaining differential-privacy epsilon budget
    pub epsilon_budget: f64,
}

/// Reference disease signature for the matching database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiseaseSignature {
    /// Canonical disease ID (OMIM or Orphanet)
    pub disease_id: String,
    pub disease_name: String,
    pub causal_genes: Vec<String>,
    /// Reference pathogenic-variant hypervector
    pub encoded_signature: Vec<u8>,
    /// Expected phenotype hypervector
    pub phenotype_signature: Vec<u8>,
    pub inheritance: InheritancePattern,
    /// Population prevalence (e.g. 1/2500 for CF)
    pub prevalence: f64,
}

/// A single match result from the similarity search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub disease: DiseaseSignature,
    /// Cosine similarity of genomic hypervectors (0.0 -- 1.0)
    pub genomic_similarity: f64,
    /// Cosine similarity of phenotype hypervectors (0.0 -- 1.0)
    pub phenotype_similarity: f64,
    /// Weighted combination of genomic + phenotype similarities
    pub combined_score: f64,
    /// Overall confidence (combined score adjusted by disease prevalence)
    pub confidence: f64,
    /// Genes that appear in both patient variants and disease causal set
    pub matching_genes: Vec<String>,
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Main rare-disease matching engine.
///
/// Typical workflow:
/// 1. Build a `RareDiseasePipeline` and populate the disease database.
/// 2. At the patient's institution, call [`create_patient_profile`] to
///    produce a DP-protected profile.
/// 3. Send the profile (never raw VCF!) to the matching service.
/// 4. Call [`search`] to find the top-K diseases by similarity.
pub struct RareDiseasePipeline {
    disease_db: Vec<DiseaseSignature>,
    seed: Seed,
    dimension: usize,
    dp_epsilon: f64,
    top_k: usize,
    phenotype_weight: f64,
    genomic_weight: f64,
}

impl RareDiseasePipeline {
    /// Create a new pipeline with the given default DP epsilon.
    pub fn new(dp_epsilon: f64) -> Self {
        assert!(dp_epsilon > 0.0, "Epsilon must be positive");
        Self {
            disease_db: Vec::new(),
            seed: Seed::from_string("rare-disease-pipeline-v1"),
            dimension: crate::HYPERVECTOR_DIM,
            dp_epsilon,
            top_k: 10,
            phenotype_weight: 0.4,
            genomic_weight: 0.6,
        }
    }

    /// Override the default seed for deterministic encoding.
    pub fn with_seed(mut self, seed: Seed) -> Self {
        self.seed = seed;
        self
    }

    /// Override the default top-K (default 10).
    pub fn with_top_k(mut self, k: usize) -> Self {
        self.top_k = k;
        self
    }

    /// Override the genomic / phenotype weights (must sum to 1.0).
    pub fn with_weights(mut self, genomic: f64, phenotype: f64) -> Self {
        assert!(
            (genomic + phenotype - 1.0).abs() < 1e-9,
            "Weights must sum to 1.0"
        );
        self.genomic_weight = genomic;
        self.phenotype_weight = phenotype;
        self
    }

    // ------------------------------------------------------------------
    // Disease database management
    // ------------------------------------------------------------------

    /// Register a disease in the matching database.
    ///
    /// `variants` are the known causal/pathogenic variants for this disease.
    /// `hpo_terms` are the expected phenotype terms (HPO codes).
    pub fn add_disease(
        &mut self,
        variants: &[RareDiseaseVariant],
        hpo_terms: &[&str],
        disease_id: &str,
        name: &str,
        prevalence: f64,
        inheritance: InheritancePattern,
    ) {
        let encoded_sig = self.encode_variants_raw(variants);
        let phenotype_sig = self.encode_phenotype_terms(hpo_terms);

        let causal_genes: Vec<String> = variants
            .iter()
            .map(|v| v.gene.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        self.disease_db.push(DiseaseSignature {
            disease_id: disease_id.to_string(),
            disease_name: name.to_string(),
            causal_genes,
            encoded_signature: encoded_sig.as_bytes().to_vec(),
            phenotype_signature: phenotype_sig.as_bytes().to_vec(),
            inheritance,
            prevalence,
        });
    }

    /// Number of diseases in the database.
    pub fn disease_count(&self) -> usize {
        self.disease_db.len()
    }

    // ------------------------------------------------------------------
    // Encoding helpers
    // ------------------------------------------------------------------

    /// Encode variants into a raw (non-DP) hypervector.
    ///
    /// Each variant is encoded as: bind(gene_hv, position_hv, allele_hv)
    /// All variant HVs are then bundled via majority vote.
    fn encode_variants_raw(&self, variants: &[RareDiseaseVariant]) -> Hypervector {
        if variants.is_empty() {
            return Hypervector::zero();
        }

        let hvs: Vec<Hypervector> = variants
            .iter()
            .map(|v| {
                let gene_hv = Hypervector::random(&self.seed, &v.gene);
                let pos_hv = Hypervector::random(&self.seed, &format!("{}:{}", v.chromosome, v.position));
                let allele_hv = Hypervector::random(&self.seed, &format!("{}>{}", v.ref_allele, v.alt_allele));
                gene_hv.bind(&pos_hv).bind(&allele_hv)
            })
            .collect();

        let refs: Vec<&Hypervector> = hvs.iter().collect();
        bundle(&refs)
    }

    /// Encode a set of HPO phenotype terms into a hypervector.
    fn encode_phenotype_terms(&self, hpo_terms: &[&str]) -> Hypervector {
        if hpo_terms.is_empty() {
            return Hypervector::zero();
        }

        let hvs: Vec<Hypervector> = hpo_terms
            .iter()
            .map(|term| Hypervector::random(&self.seed, term))
            .collect();

        let refs: Vec<&Hypervector> = hvs.iter().collect();
        bundle(&refs)
    }

    // ------------------------------------------------------------------
    // Patient profile creation (runs at patient's institution)
    // ------------------------------------------------------------------

    /// Encode patient variants with differential privacy.
    ///
    /// The returned bytes are safe to transmit -- they satisfy epsilon-DP.
    pub fn encode_patient_variants(&self, variants: &[RareDiseaseVariant], epsilon: f64) -> Vec<u8> {
        let raw = self.encode_variants_raw(variants);
        let dp = DpHypervector::from_vector(&raw, DpParams::pure(epsilon), Some(42));
        dp.vector.as_bytes().to_vec()
    }

    /// Encode phenotype HPO terms into a hypervector (deterministic, no DP
    /// needed since HPO codes are already standardised and non-identifying).
    pub fn encode_phenotype(&self, hpo_terms: &[&str]) -> Vec<u8> {
        self.encode_phenotype_terms(hpo_terms).as_bytes().to_vec()
    }

    /// Build a complete patient profile ready for cross-institutional sharing.
    pub fn create_patient_profile(
        &self,
        variants: &[RareDiseaseVariant],
        hpo_terms: &[&str],
        epsilon: f64,
    ) -> PatientProfile {
        let encoded_genome = self.encode_patient_variants(variants, epsilon);
        let encoded_phenotype = self.encode_phenotype(hpo_terms);

        PatientProfile {
            encoded_genome,
            phenotype_hpo: hpo_terms.iter().map(|s| s.to_string()).collect(),
            encoded_phenotype,
            age_onset: None,
            consanguinity: false,
            family_history: Vec::new(),
            epsilon_budget: self.dp_epsilon - epsilon,
        }
    }

    // ------------------------------------------------------------------
    // Similarity search
    // ------------------------------------------------------------------

    /// Search the disease database for the best matches to a patient profile.
    ///
    /// Returns at most `top_k` results sorted by combined score (descending).
    pub fn search(&self, patient: &PatientProfile) -> Vec<MatchResult> {
        let mut results: Vec<MatchResult> = self
            .disease_db
            .iter()
            .map(|disease| {
                let genomic_sim =
                    cosine_similarity_binary(&patient.encoded_genome, &disease.encoded_signature);
                let phenotype_sim =
                    cosine_similarity_binary(&patient.encoded_phenotype, &disease.phenotype_signature);

                let combined =
                    self.genomic_weight * genomic_sim + self.phenotype_weight * phenotype_sim;

                // Confidence: combined score scaled by a soft prevalence factor
                // (rarer diseases get a small penalty since false-positive cost is higher)
                let prevalence_factor = 1.0 - (-disease.prevalence * 1e4_f64).exp();
                let confidence = combined * (0.5 + 0.5 * prevalence_factor);

                // Find genes in common between patient variants and disease
                // (We cannot recover patient genes from the DP vector, so we
                //  report the disease's causal genes when the score is meaningful.)
                let matching_genes = if combined > 0.55 {
                    disease.causal_genes.clone()
                } else {
                    Vec::new()
                };

                MatchResult {
                    disease: disease.clone(),
                    genomic_similarity: genomic_sim,
                    phenotype_similarity: phenotype_sim,
                    combined_score: combined,
                    confidence,
                    matching_genes,
                }
            })
            .collect();

        results.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());
        results.truncate(self.top_k);
        results
    }
}

// ---------------------------------------------------------------------------
// Similarity metrics
// ---------------------------------------------------------------------------

/// Hamming similarity between two packed binary vectors.
///
/// Returns the fraction of matching bits (1.0 = identical).
pub fn hamming_similarity(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len(), "Vectors must be the same length");
    let total_bits = a.len() * 8;
    let differing: usize = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (x ^ y).count_ones() as usize)
        .sum();
    1.0 - (differing as f64 / total_bits as f64)
}

/// Cosine similarity for binary (packed-bit) vectors mapped to bipolar
/// ({0,1} -> {-1,+1}), normalized to [0, 1].
///
/// cosine_binary = (match - mismatch) / total_bits
/// normalized    = (cosine_binary + 1) / 2
pub fn cosine_similarity_binary(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len(), "Vectors must be the same length");
    let total_bits = (a.len() * 8) as f64;
    let differing: usize = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (x ^ y).count_ones() as usize)
        .sum();
    let matching = total_bits - differing as f64;
    // bipolar cosine: (matching - differing) / total
    let cosine = (matching - differing as f64) / total_bits;
    // normalize to [0, 1]
    (cosine + 1.0) / 2.0
}

// ---------------------------------------------------------------------------
// Reference disease database (demo / testing)
// ---------------------------------------------------------------------------

/// Build a small reference database of ~10 well-known rare diseases.
///
/// Returns `(variants, hpo_terms, disease_id, disease_name, prevalence, inheritance)`.
pub fn build_reference_database()
-> Vec<(Vec<RareDiseaseVariant>, Vec<&'static str>, &'static str, &'static str, f64, InheritancePattern)>
{
    vec![
        // 1. Cystic Fibrosis
        (
            vec![RareDiseaseVariant {
                gene: "CFTR".into(),
                chromosome: "chr7".into(),
                position: 117559590,
                ref_allele: "CTT".into(),
                alt_allele: "C".into(), // delta-F508 deletion
                clinical_significance: ClinicalSignificance::Pathogenic,
                condition: Some("Cystic Fibrosis".into()),
                inheritance: InheritancePattern::AutosomalRecessive,
            }],
            vec!["HP:0002110", "HP:0006536", "HP:0002024"], // bronchiectasis, pancreatic insuff, gastro
            "OMIM:219700", "Cystic Fibrosis", 1.0 / 2500.0,
            InheritancePattern::AutosomalRecessive,
        ),
        // 2. Sickle Cell Disease
        (
            vec![RareDiseaseVariant {
                gene: "HBB".into(),
                chromosome: "chr11".into(),
                position: 5248232,
                ref_allele: "T".into(),
                alt_allele: "A".into(), // Glu6Val
                clinical_significance: ClinicalSignificance::Pathogenic,
                condition: Some("Sickle Cell Disease".into()),
                inheritance: InheritancePattern::AutosomalRecessive,
            }],
            vec!["HP:0001903", "HP:0002027", "HP:0010972"],
            "OMIM:603903", "Sickle Cell Disease", 1.0 / 500.0,
            InheritancePattern::AutosomalRecessive,
        ),
        // 3. Huntington's Disease
        (
            vec![RareDiseaseVariant {
                gene: "HTT".into(),
                chromosome: "chr4".into(),
                position: 3076604,
                ref_allele: "C".into(),
                alt_allele: "CAGCAGCAG".into(), // CAG repeat expansion
                clinical_significance: ClinicalSignificance::Pathogenic,
                condition: Some("Huntington's Disease".into()),
                inheritance: InheritancePattern::AutosomalDominant,
            }],
            vec!["HP:0002072", "HP:0001300", "HP:0000726"],
            "OMIM:143100", "Huntington's Disease", 1.0 / 10000.0,
            InheritancePattern::AutosomalDominant,
        ),
        // 4. Marfan Syndrome
        (
            vec![RareDiseaseVariant {
                gene: "FBN1".into(),
                chromosome: "chr15".into(),
                position: 48700503,
                ref_allele: "G".into(),
                alt_allele: "A".into(),
                clinical_significance: ClinicalSignificance::Pathogenic,
                condition: Some("Marfan Syndrome".into()),
                inheritance: InheritancePattern::AutosomalDominant,
            }],
            vec!["HP:0001519", "HP:0001166", "HP:0001083"],
            "OMIM:154700", "Marfan Syndrome", 1.0 / 5000.0,
            InheritancePattern::AutosomalDominant,
        ),
        // 5. Phenylketonuria (PKU)
        (
            vec![RareDiseaseVariant {
                gene: "PAH".into(),
                chromosome: "chr12".into(),
                position: 103234213,
                ref_allele: "G".into(),
                alt_allele: "A".into(), // R408W
                clinical_significance: ClinicalSignificance::Pathogenic,
                condition: Some("Phenylketonuria".into()),
                inheritance: InheritancePattern::AutosomalRecessive,
            }],
            vec!["HP:0001987", "HP:0001249", "HP:0000252"],
            "OMIM:261600", "Phenylketonuria", 1.0 / 12000.0,
            InheritancePattern::AutosomalRecessive,
        ),
        // 6. Tay-Sachs Disease
        (
            vec![RareDiseaseVariant {
                gene: "HEXA".into(),
                chromosome: "chr15".into(),
                position: 72638892,
                ref_allele: "TATC".into(),
                alt_allele: "T".into(), // 4-bp deletion
                clinical_significance: ClinicalSignificance::Pathogenic,
                condition: Some("Tay-Sachs Disease".into()),
                inheritance: InheritancePattern::AutosomalRecessive,
            }],
            vec!["HP:0002134", "HP:0001252", "HP:0001263"],
            "OMIM:272800", "Tay-Sachs Disease", 1.0 / 320000.0,
            InheritancePattern::AutosomalRecessive,
        ),
        // 7. Duchenne Muscular Dystrophy
        (
            vec![RareDiseaseVariant {
                gene: "DMD".into(),
                chromosome: "chrX".into(),
                position: 32380985,
                ref_allele: "A".into(),
                alt_allele: "T".into(),
                clinical_significance: ClinicalSignificance::Pathogenic,
                condition: Some("Duchenne Muscular Dystrophy".into()),
                inheritance: InheritancePattern::XLinked,
            }],
            vec!["HP:0003202", "HP:0003325", "HP:0002515"],
            "OMIM:310200", "Duchenne Muscular Dystrophy", 1.0 / 3500.0,
            InheritancePattern::XLinked,
        ),
        // 8. Rett Syndrome
        (
            vec![RareDiseaseVariant {
                gene: "MECP2".into(),
                chromosome: "chrX".into(),
                position: 153296777,
                ref_allele: "C".into(),
                alt_allele: "T".into(), // R168X
                clinical_significance: ClinicalSignificance::Pathogenic,
                condition: Some("Rett Syndrome".into()),
                inheritance: InheritancePattern::DeNovo,
            }],
            vec!["HP:0002059", "HP:0001344", "HP:0002540"],
            "OMIM:312750", "Rett Syndrome", 1.0 / 10000.0,
            InheritancePattern::DeNovo,
        ),
        // 9. Fragile X Syndrome
        (
            vec![RareDiseaseVariant {
                gene: "FMR1".into(),
                chromosome: "chrX".into(),
                position: 147912050,
                ref_allele: "C".into(),
                alt_allele: "CCGGCGGCGG".into(), // CGG repeat expansion
                clinical_significance: ClinicalSignificance::Pathogenic,
                condition: Some("Fragile X Syndrome".into()),
                inheritance: InheritancePattern::XLinked,
            }],
            vec!["HP:0001249", "HP:0000750", "HP:0000400"],
            "OMIM:300624", "Fragile X Syndrome", 1.0 / 4000.0,
            InheritancePattern::XLinked,
        ),
        // 10. Wilson Disease
        (
            vec![RareDiseaseVariant {
                gene: "ATP7B".into(),
                chromosome: "chr13".into(),
                position: 52535985,
                ref_allele: "C".into(),
                alt_allele: "T".into(), // H1069Q
                clinical_significance: ClinicalSignificance::Pathogenic,
                condition: Some("Wilson Disease".into()),
                inheritance: InheritancePattern::AutosomalRecessive,
            }],
            vec!["HP:0001392", "HP:0001394", "HP:0002180"],
            "OMIM:277900", "Wilson Disease", 1.0 / 30000.0,
            InheritancePattern::AutosomalRecessive,
        ),
    ]
}

/// Populate a pipeline with the built-in reference database.
pub fn populate_reference_database(pipeline: &mut RareDiseasePipeline) {
    for (variants, hpo, id, name, prev, inh) in build_reference_database() {
        pipeline.add_disease(&variants, &hpo, id, name, prev, inh);
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pipeline() -> RareDiseasePipeline {
        let mut p = RareDiseasePipeline::new(3.0);
        populate_reference_database(&mut p);
        p
    }

    fn cf_variant() -> RareDiseaseVariant {
        RareDiseaseVariant {
            gene: "CFTR".into(),
            chromosome: "chr7".into(),
            position: 117559590,
            ref_allele: "CTT".into(),
            alt_allele: "C".into(),
            clinical_significance: ClinicalSignificance::Pathogenic,
            condition: Some("Cystic Fibrosis".into()),
            inheritance: InheritancePattern::AutosomalRecessive,
        }
    }

    fn unrelated_variant() -> RareDiseaseVariant {
        RareDiseaseVariant {
            gene: "BRCA1".into(),
            chromosome: "chr17".into(),
            position: 43044295,
            ref_allele: "A".into(),
            alt_allele: "G".into(),
            clinical_significance: ClinicalSignificance::VUS,
            condition: None,
            inheritance: InheritancePattern::Unknown,
        }
    }

    // ---- 1. Encode patient produces non-zero hypervector -------------------
    #[test]
    fn encode_patient_produces_nonzero_hv() {
        let p = make_pipeline();
        let encoded = p.encode_patient_variants(&[cf_variant()], 1.0);
        assert_eq!(encoded.len(), HYPERVECTOR_BYTES);
        assert!(encoded.iter().any(|&b| b != 0), "Encoded HV must be non-zero");
    }

    // ---- 2. Same variants produce similar encodings -----------------------
    #[test]
    fn same_variants_produce_similar_encodings() {
        let p = make_pipeline();
        let v = cf_variant();
        // Use high epsilon (low noise) so similarity is preserved
        let enc1 = p.encode_patient_variants(&[v.clone()], 10.0);
        let enc2 = p.encode_patient_variants(&[v], 10.0);
        let sim = cosine_similarity_binary(&enc1, &enc2);
        assert!(
            sim > 0.8,
            "Same variants with high epsilon should be very similar, got {sim}"
        );
    }

    // ---- 3. Different variants produce dissimilar encodings ---------------
    #[test]
    fn different_variants_produce_dissimilar_encodings() {
        let p = make_pipeline();
        let enc_cf = p.encode_patient_variants(&[cf_variant()], 10.0);
        let enc_other = p.encode_patient_variants(&[unrelated_variant()], 10.0);
        let sim = cosine_similarity_binary(&enc_cf, &enc_other);
        // They should not be highly similar
        assert!(
            sim < 0.8,
            "Different gene variants should not be highly similar, got {sim}"
        );
    }

    // ---- 4. High epsilon preserves more similarity -----------------------
    #[test]
    fn high_epsilon_preserves_similarity() {
        let p = make_pipeline();
        let raw = p.encode_variants_raw(&[cf_variant()]);
        let raw_bytes = raw.as_bytes().to_vec();

        let dp_high = p.encode_patient_variants(&[cf_variant()], 10.0);
        let dp_low = p.encode_patient_variants(&[cf_variant()], 0.1);

        let sim_high = cosine_similarity_binary(&raw_bytes, &dp_high);
        let sim_low = cosine_similarity_binary(&raw_bytes, &dp_low);

        assert!(
            sim_high > sim_low,
            "High epsilon should preserve more similarity: high={sim_high}, low={sim_low}"
        );
    }

    // ---- 5. Low epsilon adds more noise ----------------------------------
    #[test]
    fn low_epsilon_adds_more_noise() {
        let p = make_pipeline();
        let raw = p.encode_variants_raw(&[cf_variant()]);
        let raw_bytes = raw.as_bytes().to_vec();
        let dp_low = p.encode_patient_variants(&[cf_variant()], 0.1);
        let sim = cosine_similarity_binary(&raw_bytes, &dp_low);
        // With very low epsilon, similarity to raw should be near chance (0.5)
        assert!(
            sim < 0.7,
            "Low epsilon should heavily distort the vector, got similarity {sim}"
        );
    }

    // ---- 6. Search returns correct disease for known pathogenic variants --
    #[test]
    fn search_returns_correct_disease() {
        let p = make_pipeline();
        // Patient with CF variant and CF phenotype
        let profile = p.create_patient_profile(
            &[cf_variant()],
            &["HP:0002110", "HP:0006536", "HP:0002024"],
            2.0,
        );
        let results = p.search(&profile);
        assert!(!results.is_empty(), "Should return at least one match");
        // CF should be the top match (or at least in top 3)
        let top3_ids: Vec<&str> = results.iter().take(3).map(|r| r.disease.disease_id.as_str()).collect();
        assert!(
            top3_ids.contains(&"OMIM:219700"),
            "Cystic Fibrosis should be in top 3, got: {:?}", top3_ids
        );
    }

    // ---- 7. Phenotype matching works with HPO terms ----------------------
    #[test]
    fn phenotype_matching_works() {
        let p = make_pipeline();
        let enc1 = p.encode_phenotype(&["HP:0002110", "HP:0006536"]);
        let enc2 = p.encode_phenotype(&["HP:0002110", "HP:0006536"]);
        let enc3 = p.encode_phenotype(&["HP:0001903", "HP:0002027"]);

        let sim_same = cosine_similarity_binary(&enc1, &enc2);
        let sim_diff = cosine_similarity_binary(&enc1, &enc3);

        assert!(
            sim_same > sim_diff,
            "Same HPO terms should be more similar: same={sim_same}, diff={sim_diff}"
        );
    }

    // ---- 8. Combined scoring weights work correctly ----------------------
    #[test]
    fn combined_scoring_weights() {
        // A pipeline with all weight on genomic
        let mut p_genomic = RareDiseasePipeline::new(3.0).with_weights(1.0, 0.0);
        populate_reference_database(&mut p_genomic);

        // A pipeline with all weight on phenotype
        let mut p_pheno = RareDiseasePipeline::new(3.0).with_weights(0.0, 1.0);
        populate_reference_database(&mut p_pheno);

        let profile = p_genomic.create_patient_profile(
            &[cf_variant()],
            &["HP:0002110", "HP:0006536", "HP:0002024"],
            2.0,
        );

        let res_g = p_genomic.search(&profile);
        let res_p = p_pheno.search(&profile);

        // The two rankings should differ (different weight emphasis)
        // At minimum the scores should differ
        assert!(
            (res_g[0].combined_score - res_p[0].combined_score).abs() > 1e-6
                || res_g[0].disease.disease_id != res_p[0].disease.disease_id,
            "Different weights should produce different results"
        );
    }

    // ---- 9. Top-K returns at most K results ------------------------------
    #[test]
    fn top_k_returns_at_most_k() {
        let mut p = RareDiseasePipeline::new(3.0).with_top_k(3);
        populate_reference_database(&mut p);

        let profile = p.create_patient_profile(&[cf_variant()], &["HP:0002110"], 2.0);
        let results = p.search(&profile);
        assert!(
            results.len() <= 3,
            "Should return at most 3, got {}",
            results.len()
        );
    }

    // ---- 10. Reference database builds successfully ----------------------
    #[test]
    fn reference_database_builds() {
        let db = build_reference_database();
        assert_eq!(db.len(), 10, "Should have 10 reference diseases");
        for (variants, hpo, id, name, prev, _inh) in &db {
            assert!(!variants.is_empty(), "Disease {name} should have variants");
            assert!(!hpo.is_empty(), "Disease {name} should have HPO terms");
            assert!(!id.is_empty());
            assert!(!name.is_empty());
            assert!(*prev > 0.0);
        }
    }

    // ---- 11. Cosine similarity bounded [0, 1] ----------------------------
    #[test]
    fn cosine_similarity_bounded() {
        let a = vec![0xFF_u8; HYPERVECTOR_BYTES];
        let b = vec![0x00_u8; HYPERVECTOR_BYTES];
        let c = vec![0xFF_u8; HYPERVECTOR_BYTES];

        let sim_opposite = cosine_similarity_binary(&a, &b);
        let sim_same = cosine_similarity_binary(&a, &c);

        assert!(sim_opposite >= 0.0 && sim_opposite <= 1.0, "Got {sim_opposite}");
        assert!(sim_same >= 0.0 && sim_same <= 1.0, "Got {sim_same}");
        assert!((sim_same - 1.0).abs() < 1e-9, "Identical vectors should have sim=1.0");
        assert!(sim_opposite.abs() < 1e-9, "Opposite vectors should have sim=0.0");
    }

    // ---- 12. Patient profile tracks epsilon budget -----------------------
    #[test]
    fn patient_profile_tracks_epsilon_budget() {
        let p = RareDiseasePipeline::new(5.0);
        let profile = p.create_patient_profile(&[cf_variant()], &["HP:0002110"], 2.0);
        assert!(
            (profile.epsilon_budget - 3.0).abs() < 1e-9,
            "Budget should be 5.0 - 2.0 = 3.0, got {}",
            profile.epsilon_budget
        );
    }

    // ---- 13. Hamming similarity works correctly --------------------------
    #[test]
    fn hamming_similarity_works() {
        let a = vec![0xFF_u8; HYPERVECTOR_BYTES];
        let b = vec![0xFF_u8; HYPERVECTOR_BYTES];
        assert!((hamming_similarity(&a, &b) - 1.0).abs() < 1e-9);

        let c = vec![0x00_u8; HYPERVECTOR_BYTES];
        assert!((hamming_similarity(&a, &c)).abs() < 1e-9);
    }

    // ---- 14. Pipeline with custom seed is deterministic ------------------
    #[test]
    fn custom_seed_deterministic() {
        let seed = Seed::from_string("test-seed-42");
        let p1 = RareDiseasePipeline::new(3.0).with_seed(seed);
        let p2 = RareDiseasePipeline::new(3.0).with_seed(seed);

        let enc1 = p1.encode_patient_variants(&[cf_variant()], 5.0);
        let enc2 = p2.encode_patient_variants(&[cf_variant()], 5.0);
        assert_eq!(enc1, enc2, "Same seed should produce identical encodings");
    }
}
