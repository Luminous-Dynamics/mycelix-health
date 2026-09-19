#![deny(unsafe_code)]
//! Currentness-bound trust receipts for clinical distribution/OOD evidence.
//!
//! The v1 trust layer verifies structural OOD evidence against an admitted
//! evaluator and checks that the abstract admission was active at assessment and
//! trust-evaluation time. This v2 layer deliberately keeps those checks and adds
//! one more mandatory proof: short-lived runtime evaluator currentness composed
//! from both the bounded revocation observation and a positive root-issued lease.
//!
//! This crate still does not mint clinical presentation authority.

use mycelix_clinical_distribution_assessment::{
    ClinicalDistributionPolicyV1, ClinicalDistributionStatusV1,
    ValidatedClinicalDistributionAssessment,
};
use mycelix_clinical_distribution_currentness::{
    DistributionEvaluatorCurrentnessDigestV1, DistributionEvaluatorCurrentnessError,
    DistributionEvaluatorCurrentnessPolicyDigestV1, DistributionEvaluatorCurrentnessPolicyV1,
    VerifiedDistributionEvaluatorCurrentnessV1,
};
use mycelix_clinical_distribution_trust::{
    evaluate_distribution_trust, ClinicalDistributionTrustError, DistributionEvaluatorTrustPolicyV1,
    VerifiedDistributionEvaluatorAdmission,
};
use mycelix_clinical_evidence::ContentDigest;
use thiserror::Error;

const RECEIPT_DERIVE_KEY_CONTEXT: &str =
    "mycelix.health.clinical-distribution-trust-receipt-v2.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DistributionTrustReceiptDigestV2([u8; 32]);

impl DistributionTrustReceiptDigestV2 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Non-serializable receipt binding the existing evaluator-admission trust result
/// to one exact, still-live runtime currentness proof.
pub struct DistributionTrustReceiptV2 {
    assessment_digest: ContentDigest,
    distribution_policy_digest: ContentDigest,
    trust_policy_digest: ContentDigest,
    evaluator_admission_evidence_digest: ContentDigest,
    legacy_trust_receipt_digest: ContentDigest,
    currentness_policy_digest: DistributionEvaluatorCurrentnessPolicyDigestV1,
    currentness_digest: DistributionEvaluatorCurrentnessDigestV1,
    status: ClinicalDistributionStatusV1,
    assessed_at_micros: i64,
    trusted_at_micros: i64,
    currentness_valid_until_micros: i64,
    receipt_digest: DistributionTrustReceiptDigestV2,
}

impl DistributionTrustReceiptV2 {
    pub fn assessment_digest(&self) -> &ContentDigest {
        &self.assessment_digest
    }

    pub fn distribution_policy_digest(&self) -> &ContentDigest {
        &self.distribution_policy_digest
    }

    pub fn trust_policy_digest(&self) -> &ContentDigest {
        &self.trust_policy_digest
    }

    pub fn evaluator_admission_evidence_digest(&self) -> &ContentDigest {
        &self.evaluator_admission_evidence_digest
    }

    pub fn legacy_trust_receipt_digest(&self) -> &ContentDigest {
        &self.legacy_trust_receipt_digest
    }

    #[must_use]
    pub const fn currentness_policy_digest(
        &self,
    ) -> DistributionEvaluatorCurrentnessPolicyDigestV1 {
        self.currentness_policy_digest
    }

    #[must_use]
    pub const fn currentness_digest(&self) -> DistributionEvaluatorCurrentnessDigestV1 {
        self.currentness_digest
    }

    #[must_use]
    pub const fn status(&self) -> ClinicalDistributionStatusV1 {
        self.status
    }

    #[must_use]
    pub const fn assessed_at_micros(&self) -> i64 {
        self.assessed_at_micros
    }

    #[must_use]
    pub const fn trusted_at_micros(&self) -> i64 {
        self.trusted_at_micros
    }

    #[must_use]
    pub const fn currentness_valid_until_micros(&self) -> i64 {
        self.currentness_valid_until_micros
    }

    #[must_use]
    pub const fn receipt_digest(&self) -> DistributionTrustReceiptDigestV2 {
        self.receipt_digest
    }
}

