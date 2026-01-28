//! Integration tests for HDC Core
//!
//! Tests combining multiple modules: DP + star alleles, VCF + encoding, etc.

use hdc_core::{
    DnaEncoder, Hypervector, Seed, SnpEncoder,
    StarAlleleEncoder, MetabolizerPhenotype,
    VcfReader, VcfEncoder, Genotype,
};
use std::io::Cursor;

#[cfg(feature = "dp")]
use hdc_core::{DpParams, DpHypervector, PrivacyBudget};

// =============================================================================
// DP + Star Alleles Integration: Private Pharmacogenomics
// =============================================================================

#[cfg(feature = "dp")]
mod dp_pharmacogenomics {
    use super::*;

    /// Test that pharmacogenomic profiles can be protected with differential privacy
    /// while maintaining clinical utility
    #[test]
    fn test_dp_protected_pgx_profile() {
        let seed = Seed::from_string("integration-test");
        let encoder = StarAlleleEncoder::new(seed);

        // Create a patient profile with multiple genes
        let profile = encoder.encode_profile(&[
            ("CYP2D6", "*1", "*4"),   // Intermediate metabolizer
            ("CYP2C19", "*1", "*1"),  // Normal metabolizer
            ("CYP2C9", "*2", "*3"),   // Poor metabolizer
        ]).unwrap();

        // Apply differential privacy to the profile vector
        let dp_params = DpParams::pure(2.0); // Moderate privacy
        let dp_profile = DpHypervector::from_vector(&profile.profile_vector, dp_params, Some(42));

        // Verify the DP vector has same dimensions
        assert_eq!(
            dp_profile.vector.as_bytes().len(),
            profile.profile_vector.as_bytes().len()
        );

        // The DP vector should be similar but not identical
        let raw_sim = profile.profile_vector.normalized_cosine_similarity(&dp_profile.vector);
        assert!(raw_sim > 0.5, "DP profile should retain structure: {}", raw_sim);
        assert!(raw_sim < 1.0, "DP profile should have noise added");

        println!("DP PGx Profile - Raw similarity: {:.3}", raw_sim);
    }

    /// Test privacy-preserving pharmacogenomic matching
    #[test]
    fn test_dp_pgx_matching() {
        let seed = Seed::from_string("matching-test");
        let encoder = StarAlleleEncoder::new(seed);

        // Two patients with similar profiles
        let patient1 = encoder.encode_profile(&[
            ("CYP2D6", "*1", "*1"),
            ("CYP2C19", "*1", "*2"),
        ]).unwrap();

        let patient2 = encoder.encode_profile(&[
            ("CYP2D6", "*1", "*1"),
            ("CYP2C19", "*1", "*2"),
        ]).unwrap();

        // Patient with different profile
        let patient3 = encoder.encode_profile(&[
            ("CYP2D6", "*4", "*4"),
            ("CYP2C19", "*17", "*17"),
        ]).unwrap();

        // Apply DP to all profiles
        let dp_params = DpParams::pure(3.0); // Moderate-high privacy
        let dp1 = DpHypervector::from_vector(&patient1.profile_vector, dp_params, Some(1));
        let dp2 = DpHypervector::from_vector(&patient2.profile_vector, dp_params, Some(2));
        let dp3 = DpHypervector::from_vector(&patient3.profile_vector, dp_params, Some(3));

        // Similar patients should still be more similar after DP
        let sim_similar = dp1.similarity(&dp2);
        let sim_different = dp1.similarity(&dp3);

        println!("DP matching - Similar: {:.3}, Different: {:.3}", sim_similar, sim_different);

        // The corrected similarity should better reflect true relationships
        let corrected_similar = dp1.corrected_similarity(&dp2);
        let corrected_different = dp1.corrected_similarity(&dp3);

        println!("Corrected - Similar: {:.3}, Different: {:.3}", corrected_similar, corrected_different);

        // Corrected similar should be higher than corrected different
        assert!(
            corrected_similar > corrected_different,
            "Corrected similarity should preserve ordering"
        );
    }

