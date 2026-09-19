#![deny(unsafe_code)]
//! Composition of independently admitted Symthaea v2 inference with independently
//! verified Mycelix ClinicalFact bindings.
//!
//! This crate creates a non-serializable evidence context only when both proof
//! lines refer to the same exact Symthaea wire artifact and the exact typed fact
//! identities still agree. It does not construct clinical presentation authority.

use mycelix_clinical_evidence::EvidenceRole;
use mycelix_clinical_evidence_v2::{
    TypedFactEvidenceV1, MYCELIX_CLINICAL_FACT_SNAPSHOT_NAMESPACE_V1,
};
use mycelix_symthaea_clinical_admission_v2::{
    AdmittedSymthaeaInferenceV2, SymthaeaAdmissionCeilingV2,
    SymthaeaAdmissionPolicyDigestV2,
};
use mycelix_symthaea_clinical_fact_binding::{
    SymthaeaFactBindingPolicyDigestV1, VerifiedSymthaeaClinicalFactBindingSetV1,
};
use mycelix_symthaea_clinical_wire_v2::{
    SymthaeaClaimKindV2, SymthaeaEvidenceRoleV2, SymthaeaEvidenceStageV2,
    SymthaeaIntendedUseV2, VerifiedSymthaeaWireDigestV2,
};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

const CONTEXT_DERIVE_KEY: &str = "mycelix.health.symthaea-clinical-evidence-context.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaEvidenceContextDigestV1([u8; 32]);

impl SymthaeaEvidenceContextDigestV1 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Non-serializable composition of exact admission + exact ClinicalFact binding.
pub struct VerifiedSymthaeaEvidenceContextV1 {
    wire_digest: VerifiedSymthaeaWireDigestV2,
    admission_policy_digest: SymthaeaAdmissionPolicyDigestV2,
    fact_binding_policy_digest: SymthaeaFactBindingPolicyDigestV1,
    context_digest: SymthaeaEvidenceContextDigestV1,
    qualification_ceiling: SymthaeaAdmissionCeilingV2,
    subject_id: String,
    claim_kind: SymthaeaClaimKindV2,
    evidence_stage: SymthaeaEvidenceStageV2,
    intended_use: SymthaeaIntendedUseV2,
    statement: String,
    typed_fact_evidence: Vec<TypedFactEvidenceV1>,
}

impl VerifiedSymthaeaEvidenceContextV1 {
    pub fn wire_digest(&self) -> VerifiedSymthaeaWireDigestV2 {
        self.wire_digest
    }

    pub fn admission_policy_digest(&self) -> SymthaeaAdmissionPolicyDigestV2 {
        self.admission_policy_digest
    }

    pub fn fact_binding_policy_digest(&self) -> SymthaeaFactBindingPolicyDigestV1 {
        self.fact_binding_policy_digest
    }

    pub fn context_digest(&self) -> SymthaeaEvidenceContextDigestV1 {
        self.context_digest
    }

    pub fn qualification_ceiling(&self) -> SymthaeaAdmissionCeilingV2 {
        self.qualification_ceiling
    }

    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }

    pub fn claim_kind(&self) -> SymthaeaClaimKindV2 {
        self.claim_kind
    }

    pub fn evidence_stage(&self) -> SymthaeaEvidenceStageV2 {
        self.evidence_stage
    }

    pub fn intended_use(&self) -> SymthaeaIntendedUseV2 {
        self.intended_use
    }

    pub fn statement(&self) -> &str {
        &self.statement
    }

    pub fn typed_fact_evidence(&self) -> &[TypedFactEvidenceV1] {
        &self.typed_fact_evidence
    }
}