/// Evaluate distribution trust while requiring a live positive-currentness proof.
///
/// v1 admission trust remains part of the proof rather than being weakened or
/// replaced: it still checks evaluator admission at the structural assessment
/// time and at trust evaluation time. v2 additionally requires the exact runtime
/// currentness proof to be active at `trusted_at_micros` and to agree on detector,
/// structural policy, trust policy, and external admission evidence.
pub fn evaluate_distribution_trust_v2(
    structural: &ValidatedClinicalDistributionAssessment,
    structural_policy: &ClinicalDistributionPolicyV1,
    trust_policy: &DistributionEvaluatorTrustPolicyV1,
    admission: &VerifiedDistributionEvaluatorAdmission,
    currentness_policy: &DistributionEvaluatorCurrentnessPolicyV1,
    currentness: &VerifiedDistributionEvaluatorCurrentnessV1,
    trusted_at_micros: i64,
) -> Result<DistributionTrustReceiptV2, ClinicalDistributionTrustV2Error> {
    let currentness_policy_digest = currentness_policy.digest()?;
    if currentness.policy_digest() != currentness_policy_digest {
        return Err(ClinicalDistributionTrustV2Error::CurrentnessPolicyMismatch);
    }
    if !currentness.active_at(trusted_at_micros) {
        return Err(ClinicalDistributionTrustV2Error::CurrentnessInactive);
    }
    if currentness.detector() != &trust_policy.required_detector {
        return Err(ClinicalDistributionTrustV2Error::CurrentnessDetectorMismatch);
    }

    let legacy = evaluate_distribution_trust(
        structural,
        structural_policy,
        trust_policy,
        admission,
        trusted_at_micros,
    )?;

    if currentness.distribution_policy_digest() != legacy.distribution_policy_digest() {
        return Err(ClinicalDistributionTrustV2Error::CurrentnessDistributionPolicyMismatch);
    }
    if currentness.trust_policy_digest() != legacy.trust_policy_digest() {
        return Err(ClinicalDistributionTrustV2Error::CurrentnessTrustPolicyMismatch);
    }
    if currentness.admission_evidence_digest()
        != legacy.evaluator_admission_evidence_digest()
    {
        return Err(ClinicalDistributionTrustV2Error::CurrentnessAdmissionEvidenceMismatch);
    }

    validate_digest(legacy.assessment_digest())?;
    validate_digest(legacy.distribution_policy_digest())?;
    validate_digest(legacy.trust_policy_digest())?;
    validate_digest(legacy.evaluator_admission_evidence_digest())?;
    validate_digest(legacy.receipt_digest())?;

    let currentness_digest = currentness.currentness_digest();
    let receipt_digest = compute_receipt_digest(TrustReceiptV2Material {
        assessment_digest: legacy.assessment_digest(),
        distribution_policy_digest: legacy.distribution_policy_digest(),
        trust_policy_digest: legacy.trust_policy_digest(),
        evaluator_admission_evidence_digest: legacy.evaluator_admission_evidence_digest(),
        legacy_trust_receipt_digest: legacy.receipt_digest(),
        currentness_policy_digest: &currentness_policy_digest,
        currentness_digest: &currentness_digest,
        status: legacy.status(),
        assessed_at_micros: legacy.assessed_at_micros(),
        trusted_at_micros,
        currentness_valid_until_micros: currentness.valid_until_micros(),
    })?;

    Ok(DistributionTrustReceiptV2 {
        assessment_digest: legacy.assessment_digest().clone(),
        distribution_policy_digest: legacy.distribution_policy_digest().clone(),
        trust_policy_digest: legacy.trust_policy_digest().clone(),
        evaluator_admission_evidence_digest: legacy.evaluator_admission_evidence_digest().clone(),
        legacy_trust_receipt_digest: legacy.receipt_digest().clone(),
        currentness_policy_digest,
        currentness_digest,
        status: legacy.status(),
        assessed_at_micros: legacy.assessed_at_micros(),
        trusted_at_micros,
        currentness_valid_until_micros: currentness.valid_until_micros(),
        receipt_digest,
    })
}

struct TrustReceiptV2Material<'a> {
    assessment_digest: &'a ContentDigest,
    distribution_policy_digest: &'a ContentDigest,
    trust_policy_digest: &'a ContentDigest,
    evaluator_admission_evidence_digest: &'a ContentDigest,
    legacy_trust_receipt_digest: &'a ContentDigest,
    currentness_policy_digest: &'a DistributionEvaluatorCurrentnessPolicyDigestV1,
    currentness_digest: &'a DistributionEvaluatorCurrentnessDigestV1,
    status: ClinicalDistributionStatusV1,
    assessed_at_micros: i64,
    trusted_at_micros: i64,
    currentness_valid_until_micros: i64,
}