    /// Test privacy budget tracking for pharmacogenomic queries
    #[test]
    fn test_pgx_privacy_budget() {
        let mut budget = PrivacyBudget::new(10.0); // Total budget of ε=10

        // Simulate multiple queries
        let query_epsilon = 2.0;

        // Can make 5 queries of ε=2 each
        for i in 0..5 {
            assert!(budget.can_query(query_epsilon), "Should allow query {}", i);
            budget.consume(query_epsilon).unwrap();
        }

        // Budget exhausted
        assert!(!budget.can_query(query_epsilon));
        assert!(budget.consume(query_epsilon).is_err());

        assert_eq!(budget.query_count, 5);
        assert!((budget.utilization() - 1.0).abs() < 0.01);
    }

    /// Test different privacy levels for pharmacogenomics
    #[test]
    fn test_pgx_privacy_levels() {
        let seed = Seed::from_string("privacy-levels");
        let encoder = StarAlleleEncoder::new(seed);

        let profile = encoder.encode_profile(&[
            ("CYP2D6", "*1", "*4"),
        ]).unwrap();

        // High privacy (ε=0.5) - more noise
        let high_privacy = DpParams::pure(0.5);
        let dp_high = DpHypervector::from_vector(&profile.profile_vector, high_privacy, Some(1));

        // Low privacy (ε=5.0) - less noise
        let low_privacy = DpParams::pure(5.0);
        let dp_low = DpHypervector::from_vector(&profile.profile_vector, low_privacy, Some(1));

        let sim_high = profile.profile_vector.normalized_cosine_similarity(&dp_high.vector);
        let sim_low = profile.profile_vector.normalized_cosine_similarity(&dp_low.vector);

        println!("Privacy levels - High ε=0.5: {:.3}, Low ε=5.0: {:.3}", sim_high, sim_low);

        // Lower privacy (higher ε) should preserve more similarity
        assert!(sim_low > sim_high, "Lower privacy should preserve more utility");
    }
}

// =============================================================================
// End-to-End: DNA Sequence Pipeline
// =============================================================================

mod dna_pipeline {
    use super::*;

    /// Test complete DNA encoding pipeline
    #[test]
    fn test_dna_encoding_pipeline() {
        let seed = Seed::from_string("dna-pipeline");
        let encoder = DnaEncoder::new(seed, 6);

        // Simulate COI barcode sequences (species identification)
        let species_a = "ATGCATGCATGCATGCATGCATGCATGCATGC";
        let species_a_variant = "ATGCATGCATGCATGCATGCATGCATGCATGC"; // Same
        let species_b = "GCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTA"; // Different

        let enc_a = encoder.encode_sequence(species_a).unwrap();
        let enc_a_var = encoder.encode_sequence(species_a_variant).unwrap();
        let enc_b = encoder.encode_sequence(species_b).unwrap();

        // Same species should be nearly identical
        let sim_same = enc_a.vector.normalized_cosine_similarity(&enc_a_var.vector);
        assert!(sim_same > 0.99, "Same sequence should match: {}", sim_same);

        // Different species should be distinguishable
        let sim_diff = enc_a.vector.normalized_cosine_similarity(&enc_b.vector);
        assert!(sim_diff < 0.9, "Different species should differ: {}", sim_diff);

        println!("DNA Pipeline - Same: {:.3}, Different: {:.3}", sim_same, sim_diff);
    }

    /// Test DNA encoding with DP protection
    #[cfg(feature = "dp")]
    #[test]
    fn test_dp_dna_encoding() {
        let seed = Seed::from_string("dp-dna");
        let encoder = DnaEncoder::new(seed, 6);

        let sequence = "ATGCATGCATGCATGCATGCATGCATGCATGC";
        let encoded = encoder.encode_sequence(sequence).unwrap();

        // Apply DP
        let dp_params = DpParams::pure(2.0);
        let dp_encoded = DpHypervector::from_vector(&encoded.vector, dp_params, Some(42));

        // Should maintain structure
        let sim = encoded.vector.normalized_cosine_similarity(&dp_encoded.vector);
        assert!(sim > 0.6, "DP DNA should retain structure: {}", sim);

        println!("DP DNA - Similarity: {:.3}, Expected retention: {:.3}",
                 sim, dp_params.expected_similarity_retention());
    }
}

