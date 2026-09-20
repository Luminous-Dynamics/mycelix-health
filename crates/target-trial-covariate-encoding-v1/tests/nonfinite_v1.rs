use mycelix_target_trial_covariate_encoding_v1::*;

fn matrix(value: EncodedCovariateValueV1) -> EncodedBaselineCovariateMatrixV1 {
    EncodedBaselineCovariateMatrixV1 {
        schema_version: TARGET_TRIAL_COVARIATE_ENCODING_V1_VERSION,
        baseline_matrix_digest: [1; 32],
        protocol_digest: [2; 32],
        emulation_plan_digest: [3; 32],
        estimand_id: "primary".to_string(),
        analysis_manifest_digest: [4; 32],
        columns: vec![EncodedCovariateColumnV1 {
            confounder_id: "x".to_string(),
            measurement_policy_digest: [5; 32],
            encoding_policy_digest: [6; 32],
        }],
        rows: vec![EncodedCovariateRowV1 {
            subject_resource_type: "Patient".to_string(),
            subject_id: "patient-1".to_string(),
            strategy_id: "a".to_string(),
            contribution_digest: [7; 32],
            time_zero_receipt_digest: [8; 32],
            cells: vec![EncodedCovariateCellV1 {
                confounder_id: "x".to_string(),
                measurement_receipt_digest: [9; 32],
                selected_fact_snapshot_digest: Some([10; 32]),
                value,
            }],
        }],
    }
}

#[test]
fn nan_decimal_bits_are_rejected_before_identity() {
    let artifact = matrix(EncodedCovariateValueV1::DecimalBits(f64::NAN.to_bits()));
    assert!(matches!(
        encoded_baseline_covariate_matrix_digest_v1(&artifact),
        Err(CovariateEncodingV1Error::NonFiniteAnalysisValue)
    ));
}

#[test]
fn infinite_quantity_bits_are_rejected_before_identity() {
    let artifact = matrix(EncodedCovariateValueV1::QuantityUcumExact {
        value_bits: f64::INFINITY.to_bits(),
        unit_code: "kg".to_string(),
    });
    assert!(matches!(
        encoded_baseline_covariate_matrix_digest_v1(&artifact),
        Err(CovariateEncodingV1Error::NonFiniteAnalysisValue)
    ));
}

#[test]
fn empty_quantity_unit_is_rejected_before_identity() {
    let artifact = matrix(EncodedCovariateValueV1::QuantityUcumExact {
        value_bits: 75.0f64.to_bits(),
        unit_code: String::new(),
    });
    assert!(matches!(
        encoded_baseline_covariate_matrix_digest_v1(&artifact),
        Err(CovariateEncodingV1Error::InvalidEncodedMatrix)
    ));
}