fn compute_receipt_digest(
    material: TrustReceiptV2Material<'_>,
) -> Result<DistributionTrustReceiptDigestV2, ClinicalDistributionTrustV2Error> {
    let mut framed = Vec::new();
    append_content_digest(&mut framed, material.assessment_digest)?;
    append_content_digest(&mut framed, material.distribution_policy_digest)?;
    append_content_digest(&mut framed, material.trust_policy_digest)?;
    append_content_digest(&mut framed, material.evaluator_admission_evidence_digest)?;
    append_content_digest(&mut framed, material.legacy_trust_receipt_digest)?;
    append_bytes(&mut framed, material.currentness_policy_digest.as_bytes())?;
    append_bytes(&mut framed, material.currentness_digest.as_bytes())?;
    framed.push(status_tag(material.status));
    framed.extend_from_slice(&material.assessed_at_micros.to_be_bytes());
    framed.extend_from_slice(&material.trusted_at_micros.to_be_bytes());
    framed.extend_from_slice(&material.currentness_valid_until_micros.to_be_bytes());

    let mut hasher = blake3::Hasher::new_derive_key(RECEIPT_DERIVE_KEY_CONTEXT);
    hasher.update(&(framed.len() as u64).to_be_bytes());
    hasher.update(&framed);
    Ok(DistributionTrustReceiptDigestV2(*hasher.finalize().as_bytes()))
}

fn status_tag(status: ClinicalDistributionStatusV1) -> u8 {
    match status {
        ClinicalDistributionStatusV1::NotRun => 0,
        ClinicalDistributionStatusV1::Unavailable => 1,
        ClinicalDistributionStatusV1::Indeterminate => 2,
        ClinicalDistributionStatusV1::InDistribution => 3,
        ClinicalDistributionStatusV1::OutOfDistribution => 4,
    }
}

fn append_content_digest(
    target: &mut Vec<u8>,
    digest: &ContentDigest,
) -> Result<(), ClinicalDistributionTrustV2Error> {
    validate_digest(digest)?;
    append_bytes(target, digest.algorithm.as_bytes())?;
    append_bytes(target, digest.value.as_bytes())
}

fn append_bytes(
    target: &mut Vec<u8>,
    value: &[u8],
) -> Result<(), ClinicalDistributionTrustV2Error> {
    let len = u32::try_from(value.len())
        .map_err(|_| ClinicalDistributionTrustV2Error::DigestMaterialTooLarge)?;
    target.extend_from_slice(&len.to_be_bytes());
    target.extend_from_slice(value);
    Ok(())
}