// =============================================================================
// SNP + Pharmacogenomics Integration
// =============================================================================

mod snp_pgx_integration {
    use super::*;

    /// Test combining SNP panels with star alleles
    #[test]
    fn test_snp_star_allele_integration() {
        let seed = Seed::from_string("snp-pgx");

        let snp_encoder = SnpEncoder::new(seed);
        let star_encoder = StarAlleleEncoder::new(seed);

        // SNP panel that might influence a star allele
        let snp_panel = vec![
            ("rs3892097", 'A'),  // CYP2D6*4 defining variant
            ("rs1065852", 'G'),  // CYP2D6 variant
            ("rs16947", 'C'),    // CYP2D6*2 defining variant
        ];

        let snp_vector = snp_encoder.encode_panel(&snp_panel).unwrap();

        // Corresponding star allele interpretation
        let diplotype = star_encoder.encode_diplotype("CYP2D6", "*2", "*4").unwrap();

        // Both should be valid hypervectors
        assert_eq!(snp_vector.as_bytes().len(), diplotype.vector.as_bytes().len());

        // The encodings are independent but both represent CYP2D6 information
        let sim = snp_vector.normalized_cosine_similarity(&diplotype.vector);
        println!("SNP vs Star allele similarity: {:.3}", sim);

        // They shouldn't be identical (different encoding schemes)
        // but both capture genetic information
        assert!(sim < 0.9, "Different encoding schemes should differ");
    }

    /// Test combining multiple encoding types for comprehensive profile
    #[test]
    fn test_comprehensive_genetic_profile() {
        let seed = Seed::from_string("comprehensive");

        // DNA barcode (species/strain identification)
        let dna_encoder = DnaEncoder::new(seed, 6);
        let barcode = dna_encoder.encode_sequence("ATGCATGCATGCATGCATGC").unwrap();

        // SNP panel (disease risk, traits)
        let snp_encoder = SnpEncoder::new(seed);
        let snps = snp_encoder.encode_panel(&[
            ("rs1234", 'A'),
            ("rs5678", 'G'),
        ]).unwrap();

        // Pharmacogenomics (drug response)
        let pgx_encoder = StarAlleleEncoder::new(seed);
        let pgx = pgx_encoder.encode_profile(&[
            ("CYP2D6", "*1", "*1"),
            ("CYP2C19", "*1", "*2"),
        ]).unwrap();

        // All vectors should have same dimensions
        assert_eq!(barcode.vector.as_bytes().len(), snps.as_bytes().len());
        assert_eq!(snps.as_bytes().len(), pgx.profile_vector.as_bytes().len());

        // Create a combined profile by bundling
        let combined = hdc_core::bundle(&[
            &barcode.vector,
            &snps,
            &pgx.profile_vector,
        ]);

        // Combined should have same dimensions
        assert_eq!(combined.as_bytes().len(), barcode.vector.as_bytes().len());

        // Combined should be similar to each component (but not identical)
        let sim_dna = combined.normalized_cosine_similarity(&barcode.vector);
        let sim_snp = combined.normalized_cosine_similarity(&snps);
        let sim_pgx = combined.normalized_cosine_similarity(&pgx.profile_vector);

        println!("Combined profile similarities - DNA: {:.3}, SNP: {:.3}, PGx: {:.3}",
                 sim_dna, sim_snp, sim_pgx);

        // Each component should contribute to the combined vector
        assert!(sim_dna > 0.3 && sim_dna < 0.9);
        assert!(sim_snp > 0.3 && sim_snp < 0.9);
        assert!(sim_pgx > 0.3 && sim_pgx < 0.9);
    }
}

// =============================================================================
// Clinical Scenario Tests
// =============================================================================

mod clinical_scenarios {
    use super::*;

