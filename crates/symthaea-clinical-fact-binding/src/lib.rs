#![deny(unsafe_code)]
//! Exact binding from typed Symthaea v2 evidence identities to locally validated
//! Mycelix [`ClinicalFact`] snapshots.
//!
//! An evidence namespace is a type claim, not proof. For the namespace
//! `mycelix/clinical-fact-snapshot/v1`, this crate requires the actual Mycelix fact,
//! validates it, computes the canonical snapshot digest locally, and binds subject,
//! artifact ID and digest before producing a non-serializable verification result.
//!
//! This remains evidence verification, not clinical authority.

use mycelix_clinical_fact_snapshot::{
    clinical_fact_snapshot_digest_v1, ClinicalFactSnapshotDigestV1, ClinicalFactSnapshotError,
};
use mycelix_clinical_semantics::ClinicalFact;
use mycelix_symthaea_clinical_wire_v2::{
    verify_symthaea_clinical_wire_v2, verify_symthaea_clinical_wire_v2_digest,
    SymthaeaClinicalInferenceV2, SymthaeaDigestAlgorithmV2, SymthaeaEvidenceRoleV2,
    SymthaeaIntendedUseV2, SymthaeaWireV2Error, VerifiedSymthaeaWireDigestV2,
};
use std::collections::{HashMap, HashSet};

pub const SYMTHAEA_FACT_BINDING_POLICY_VERSION: u16 = 1;
pub const MYCELIX_CLINICAL_FACT_SNAPSHOT_NAMESPACE_V1: &str =
    "mycelix/clinical-fact-snapshot/v1";

const POLICY_DERIVE_KEY_CONTEXT: &str =
    "mycelix.health.symthaea-clinical-fact-binding-policy.v1";

/// Explicit subject-namespace crosswalk used only for this binding operation.
///
/// Example: external `fhir/Patient` -> Mycelix `Patient`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymthaeaFactBindingPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub external_subject_namespace: String,
    pub mycelix_subject_resource_type: String,
}

impl SymthaeaFactBindingPolicyV1 {
    pub fn validate(&self) -> Result<(), SymthaeaFactBindingError> {
        if self.schema_version != SYMTHAEA_FACT_BINDING_POLICY_VERSION {
            return Err(SymthaeaFactBindingError::UnsupportedPolicyVersion(
                self.schema_version,
            ));
        }
        validate_token(&self.policy_id)?;
        validate_token(&self.external_subject_namespace)?;
        validate_token(&self.mycelix_subject_resource_type)?;
        Ok(())
    }