fn validate_digest(digest: &ContentDigest) -> Result<(), ClinicalDistributionTrustV2Error> {
    if digest.algorithm.is_empty()
        || digest.value.is_empty()
        || digest.algorithm.trim() != digest.algorithm
        || digest.value.trim() != digest.value
        || digest.algorithm.chars().any(char::is_control)
        || digest.value.chars().any(char::is_control)
    {
        return Err(ClinicalDistributionTrustV2Error::InvalidDigest);
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClinicalDistributionTrustV2Error {
    #[error(transparent)]
    LegacyTrust(#[from] ClinicalDistributionTrustError),
    #[error(transparent)]
    Currentness(#[from] DistributionEvaluatorCurrentnessError),
    #[error("runtime currentness proof belongs to another currentness policy")]
    CurrentnessPolicyMismatch,
    #[error("runtime currentness proof is not active at trust evaluation time")]
    CurrentnessInactive,
    #[error("runtime currentness detector differs from trust policy detector")]
    CurrentnessDetectorMismatch,
    #[error("runtime currentness structural distribution policy differs from trust receipt")]
    CurrentnessDistributionPolicyMismatch,
    #[error("runtime currentness evaluator trust policy differs from trust receipt")]
    CurrentnessTrustPolicyMismatch,
    #[error("runtime currentness admission evidence differs from trust receipt")]
    CurrentnessAdmissionEvidenceMismatch,
    #[error("receipt digest material contains an invalid content digest")]
    InvalidDigest,
    #[error("receipt digest material exceeds framing limits")]
    DigestMaterialTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clinical_distribution_lease_integrity::ClinicalDistributionEvaluatorLeaseV1;
    use clinical_distribution_lease_zome::{
        DistributionEvaluatorLeaseReadBoundaryV1, DistributionEvaluatorPositiveLeaseObservationV1,
    };
    use clinical_distribution_trust_integrity::{
        DistributionEvaluatorAdmissionProposalV1, QualifiedDistributionEvaluatorAdmission,
        RuntimeArtifactIdentityV1, RuntimeContentDigestV1,
    };
    use clinical_distribution_trust_zome::{
        DistributionEvaluatorAdmissionSnapshotV1, DistributionEvaluatorSnapshotReadBoundaryV1,
    };
    use hdi::prelude::Timestamp;
    use holo_hash::ActionHash;
    use mycelix_clinical_distribution_assessment::{
        capsule_binding_digest, validate_distribution_for_capsule,
        ClinicalDistributionAssessmentV1, NumericDistributionBoundaryV1,
        NumericDistributionDirectionV1, CLINICAL_DISTRIBUTION_ASSESSMENT_VERSION,
        CLINICAL_DISTRIBUTION_POLICY_VERSION,
    };
    use mycelix_clinical_distribution_currentness::{
        compose_distribution_evaluator_currentness_v1,
        DISTRIBUTION_EVALUATOR_CURRENTNESS_POLICY_VERSION,
    };
    use mycelix_clinical_evidence::{
        ArtifactIdentity, AssertionKind, AssertionUncertainty, AuthorityMode, ClinicalAuthority,
        ClinicalEvidenceCapsule, ContentDigest, EvidenceRole, ExecutionIdentity, FactEvidence,
        HumanReview, IntendedUse, QualificationLevel, ReviewStatus,
    };
    use mycelix_clinical_semantics::{EvaluationState, SubjectRef};

    fn digest(value: &str) -> ContentDigest {
        ContentDigest {
            algorithm: "blake3-256".into(),
            value: value.into(),
        }
    }

    fn artifact(name: &str, version: &str, value: &str) -> ArtifactIdentity {
        ArtifactIdentity {
            name: name.into(),
            version: version.into(),
            digest: digest(value),
        }
    }

    fn capsule() -> ClinicalEvidenceCapsule {
        ClinicalEvidenceCapsule {
            capsule_id: "capsule-ai-1".into(),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: "patient-a".into(),
            },
            kind: AssertionKind::RiskPrediction,
            statement: "Risk signal".into(),
            code: None,
            state: EvaluationState::Satisfied,
            reasons: vec!["model output".into()],
            fact_evidence: vec![FactEvidence {
                fact_id: "fact-1".into(),
                role: EvidenceRole::Supports,
                fact_digest: Some(digest("fact")),
            }],
            sources: vec![],
            missing_requirements: vec![],
            alternatives: vec![],
            uncertainty: Some(AssertionUncertainty {
                confidence: Some(0.7),
                interpretation: Some("calibrated".into()),
                calibration_reference: Some(digest("calibration")),
            }),
            execution: ExecutionIdentity {
                engine: artifact("symthaea", "1.0.0", "engine"),
                model: Some(artifact("risk-model", "1.0.0", "model")),
                knowledge_artifact: None,
                environment_digest: Some(digest("environment")),
                operation: "risk".into(),
            },
            intended_use: IntendedUse {
                use_case: "decision support".into(),
                intended_user: "licensed clinician".into(),
                population: "adults".into(),
                care_setting: "outpatient".into(),
                qualification: QualificationLevel::SupervisedClinical,
            },
            authority: ClinicalAuthority {
                mode: AuthorityMode::ClinicianReviewRequired,
                review: HumanReview {
                    status: ReviewStatus::Approved,
                    reviewer: Some("Practitioner/1".into()),
                    reviewed_at_micros: Some(1_050),
                    notes: None,
                },
            },
            issued_at_micros: 900,
            supersedes_capsule_id: None,
        }
    }

    fn structural_policy() -> ClinicalDistributionPolicyV1 {
        ClinicalDistributionPolicyV1 {
            schema_version: CLINICAL_DISTRIBUTION_POLICY_VERSION,
            policy_id: "distribution-policy-v1".into(),
            required_detector: artifact("ood-detector", "1.0.0", "detector"),
            required_reference_population: "adult-outpatient-v1".into(),
            required_reference_domain_digest: digest("reference-domain"),
            max_age_micros: 10_000,
            max_future_skew_micros: 100,
        }
    }

    fn trust_policy(structural: &ClinicalDistributionPolicyV1) -> DistributionEvaluatorTrustPolicyV1 {
        DistributionEvaluatorTrustPolicyV1 {
            schema_version: 1,
            policy_id: "distribution-trust-v1".into(),
            distribution_policy_digest: structural.digest().unwrap(),
            required_detector: structural.required_detector.clone(),
            max_future_skew_micros: 100,
        }
    }

    fn validated_assessment(
        capsule: &ClinicalEvidenceCapsule,
        policy: &ClinicalDistributionPolicyV1,
    ) -> ValidatedClinicalDistributionAssessment {
        let assessment = ClinicalDistributionAssessmentV1 {
            schema_version: CLINICAL_DISTRIBUTION_ASSESSMENT_VERSION,
            assessment_id: "assessment-1".into(),
            subject: capsule.subject.clone(),
            model: capsule.execution.model.clone().unwrap(),
            detector: policy.required_detector.clone(),
            reference_population: policy.required_reference_population.clone(),
            reference_domain_digest: policy.required_reference_domain_digest.clone(),
            capsule_binding_digest: capsule_binding_digest(capsule).unwrap(),
            status: ClinicalDistributionStatusV1::InDistribution,
            numeric_boundary: Some(NumericDistributionBoundaryV1 {
                score: 0.8,
                threshold: 0.5,
                direction: NumericDistributionDirectionV1::HigherMeansMoreInDistribution,
            }),
            assessment_evidence_digest: Some(digest("distribution-evidence")),
            assessed_at_micros: 1_000,
        };
        validate_distribution_for_capsule(capsule, &assessment, policy, 1_100).unwrap()
    }

    fn runtime_digest(value: &ContentDigest) -> RuntimeContentDigestV1 {
        RuntimeContentDigestV1 {
            algorithm: value.algorithm.clone(),
            value: value.value.clone(),
        }
    }

    fn runtime_artifact(value: &ArtifactIdentity) -> RuntimeArtifactIdentityV1 {
        RuntimeArtifactIdentityV1 {
            name: value.name.clone(),
            version: value.version.clone(),
            digest: runtime_digest(&value.digest),
        }
    }

    fn action_hash(seed: u8) -> ActionHash {
        ActionHash::from_raw_36(vec![seed; 36])
    }

    fn currentness_and_admission(
        structural_policy: &ClinicalDistributionPolicyV1,
        trust_policy: &DistributionEvaluatorTrustPolicyV1,
    ) -> (
        DistributionEvaluatorCurrentnessPolicyV1,
        VerifiedDistributionEvaluatorCurrentnessV1,
        VerifiedDistributionEvaluatorAdmission,
    ) {
        let trust_policy_digest = trust_policy.digest().unwrap();
        let external_evidence = digest("registry-evidence");
        let proposal = DistributionEvaluatorAdmissionProposalV1 {
            schema_version: 1,
            admission_id: "admission-a".into(),
            detector: runtime_artifact(&structural_policy.required_detector),
            distribution_policy_digest: runtime_digest(&structural_policy.digest().unwrap()),
            trust_policy_digest: runtime_digest(&trust_policy_digest),
            external_admission_evidence_digest: runtime_digest(&external_evidence),
            valid_from: Timestamp::from_micros(500),
            valid_until: Some(Timestamp::from_micros(5_000_000)),
        };
        let proposal_digest = proposal.digest().unwrap();
        let admission_hash = action_hash(4);
        let snapshot = DistributionEvaluatorAdmissionSnapshotV1 {
            admission_action_hash: admission_hash.clone(),
            admission: QualifiedDistributionEvaluatorAdmission {
                proposal: proposal.clone(),
                verifier_authorization_hash: action_hash(7),
            },
            revocations: vec![],
            read_boundary:
                DistributionEvaluatorSnapshotReadBoundaryV1::NetworkBackedStableDoubleRead,
            observed_revocation_target_count: 0,
            observation_started_at: Timestamp::from_micros(1_050),
            observation_completed_at: Timestamp::from_micros(1_100),
        };
        let lease = DistributionEvaluatorPositiveLeaseObservationV1 {
            admission_action_hash: admission_hash.clone(),
            admission_proposal_digest: proposal_digest,
            selected_lease_action_hash: action_hash(8),
            selected_lease: ClinicalDistributionEvaluatorLeaseV1 {
                schema_version: 1,
                lease_id: "lease-a".into(),
                admission_hash,
                admission_proposal_digest: proposal_digest,
                detector: proposal.detector,
                distribution_policy_digest: proposal.distribution_policy_digest,
                trust_policy_digest: proposal.trust_policy_digest,
                lease_sequence: 0,
                supersedes_lease_hash: None,
                valid_from: Timestamp::from_micros(1_000),
                valid_until: Timestamp::from_micros(2_000_000),
            },
            observed_lease_target_count: 1,
            read_boundary:
                DistributionEvaluatorLeaseReadBoundaryV1::NetworkBackedStableDoubleRead,
            observation_started_at: Timestamp::from_micros(1_075),
            observation_completed_at: Timestamp::from_micros(1_125),
        };
        let currentness_policy = DistributionEvaluatorCurrentnessPolicyV1 {
            schema_version: DISTRIBUTION_EVALUATOR_CURRENTNESS_POLICY_VERSION,
            policy_id: "currentness-v1".into(),
            max_observation_age_micros: 1_000_000,
            max_inter_observation_gap_micros: 500_000,
        };
        let currentness = compose_distribution_evaluator_currentness_v1(
            &snapshot,
            &lease,
            &currentness_policy,
            1_200,
        )
        .unwrap();
        let admission = VerifiedDistributionEvaluatorAdmission::from_verified_record(
            structural_policy.required_detector.clone(),
            trust_policy_digest,
            500,
            Some(5_000_000),
            None,
            external_evidence,
        )
        .unwrap();
        (currentness_policy, currentness, admission)
    }

    #[test]
    fn exact_currentness_is_required_and_bound_into_receipt() {
        let capsule = capsule();
        let structural_policy = structural_policy();
        let trust_policy = trust_policy(&structural_policy);
        let structural = validated_assessment(&capsule, &structural_policy);
        let (currentness_policy, currentness, admission) =
            currentness_and_admission(&structural_policy, &trust_policy);

        let receipt = evaluate_distribution_trust_v2(
            &structural,
            &structural_policy,
            &trust_policy,
            &admission,
            &currentness_policy,
            &currentness,
            1_200,
        )
        .unwrap();

        assert_eq!(receipt.status(), ClinicalDistributionStatusV1::InDistribution);
        assert_eq!(receipt.currentness_digest(), currentness.currentness_digest());
        assert_ne!(receipt.receipt_digest().as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn substituted_currentness_policy_is_rejected() {
        let capsule = capsule();
        let structural_policy = structural_policy();
        let trust_policy = trust_policy(&structural_policy);
        let structural = validated_assessment(&capsule, &structural_policy);
        let (mut currentness_policy, currentness, admission) =
            currentness_and_admission(&structural_policy, &trust_policy);
        currentness_policy.policy_id = "substituted-currentness-policy".into();

        assert_eq!(
            evaluate_distribution_trust_v2(
                &structural,
                &structural_policy,
                &trust_policy,
                &admission,
                &currentness_policy,
                &currentness,
                1_200,
            )
            .err(),
            Some(ClinicalDistributionTrustV2Error::CurrentnessPolicyMismatch)
        );
    }

    #[test]
    fn expired_currentness_cannot_be_reused() {
        let capsule = capsule();
        let structural_policy = structural_policy();
        let trust_policy = trust_policy(&structural_policy);
        let structural = validated_assessment(&capsule, &structural_policy);
        let (currentness_policy, currentness, admission) =
            currentness_and_admission(&structural_policy, &trust_policy);

        assert_eq!(
            evaluate_distribution_trust_v2(
                &structural,
                &structural_policy,
                &trust_policy,
                &admission,
                &currentness_policy,
                &currentness,
                currentness.valid_until_micros(),
            )
            .err(),
            Some(ClinicalDistributionTrustV2Error::CurrentnessInactive)
        );
    }
}