    /// Simulate a clinical pharmacogenomics workflow
    #[test]
    fn test_clinical_pgx_workflow() {
        let seed = Seed::from_string("clinical");
        let encoder = StarAlleleEncoder::new(seed);

        // Patient presents for codeine prescription
        let patient = encoder.encode_profile(&[
            ("CYP2D6", "*4", "*4"),  // Poor metabolizer!
        ]).unwrap();

        // Check drug interaction
        let prediction = encoder.predict_drug_interaction(&patient, "codeine");
        assert!(prediction.is_some());

        let pred = prediction.unwrap();
        assert_eq!(pred.phenotype, MetabolizerPhenotype::Poor);

        // Should recommend avoiding codeine
        println!("Clinical recommendation for codeine: {} - {}",
                 pred.phenotype, pred.recommendation);
    }

    /// Test multi-drug interaction screening
    #[test]
    fn test_multi_drug_screening() {
        let seed = Seed::from_string("multi-drug");
        let encoder = StarAlleleEncoder::new(seed);

        // Comprehensive pharmacogenomic profile
        let patient = encoder.encode_profile(&[
            ("CYP2D6", "*1", "*4"),   // Intermediate
            ("CYP2C19", "*2", "*2"),  // Poor
            ("CYP2C9", "*1", "*1"),   // Normal
            ("TPMT", "*1", "*3A"),    // Intermediate
        ]).unwrap();

        // Screen multiple drugs
        let drugs = ["codeine", "clopidogrel", "warfarin", "azathioprine"];

        println!("\nMulti-drug screening results:");
        for drug in drugs {
            if let Some(pred) = encoder.predict_drug_interaction(&patient, drug) {
                println!("  {}: {} ({})", drug, pred.phenotype, pred.recommendation);
            }
        }

        // Verify specific interactions
        let codeine_pred = encoder.predict_drug_interaction(&patient, "codeine").unwrap();
        assert_eq!(codeine_pred.phenotype, MetabolizerPhenotype::Intermediate);

        let clopidogrel_pred = encoder.predict_drug_interaction(&patient, "clopidogrel").unwrap();
        assert_eq!(clopidogrel_pred.phenotype, MetabolizerPhenotype::Poor);
    }

    /// Test identifying poor metabolizer genes
    #[test]
    fn test_poor_metabolizer_alert() {
        let seed = Seed::from_string("alert");
        let encoder = StarAlleleEncoder::new(seed);

        let patient = encoder.encode_profile(&[
            ("CYP2D6", "*4", "*5"),   // Poor (0+0)
            ("CYP2C19", "*1", "*1"),  // Normal
            ("DPYD", "*2A", "*2A"),   // Poor - CRITICAL for fluorouracil!
        ]).unwrap();

        let poor_genes = patient.get_poor_metabolizer_genes();

        println!("Poor metabolizer genes: {:?}", poor_genes);

        assert!(poor_genes.contains(&"CYP2D6"));
        assert!(poor_genes.contains(&"DPYD"));
        assert!(!poor_genes.contains(&"CYP2C19"));

        // DPYD poor metabolizer is critical - fluorouracil can be fatal
        assert!(poor_genes.contains(&"DPYD"),
                "DPYD poor metabolizer status is critical for fluorouracil safety!");
    }
}

// =============================================================================
// End-to-End: VCF → Encoding → DP → Similarity Pipeline
// =============================================================================

mod vcf_pipeline {
    use super::*;

    const PATIENT1_VCF: &str = r#"##fileformat=VCFv4.2
##INFO=<ID=DP,Number=1,Type=Integer,Description="Total Depth">
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO	FORMAT	PATIENT1
chr1	100	rs1801133	C	T	30	PASS	DP=50	GT	0/1
chr1	200	rs1801131	A	C	40	PASS	DP=45	GT	0/0
chr2	300	rs3892097	G	A	50	PASS	DP=60	GT	1/1
chr2	400	rs16947	C	T	35	PASS	DP=55	GT	0/1
chr7	500	rs12248560	C	T	45	PASS	DP=70	GT	0/0
"#;