/// Compose two independently established proof lines.
///
/// This function does not trust either proof merely because it exists. It checks
/// that both proofs name the same exact wire artifact, the same patient, and the
/// same exact ClinicalFact evidence identities before producing a context.
pub fn compose_verified_symthaea_evidence_context_v1(
    admission: &AdmittedSymthaeaInferenceV2,
    fact_bindings: &VerifiedSymthaeaClinicalFactBindingSetV1,
) -> Result<VerifiedSymthaeaEvidenceContextV1, SymthaeaEvidenceContextError> {
    if admission.wire_digest() != fact_bindings.wire_digest() {
        return Err(SymthaeaEvidenceContextError::WireDigestMismatch);
    }

    let subject = admission
        .envelope()
        .subject
        .as_ref()
        .ok_or(SymthaeaEvidenceContextError::SubjectRequired)?;
    if subject.subject_id != fact_bindings.subject_id() {
        return Err(SymthaeaEvidenceContextError::SubjectMismatch);
    }

    let mut admitted_facts = HashMap::new();
    for evidence in &admission.envelope().evidence {
        if evidence.identity.namespace == MYCELIX_CLINICAL_FACT_SNAPSHOT_NAMESPACE_V1 {
            if admitted_facts
                .insert(evidence.identity.artifact_id.as_str(), evidence)
                .is_some()
            {
                return Err(SymthaeaEvidenceContextError::DuplicateAdmittedFact);
            }
        }
    }

    if admitted_facts.len() != fact_bindings.bindings().len() {
        return Err(SymthaeaEvidenceContextError::FactBindingCoverageMismatch);
    }

    let mut seen = HashSet::new();
    let mut typed_fact_evidence = Vec::with_capacity(fact_bindings.bindings().len());
    for binding in fact_bindings.bindings() {
        if !seen.insert(binding.fact_id()) {
            return Err(SymthaeaEvidenceContextError::DuplicateVerifiedFact);
        }
        let admitted = admitted_facts
            .get(binding.evidence_artifact_id())
            .copied()
            .ok_or_else(|| {
                SymthaeaEvidenceContextError::MissingAdmittedFact(
                    binding.evidence_artifact_id().to_string(),
                )
            })?;

        if admitted.role != binding.role() {
            return Err(SymthaeaEvidenceContextError::EvidenceRoleMismatch);
        }
        if admitted.identity.digest.value != binding.fact_snapshot_digest().into_bytes() {
            return Err(SymthaeaEvidenceContextError::FactDigestMismatch);
        }
        if admitted.identity.artifact_id != binding.fact_id() {
            return Err(SymthaeaEvidenceContextError::FactArtifactIdMismatch);
        }

        typed_fact_evidence.push(TypedFactEvidenceV1::from_clinical_fact_snapshot(
            binding.fact_id().to_string(),
            map_role(binding.role()),
            binding.fact_snapshot_digest(),
        ));
    }

    let admission_policy_digest = admission.policy_digest();
    let fact_binding_policy_digest = fact_bindings.policy_digest();
    let qualification_ceiling = admission.qualification_ceiling();
    let context_digest = context_digest(
        admission.wire_digest(),
        admission_policy_digest,
        fact_binding_policy_digest,
        qualification_ceiling,
        &subject.subject_id,
        &typed_fact_evidence,
    );

    Ok(VerifiedSymthaeaEvidenceContextV1 {
        wire_digest: admission.wire_digest(),
        admission_policy_digest,
        fact_binding_policy_digest,
        context_digest,
        qualification_ceiling,
        subject_id: subject.subject_id.clone(),
        claim_kind: admission.envelope().semantics.claim_kind,
        evidence_stage: admission.envelope().semantics.evidence_stage,
        intended_use: admission.envelope().semantics.intended_use,
        statement: admission.envelope().statement.clone(),
        typed_fact_evidence,
    })
}

fn map_role(role: SymthaeaEvidenceRoleV2) -> EvidenceRole {
    match role {
        SymthaeaEvidenceRoleV2::Supports => EvidenceRole::Supports,
        SymthaeaEvidenceRoleV2::Opposes => EvidenceRole::Opposes,
        SymthaeaEvidenceRoleV2::Context => EvidenceRole::Context,
        SymthaeaEvidenceRoleV2::Contraindication => EvidenceRole::Contraindication,
    }
}

fn context_digest(
    wire: VerifiedSymthaeaWireDigestV2,
    admission_policy: SymthaeaAdmissionPolicyDigestV2,
    binding_policy: SymthaeaFactBindingPolicyDigestV1,
    ceiling: SymthaeaAdmissionCeilingV2,
    subject_id: &str,
    facts: &[TypedFactEvidenceV1],
) -> SymthaeaEvidenceContextDigestV1 {
    let mut hasher = blake3::Hasher::new_derive_key(CONTEXT_DERIVE_KEY);
    hasher.update(wire.as_bytes());
    hasher.update(admission_policy.as_bytes());
    hasher.update(binding_policy.as_bytes());
    hasher.update(&[ceiling_tag(ceiling)]);
    hasher.update(&(subject_id.len() as u32).to_be_bytes());
    hasher.update(subject_id.as_bytes());
    hasher.update(&(facts.len() as u32).to_be_bytes());
    for fact in facts {
        hasher.update(&(fact.fact_id.len() as u32).to_be_bytes());
        hasher.update(fact.fact_id.as_bytes());
        hasher.update(&[role_tag(fact.role)]);
        hasher.update(&(fact.identity.namespace.len() as u32).to_be_bytes());
        hasher.update(fact.identity.namespace.as_bytes());
        hasher.update(&(fact.identity.artifact_id.len() as u32).to_be_bytes());
        hasher.update(fact.identity.artifact_id.as_bytes());
        hasher.update(&fact.identity.digest);
    }
    SymthaeaEvidenceContextDigestV1(*hasher.finalize().as_bytes())
}