    pub fn digest(&self) -> Result<SymthaeaFactBindingPolicyDigestV1, SymthaeaFactBindingError> {
        self.validate()?;
        let mut framed = Vec::new();
        framed.extend_from_slice(&self.schema_version.to_be_bytes());
        append_string(&mut framed, &self.policy_id)?;
        append_string(&mut framed, &self.external_subject_namespace)?;
        append_string(&mut framed, &self.mycelix_subject_resource_type)?;
        let mut hasher = blake3::Hasher::new_derive_key(POLICY_DERIVE_KEY_CONTEXT);
        hasher.update(&(framed.len() as u64).to_be_bytes());
        hasher.update(&framed);
        Ok(SymthaeaFactBindingPolicyDigestV1(*hasher.finalize().as_bytes()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymthaeaFactBindingPolicyDigestV1([u8; 32]);

impl SymthaeaFactBindingPolicyDigestV1 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// One verified binding between a typed Symthaea evidence identity and one exact
/// locally validated Mycelix ClinicalFact snapshot.
pub struct VerifiedSymthaeaFactBindingV1 {
    evidence_artifact_id: String,
    role: SymthaeaEvidenceRoleV2,
    fact_id: String,
    fact_snapshot_digest: ClinicalFactSnapshotDigestV1,
}

impl VerifiedSymthaeaFactBindingV1 {
    pub fn evidence_artifact_id(&self) -> &str {
        &self.evidence_artifact_id
    }

    pub fn role(&self) -> SymthaeaEvidenceRoleV2 {
        self.role
    }

    pub fn fact_id(&self) -> &str {
        &self.fact_id
    }

    pub fn fact_snapshot_digest(&self) -> ClinicalFactSnapshotDigestV1 {
        self.fact_snapshot_digest
    }
}

/// Non-serializable verification result for one exact Symthaea wire artifact.
///
/// This proves only exact fact binding. It is not an admission, promotion, or
/// practitioner-authority capability.
pub struct VerifiedSymthaeaClinicalFactBindingSetV1 {
    wire_digest: VerifiedSymthaeaWireDigestV2,
    policy_digest: SymthaeaFactBindingPolicyDigestV1,
    subject_id: String,
    bindings: Vec<VerifiedSymthaeaFactBindingV1>,
}

impl VerifiedSymthaeaClinicalFactBindingSetV1 {
    pub fn wire_digest(&self) -> VerifiedSymthaeaWireDigestV2 {
        self.wire_digest
    }

    pub fn policy_digest(&self) -> SymthaeaFactBindingPolicyDigestV1 {
        self.policy_digest
    }

    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }

    pub fn bindings(&self) -> &[VerifiedSymthaeaFactBindingV1] {
        &self.bindings
    }
}

/// Verify every Symthaea evidence item that claims the Mycelix ClinicalFact
/// snapshot namespace against an actual supplied Mycelix fact.
///
/// Inputs are fail-closed:
///
/// - at least one Mycelix ClinicalFact evidence reference must exist;
/// - every referenced fact must be supplied exactly once;
/// - no unreferenced fact may be supplied;
/// - the evidence artifact ID must equal the Mycelix `fact_id`;
/// - local snapshot digest must equal the typed evidence digest;
/// - all bound facts must match the explicitly crosswalked envelope subject.
pub fn verify_symthaea_clinical_fact_bindings_v1(
    wire_bytes: &[u8],
    facts: &[ClinicalFact],
    policy: &SymthaeaFactBindingPolicyV1,
) -> Result<VerifiedSymthaeaClinicalFactBindingSetV1, SymthaeaFactBindingError> {
    policy.validate()?;
    let envelope = verify_symthaea_clinical_wire_v2(wire_bytes)?;
    let wire_digest = verify_symthaea_clinical_wire_v2_digest(wire_bytes)?;
    let policy_digest = policy.digest()?;

    let subject = envelope
        .subject
        .as_ref()
        .ok_or(SymthaeaFactBindingError::SubjectRequiredForFactBinding)?;
    if subject.subject_namespace != policy.external_subject_namespace {
        return Err(SymthaeaFactBindingError::SubjectNamespaceMismatch);
    }

    let expected = mycelix_fact_evidence(&envelope);
    if expected.is_empty() {
        return Err(SymthaeaFactBindingError::NoMycelixFactEvidence);
    }

    let mut facts_by_id: HashMap<&str, &ClinicalFact> = HashMap::new();
    for fact in facts {
        if facts_by_id.insert(fact.fact_id.as_str(), fact).is_some() {
            return Err(SymthaeaFactBindingError::DuplicateFactInput(
                fact.fact_id.clone(),
            ));
        }
    }

    let expected_ids: HashSet<&str> = expected
        .iter()
        .map(|(_, evidence)| evidence.identity.artifact_id.as_str())
        .collect();
    for fact in facts {
        if !expected_ids.contains(fact.fact_id.as_str()) {
            return Err(SymthaeaFactBindingError::UnreferencedFactInput(
                fact.fact_id.clone(),
            ));
        }
    }

    let mut bindings = Vec::with_capacity(expected.len());
    for (role, evidence) in expected {
        let fact = facts_by_id
            .get(evidence.identity.artifact_id.as_str())
            .copied()
            .ok_or_else(|| {
                SymthaeaFactBindingError::MissingFactInput(
                    evidence.identity.artifact_id.clone(),
                )
            })?;

        if fact.subject.resource_type != policy.mycelix_subject_resource_type
            || fact.subject.id != subject.subject_id
        {
            return Err(SymthaeaFactBindingError::FactSubjectMismatch(
                fact.fact_id.clone(),
            ));
        }

        if evidence.identity.digest.algorithm != SymthaeaDigestAlgorithmV2::Blake3_256 {
            return Err(SymthaeaFactBindingError::UnsupportedEvidenceDigestAlgorithm);
        }

        let snapshot_digest = clinical_fact_snapshot_digest_v1(fact)?;
        if snapshot_digest.into_bytes() != evidence.identity.digest.value {
            return Err(SymthaeaFactBindingError::FactSnapshotDigestMismatch(
                fact.fact_id.clone(),
            ));
        }

        bindings.push(VerifiedSymthaeaFactBindingV1 {
            evidence_artifact_id: evidence.identity.artifact_id.clone(),
            role,
            fact_id: fact.fact_id.clone(),
            fact_snapshot_digest: snapshot_digest,
        });
    }

    Ok(VerifiedSymthaeaClinicalFactBindingSetV1 {
        wire_digest,
        policy_digest,
        subject_id: subject.subject_id.clone(),
        bindings,
    })
}

fn mycelix_fact_evidence(
    envelope: &SymthaeaClinicalInferenceV2,
) -> Vec<(SymthaeaEvidenceRoleV2, &mycelix_symthaea_clinical_wire_v2::SymthaeaEvidenceRefV2)> {
    envelope
        .evidence
        .iter()
        .filter(|evidence| {
            evidence.identity.namespace == MYCELIX_CLINICAL_FACT_SNAPSHOT_NAMESPACE_V1
        })
        .map(|evidence| (evidence.role, evidence))
        .collect()
}

fn validate_token(value: &str) -> Result<(), SymthaeaFactBindingError> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(SymthaeaFactBindingError::InvalidPolicyToken);
    }
    if value.len() > 512 {
        return Err(SymthaeaFactBindingError::PolicyTokenTooLong);
    }
    Ok(())
}

fn append_string(target: &mut Vec<u8>, value: &str) -> Result<(), SymthaeaFactBindingError> {
    let len = u32::try_from(value.len()).map_err(|_| SymthaeaFactBindingError::PolicyTokenTooLong)?;
    target.extend_from_slice(&len.to_be_bytes());
    target.extend_from_slice(value.as_bytes());
    Ok(())
}

#[derive(Debug)]
pub enum SymthaeaFactBindingError {
    Wire(SymthaeaWireV2Error),
    Snapshot(ClinicalFactSnapshotError),
    UnsupportedPolicyVersion(u16),
    InvalidPolicyToken,
    PolicyTokenTooLong,
    SubjectRequiredForFactBinding,
    SubjectNamespaceMismatch,
    NoMycelixFactEvidence,
    DuplicateFactInput(String),
    UnreferencedFactInput(String),
    MissingFactInput(String),
    FactSubjectMismatch(String),
    UnsupportedEvidenceDigestAlgorithm,
    FactSnapshotDigestMismatch(String),
}

impl From<SymthaeaWireV2Error> for SymthaeaFactBindingError {
    fn from(value: SymthaeaWireV2Error) -> Self {
        Self::Wire(value)
    }
}

impl From<ClinicalFactSnapshotError> for SymthaeaFactBindingError {
    fn from(value: ClinicalFactSnapshotError) -> Self {
        Self::Snapshot(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_semantics::{
        ClinicalValue, CodeableConcept, Coding, FactProvenance, Quantity, SubjectRef,
        TransformationProvenance, Uncertainty,
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

    fn fact(id: &str, patient: &str) -> ClinicalFact {
        ClinicalFact {
            fact_id: id.into(),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: patient.into(),
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

    fn policy() -> SymthaeaFactBindingPolicyV1 {
        SymthaeaFactBindingPolicyV1 {
            schema_version: SYMTHAEA_FACT_BINDING_POLICY_VERSION,
            policy_id: "fhir-patient-to-mycelix-patient-v1".into(),
            external_subject_namespace: "fhir/Patient".into(),
            mycelix_subject_resource_type: "Patient".into(),
        }
    }

    /// Rewrite both occurrences of the fixture's fact snapshot digest to the local
    /// snapshot digest. This preserves the frozen framing while making the evidence
    /// claim point to the actual test fact.
    fn fixture_bound_to_fact(fact: &ClinicalFact) -> Vec<u8> {
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
        assert_eq!(replacements, 2, "fixture should bind fact digest in claim + execution input");
        bytes
    }

    #[test]
    fn exact_fact_snapshot_binding_succeeds() {
        let fact = fact("fact-1", "patient-a");
        let wire = fixture_bound_to_fact(&fact);
        let verified = verify_symthaea_clinical_fact_bindings_v1(&wire, &[fact], &policy()).unwrap();
        assert_eq!(verified.subject_id(), "patient-a");
        assert_eq!(verified.bindings().len(), 1);
        assert_eq!(verified.bindings()[0].fact_id(), "fact-1");
        assert_eq!(verified.bindings()[0].role(), SymthaeaEvidenceRoleV2::Supports);
    }

    #[test]
    fn wrong_patient_is_rejected() {
        let bound = fact("fact-1", "patient-a");
        let wire = fixture_bound_to_fact(&bound);
        let wrong = fact("fact-1", "patient-b");
        assert!(matches!(
            verify_symthaea_clinical_fact_bindings_v1(&wire, &[wrong], &policy()),
            Err(SymthaeaFactBindingError::FactSubjectMismatch(_))
                | Err(SymthaeaFactBindingError::FactSnapshotDigestMismatch(_))
        ));
    }

    #[test]
    fn missing_fact_is_rejected() {
        let fact = fact("fact-1", "patient-a");
        let wire = fixture_bound_to_fact(&fact);
        assert!(matches!(
            verify_symthaea_clinical_fact_bindings_v1(&wire, &[], &policy()),
            Err(SymthaeaFactBindingError::MissingFactInput(_))
        ));
    }

    #[test]
    fn unreferenced_extra_fact_is_rejected() {
        let primary = fact("fact-1", "patient-a");
        let wire = fixture_bound_to_fact(&primary);
        let extra = fact("fact-extra", "patient-a");
        assert!(matches!(
            verify_symthaea_clinical_fact_bindings_v1(&wire, &[primary, extra], &policy()),
            Err(SymthaeaFactBindingError::UnreferencedFactInput(_))
        ));
    }

    #[test]
    fn snapshot_substitution_is_rejected() {
        let original = fact("fact-1", "patient-a");
        let wire = fixture_bound_to_fact(&original);
        let mut substituted = original;
        substituted.effective_at_micros += 1;
        assert!(matches!(
            verify_symthaea_clinical_fact_bindings_v1(&wire, &[substituted], &policy()),
            Err(SymthaeaFactBindingError::FactSnapshotDigestMismatch(_))
        ));
    }

    #[test]
    fn subject_namespace_policy_is_explicit() {
        let fact = fact("fact-1", "patient-a");
        let wire = fixture_bound_to_fact(&fact);
        let mut wrong_policy = policy();
        wrong_policy.external_subject_namespace = "local/Patient".into();
        assert!(matches!(
            verify_symthaea_clinical_fact_bindings_v1(&wire, &[fact], &wrong_policy),
            Err(SymthaeaFactBindingError::SubjectNamespaceMismatch)
        ));
    }
}