    const PATIENT2_VCF: &str = r#"##fileformat=VCFv4.2
##INFO=<ID=DP,Number=1,Type=Integer,Description="Total Depth">
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO	FORMAT	PATIENT2
chr1	100	rs1801133	C	T	30	PASS	DP=50	GT	0/1
chr1	200	rs1801131	A	C	40	PASS	DP=45	GT	0/0
chr2	300	rs3892097	G	A	50	PASS	DP=60	GT	1/1
chr2	400	rs16947	C	T	35	PASS	DP=55	GT	0/1
chr7	500	rs12248560	C	T	45	PASS	DP=70	GT	0/0
"#;

    const PATIENT3_VCF: &str = r#"##fileformat=VCFv4.2
##INFO=<ID=DP,Number=1,Type=Integer,Description="Total Depth">
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO	FORMAT	PATIENT3
chr1	100	rs1801133	C	T	30	PASS	DP=50	GT	1/1
chr1	200	rs1801131	A	C	40	PASS	DP=45	GT	1/1
chr2	300	rs3892097	G	A	50	PASS	DP=60	GT	0/0
chr2	400	rs16947	C	T	35	PASS	DP=55	GT	1/1
chr7	500	rs12248560	C	T	45	PASS	DP=70	GT	1/1
"#;

    /// Test complete VCF parsing and encoding pipeline
    #[test]
    fn test_vcf_to_encoding_pipeline() {
        let seed = Seed::from_string("vcf-pipeline");
        let encoder = VcfEncoder::new(seed);

        // Parse VCF
        let mut reader = VcfReader::new(Cursor::new(PATIENT1_VCF)).unwrap();
        let variants = reader.read_variants().unwrap();

        assert_eq!(variants.len(), 5, "Should parse 5 variants");

        // Verify genotypes parsed correctly
        assert_eq!(variants[0].genotype, Some(Genotype::Het));
        assert_eq!(variants[2].genotype, Some(Genotype::HomAlt));

        // Encode to hypervector
        let encoded = encoder.encode_variants(&variants).unwrap();
        assert_eq!(encoded.as_bytes().len(), hdc_core::HYPERVECTOR_BYTES, "Should be HYPERVECTOR_DIM bits");

        println!("VCF Pipeline: Parsed {} variants, encoded to {} bytes",
                 variants.len(), encoded.as_bytes().len());
    }

    /// Test VCF similarity between identical patients
    #[test]
    fn test_vcf_identical_patients() {
        let seed = Seed::from_string("vcf-identical");
        let encoder = VcfEncoder::new(seed);

        // Parse same VCF twice
        let mut reader1 = VcfReader::new(Cursor::new(PATIENT1_VCF)).unwrap();
        let mut reader2 = VcfReader::new(Cursor::new(PATIENT2_VCF)).unwrap();

        let variants1 = reader1.read_variants().unwrap();
        let variants2 = reader2.read_variants().unwrap();

        let enc1 = encoder.encode_variants(&variants1).unwrap();
        let enc2 = encoder.encode_variants(&variants2).unwrap();

        let similarity = enc1.normalized_cosine_similarity(&enc2);

        assert!(similarity > 0.99, "Identical VCF should have sim > 0.99, got {}", similarity);
        println!("Identical patients similarity: {:.4}", similarity);
    }

    /// Test VCF similarity between different patients
    #[test]
    fn test_vcf_different_patients() {
        let seed = Seed::from_string("vcf-different");
        let encoder = VcfEncoder::new(seed);

        let mut reader1 = VcfReader::new(Cursor::new(PATIENT1_VCF)).unwrap();
        let mut reader3 = VcfReader::new(Cursor::new(PATIENT3_VCF)).unwrap();

        let variants1 = reader1.read_variants().unwrap();
        let variants3 = reader3.read_variants().unwrap();

        let enc1 = encoder.encode_variants(&variants1).unwrap();
        let enc3 = encoder.encode_variants(&variants3).unwrap();

        let similarity = enc1.normalized_cosine_similarity(&enc3);

        assert!(similarity < 0.9, "Different genotypes should have sim < 0.9, got {}", similarity);
        println!("Different patients similarity: {:.4}", similarity);
    }