const fn role_tag(role: EvidenceRole) -> u8 {
    match role {
        EvidenceRole::Supports => 0,
        EvidenceRole::Opposes => 1,
        EvidenceRole::Context => 2,
        EvidenceRole::Contraindication => 3,
    }
}

const fn ceiling_tag(ceiling: SymthaeaAdmissionCeilingV2) -> u8 {
    match ceiling {
        SymthaeaAdmissionCeilingV2::Experimental => 0,
        SymthaeaAdmissionCeilingV2::ValidatedOffline => 1,
        SymthaeaAdmissionCeilingV2::ShadowClinical => 2,
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SymthaeaEvidenceContextError {
    #[error("admission and ClinicalFact binding refer to different Symthaea wire artifacts")]
    WireDigestMismatch,
    #[error("a subject is required for verified ClinicalFact evidence composition")]
    SubjectRequired,
    #[error("admission and ClinicalFact binding refer to different subjects")]
    SubjectMismatch,
    #[error("duplicate admitted Mycelix ClinicalFact artifact id")]
    DuplicateAdmittedFact,
    #[error("verified fact bindings do not cover admitted Mycelix fact evidence exactly")]
    FactBindingCoverageMismatch,
    #[error("duplicate verified ClinicalFact binding")]
    DuplicateVerifiedFact,
    #[error("verified ClinicalFact is absent from the admitted inference: {0}")]
    MissingAdmittedFact(String),
    #[error("evidence role differs between admitted inference and verified binding")]
    EvidenceRoleMismatch,
    #[error("ClinicalFact snapshot digest differs between admitted inference and verified binding")]
    FactDigestMismatch,
    #[error("ClinicalFact artifact id differs between admitted inference and verified binding")]
    FactArtifactIdMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_semantics::{
        ClinicalFact, ClinicalValue, CodeableConcept, Coding, FactProvenance, Quantity,
        SubjectRef, TransformationProvenance, Uncertainty,
    };
    use mycelix_symthaea_clinical_admission_v2::{
        admit_symthaea_clinical_inference_v2, SymthaeaClinicalAdmissionPolicyV2,
        SYMTHAEA_CLINICAL_ADMISSION_POLICY_V2_VERSION,
    };
    use mycelix_symthaea_clinical_fact_binding::{
        verify_symthaea_clinical_fact_bindings_v1, SymthaeaFactBindingPolicyV1,
        SYMTHAEA_FACT_BINDING_POLICY_VERSION,
    };
    use mycelix_symthaea_clinical_wire_v2::{
        verify_symthaea_clinical_wire_v2, SYMTHAEA_CLINICAL_ENVELOPE_V2_VERSION,
        SYMTHAEA_CLINICAL_WIRE_V2_VERSION,
    };

    const VECTOR_HEX: &str = include_str!(
        "../../symthaea-clinical-wire-v2/fixtures/clinical_inference_wire_v2.hex"
    );

    fn fixture() -> Vec<u8> {
        let compact: Vec<u8> = VECTOR_HEX
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect();
        compact
            .chunks_exact(2)
            .map(|pair| (hex(pair[0]) << 4) | hex(pair[1]))
            .collect()
    }

    fn hex(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            b'A'..=b'F' => value - b'A' + 10,
            _ => panic!("invalid fixture hex"),
        }
    }

    fn fact() -> ClinicalFact {
        ClinicalFact {
            fact_id: "fact-1".into(),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: "patient-a".into(),
            },
            concept: CodeableConcept {
                coding: vec![Coding {
                    system: "http://loinc.org".into(),
                    code: "718-7".into(),
                    display: Some("Hemoglobin [Mass/volume] in Blood".into()),
                    version: Some("2.80".into()),
                }],
                text: Some("Hemoglobin".into()),
            },
            value: ClinicalValue::Quantity(Quantity::ucum(13.7, "g/dL")),
            effective_at_micros: 900,
            provenance: FactProvenance {
                source_system: "https://ehr.example/fhir".into(),
                source_resource_type: "Observation".into(),
                source_resource_id: "obs-1".into(),
                source_version: Some("7".into()),
                recorded_at_micros: 1_000,
                asserted_by: Some("Practitioner/clinician-a".into()),
                transformation: Some(TransformationProvenance {
                    software: "mycelix-fhir-bridge".into(),
                    version: "1.0.0".into(),
                    operation: "observation_to_clinical_fact".into(),
                    input_fact_ids: vec!["source-observation-obs-1".into()],
                }),
            },
            uncertainty: Some(Uncertainty {
                confidence: Some(0.98),
                interpretation: Some("laboratory measurement confidence".into()),
            }),
        }
    }

    fn wire_bound_to_fact(fact: &ClinicalFact) -> Vec<u8> {
        use mycelix_clinical_fact_snapshot::clinical_fact_snapshot_digest_v1;
        let mut bytes = fixture();
        let digest = clinical_fact_snapshot_digest_v1(fact).unwrap().into_bytes();
        let needle = [11u8; 32];
        let mut replacements = 0;
        let mut index = 0;
        while index + needle.len() <= bytes.len() {
            if bytes[index..index + needle.len()] == needle {
                bytes[index..index + needle.len()].copy_from_slice(&digest);
                replacements += 1;
                index += needle.len();
            } else {
                index += 1;
            }
        }
        assert_eq!(replacements, 2);
        bytes
    }

    fn admission_policy(wire: &[u8]) -> SymthaeaClinicalAdmissionPolicyV2 {
        let envelope = verify_symthaea_clinical_wire_v2(wire).unwrap();
        SymthaeaClinicalAdmissionPolicyV2 {
            schema_version: SYMTHAEA_CLINICAL_ADMISSION_POLICY_V2_VERSION,
            policy_id: "context-admission-v1".into(),
            accepted_wire_version: SYMTHAEA_CLINICAL_WIRE_V2_VERSION,
            accepted_envelope_version: SYMTHAEA_CLINICAL_ENVELOPE_V2_VERSION,
            accepted_engine: envelope.execution.engine.clone(),
            accepted_model: envelope.execution.model.clone(),
            accepted_runtime_digest: envelope.execution.runtime_digest,
            accepted_configuration_digest: envelope.execution.configuration_digest,
            accepted_operation: envelope.execution.operation.clone(),
            allowed_claim_kinds: vec![envelope.semantics.claim_kind],
            allowed_evidence_stages: vec![envelope.semantics.evidence_stage],
            required_intended_use: envelope.semantics.intended_use,
            required_subject_namespace: Some("fhir/Patient".into()),
            required_subject_binding_namespace: Some(
                "mycelix/patient-subject-binding-evidence/v1".into(),
            ),
            allowed_claim_evidence_namespaces: vec![
                MYCELIX_CLINICAL_FACT_SNAPSHOT_NAMESPACE_V1.into(),
            ],
            required_claim_evidence_namespaces: vec![
                MYCELIX_CLINICAL_FACT_SNAPSHOT_NAMESPACE_V1.into(),
            ],
            required_producer_distribution_namespace: Some(
                "symthaea/ood-detector-evidence/v1".into(),
            ),
            max_inference_age_micros: 10_000,
            max_future_skew_micros: 100,
            require_calibrated_probability: true,
            qualification_ceiling: SymthaeaAdmissionCeilingV2::ShadowClinical,
        }
    }

    fn binding_policy() -> SymthaeaFactBindingPolicyV1 {
        SymthaeaFactBindingPolicyV1 {
            schema_version: SYMTHAEA_FACT_BINDING_POLICY_VERSION,
            policy_id: "context-binding-v1".into(),
            external_subject_namespace: "fhir/Patient".into(),
            mycelix_subject_resource_type: "Patient".into(),
        }
    }

    #[test]
    fn exact_admission_and_fact_binding_compose() {
        let fact = fact();
        let wire = wire_bound_to_fact(&fact);
        let admission =
            admit_symthaea_clinical_inference_v2(&wire, &admission_policy(&wire), 1_100).unwrap();
        let bindings = verify_symthaea_clinical_fact_bindings_v1(
            &wire,
            &[fact],
            &binding_policy(),
        )
        .unwrap();
        let context = compose_verified_symthaea_evidence_context_v1(&admission, &bindings).unwrap();
        assert_eq!(context.subject_id(), "patient-a");
        assert_eq!(context.statement(), "Candidate risk prediction");
        assert_eq!(context.typed_fact_evidence().len(), 1);
        assert_eq!(context.typed_fact_evidence()[0].fact_id, "fact-1");
        assert_ne!(context.context_digest().as_bytes(), &[0u8; 32]);
    }
}