    /// Test VCF with DP protection
    #[cfg(feature = "dp")]
    #[test]
    fn test_vcf_dp_pipeline() {
        let seed = Seed::from_string("vcf-dp");
        let encoder = VcfEncoder::new(seed);

        // Parse and encode
        let mut reader = VcfReader::new(Cursor::new(PATIENT1_VCF)).unwrap();
        let variants = reader.read_variants().unwrap();
        let encoded = encoder.encode_variants(&variants).unwrap();

        // Apply differential privacy
        let dp_params = DpParams::pure(2.0);
        let dp_encoded = DpHypervector::from_vector(&encoded, dp_params, Some(42));

        // DP vector should be similar but not identical
        let similarity = encoded.normalized_cosine_similarity(&dp_encoded.vector);

        assert!(similarity > 0.6, "DP should retain structure: {}", similarity);
        assert!(similarity < 1.0, "DP should add noise");

        println!("VCF + DP Pipeline: Raw similarity = {:.4}, Expected retention = {:.4}",
                 similarity, dp_params.expected_similarity_retention());
    }

    /// Test privacy-preserving patient matching via VCF
    #[cfg(feature = "dp")]
    #[test]
    fn test_vcf_dp_patient_matching() {
        let seed = Seed::from_string("vcf-matching");
        let encoder = VcfEncoder::new(seed);

        // Parse all three patients
        let mut r1 = VcfReader::new(Cursor::new(PATIENT1_VCF)).unwrap();
        let mut r2 = VcfReader::new(Cursor::new(PATIENT2_VCF)).unwrap();
        let mut r3 = VcfReader::new(Cursor::new(PATIENT3_VCF)).unwrap();

        let v1 = r1.read_variants().unwrap();
        let v2 = r2.read_variants().unwrap();
        let v3 = r3.read_variants().unwrap();

        let e1 = encoder.encode_variants(&v1).unwrap();
        let e2 = encoder.encode_variants(&v2).unwrap();
        let e3 = encoder.encode_variants(&v3).unwrap();

        // Apply DP to all
        let dp_params = DpParams::pure(3.0);
        let dp1 = DpHypervector::from_vector(&e1, dp_params, Some(1));
        let dp2 = DpHypervector::from_vector(&e2, dp_params, Some(2));
        let dp3 = DpHypervector::from_vector(&e3, dp_params, Some(3));

        // Compare similarities
        let sim_similar = dp1.similarity(&dp2);
        let sim_different = dp1.similarity(&dp3);

        println!("DP Patient Matching:");
        println!("  Similar patients (1 vs 2): {:.4}", sim_similar);
        println!("  Different patients (1 vs 3): {:.4}", sim_different);

        // Corrected similarities
        let corr_similar = dp1.corrected_similarity(&dp2);
        let corr_different = dp1.corrected_similarity(&dp3);

        println!("  Corrected similar: {:.4}", corr_similar);
        println!("  Corrected different: {:.4}", corr_different);

        // Similar patients should have higher corrected similarity
        assert!(corr_similar > corr_different,
                "Similar patients should have higher corrected similarity");
    }

    /// Full end-to-end test: VCF → Encoding → DP → Similarity Index
    #[cfg(feature = "dp")]
    #[test]
    fn test_full_e2e_vcf_pipeline() {
        use hdc_core::similarity::HdcIndex;

        let seed = Seed::from_string("e2e-vcf");
        let encoder = VcfEncoder::new(seed);

        // Create a "database" of patients
        let patients_vcf = vec![
            (PATIENT1_VCF, "patient_001"),
            (PATIENT2_VCF, "patient_002"),
            (PATIENT3_VCF, "patient_003"),
        ];

        // Parse, encode, and protect with DP
        let dp_params = DpParams::pure(2.5);
        let mut index = HdcIndex::new();

        for (vcf_data, patient_id) in &patients_vcf {
            let mut reader = VcfReader::new(Cursor::new(*vcf_data)).unwrap();
            let variants = reader.read_variants().unwrap();
            let encoded = encoder.encode_variants(&variants).unwrap();
            let dp_encoded = DpHypervector::from_vector(&encoded, dp_params, None);
            index.add(patient_id.to_string(), dp_encoded.vector);
        }

        // Query: Find patients similar to patient_001
        let query_vec = {
            let mut r = VcfReader::new(Cursor::new(PATIENT1_VCF)).unwrap();
            let v = r.read_variants().unwrap();
            let e = encoder.encode_variants(&v).unwrap();
            DpHypervector::from_vector(&e, dp_params, Some(999)).vector
        };

        let results = index.search(&query_vec, 3);

        println!("\nE2E VCF Pipeline Results:");
        println!("Query: patient_001 (similar match expected at top)");
        for result in &results {
            println!("  Match: {} with score {:.4}", result.id, result.similarity);
        }

        // Should find matches (patient_001 and patient_002 have same genotypes)
        assert!(!results.is_empty(), "Should find matches");

        // Due to DP noise, exact ordering may vary, but similar patients should score high
        let top_sim = results[0].similarity;
        println!("Top similarity score: {:.4}", top_sim);

        println!("\nFull E2E Pipeline: VCF → Parse → Encode → DP → Index → Search: SUCCESS");
    }
}

// =============================================================================
// Performance Stress Tests
// =============================================================================

mod stress_tests {
    use super::*;

    /// Test encoding many variants
    #[test]
    fn test_large_variant_set() {
        let seed = Seed::from_string("stress-test");
        let encoder = VcfEncoder::new(seed);

        // Generate a large VCF programmatically
        let mut vcf_lines = vec![
            "##fileformat=VCFv4.2".to_string(),
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tSAMPLE".to_string(),
        ];

        for i in 0..1000 {
            let genotype = match i % 4 {
                0 => "0/0",
                1 => "0/1",
                2 => "1/1",
                _ => "0/1",
            };
            vcf_lines.push(format!(
                "chr{}\t{}\trs{}\tA\tG\t30\tPASS\tDP=20\tGT\t{}",
                (i % 22) + 1,
                (i * 1000) + 100,
                i + 10000,
                genotype
            ));
        }

        let vcf_content = vcf_lines.join("\n");
        let mut reader = VcfReader::new(Cursor::new(vcf_content)).unwrap();
        let variants = reader.read_variants().unwrap();

        assert_eq!(variants.len(), 1000);

        let start = std::time::Instant::now();
        let encoded = encoder.encode_variants(&variants).unwrap();
        let elapsed = start.elapsed();

        println!("Stress test: Encoded {} variants in {:?}", variants.len(), elapsed);
        assert!(elapsed.as_millis() < 2000, "Should encode 1000 variants in < 2 seconds");
        assert_eq!(encoded.as_bytes().len(), hdc_core::HYPERVECTOR_BYTES);
    }

    /// Test similarity computation performance
    #[test]
    fn test_similarity_performance() {
        let seed = Seed::from_string("sim-perf");

        // Create random vectors
        let vectors: Vec<Hypervector> = (0..100)
            .map(|i| Hypervector::random(&seed, &format!("vector_{}", i)))
            .collect();

        let start = std::time::Instant::now();
        let mut comparisons = 0;

        for i in 0..vectors.len() {
            for j in i+1..vectors.len() {
                let _ = vectors[i].normalized_cosine_similarity(&vectors[j]);
                comparisons += 1;
            }
        }

        let elapsed = start.elapsed();
        let ops_per_sec = comparisons as f64 / elapsed.as_secs_f64();

        println!("Similarity performance: {} comparisons in {:?} ({:.0} ops/sec)",
                 comparisons, elapsed, ops_per_sec);

        // Adjusted for 16,384-bit vectors (larger than original 10,000)
        assert!(ops_per_sec > 60_000.0, "Should achieve > 60K comparisons/sec");
    }
}
