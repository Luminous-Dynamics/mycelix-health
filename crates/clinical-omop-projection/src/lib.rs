#![deny(unsafe_code)]
//! Research-only OMOP CDM 5.5 projection from validated Mycelix clinical facts.
//!
//! This crate deliberately does **not** infer OMOP vocabulary concept IDs from
//! source coding strings. Every concept, provenance type, unit, and person binding
//! must be supplied by an explicit versioned projection policy. Ambiguous or absent
//! mappings fail closed.
//!
//! V1 is intentionally narrow: it projects only quantitative [`ClinicalFact`]
//! values represented as strict UCUM [`Quantity`] values into an OMOP-like
//! MEASUREMENT research record. It does not create a database primary key, mutate an
//! OMOP database, establish cohort membership, diagnose disease, recommend treatment,
//! or grant clinical authority.

use std::collections::HashSet;

use mycelix_clinical_fact_snapshot::{
    clinical_fact_snapshot_digest_v1, ClinicalFactSnapshotError,
};
use mycelix_clinical_semantics::{ClinicalFact, ClinicalValue, Coding, SubjectRef, UCUM_SYSTEM};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Projection contract version.
pub const OMOP_PROJECTION_VERSION: u16 = 1;
/// OMOP CDM release supported by this projection contract.
pub const OMOP_CDM_VERSION: &str = "5.5";
const POLICY_TAG: &[u8] = b"mycelix/clinical-omop-projection-policy/v1";
const PROJECTION_TAG: &[u8] = b"mycelix/clinical-omop-measurement-projection/v1";
const POLICY_DERIVE_CONTEXT: &str = "mycelix.health.clinical-omop-projection-policy.v1";
const PROJECTION_DERIVE_CONTEXT: &str = "mycelix.health.clinical-omop-measurement-projection.v1";
const OMOP_SOURCE_VALUE_MAX_BYTES: usize = 50;

/// Exact source terminology key. `version` matching is exact: `None` only matches
/// a source Coding whose version is also `None`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceCodingKeyV1 {
    pub system: String,
    pub code: String,
    pub version: Option<String>,
}

impl SourceCodingKeyV1 {
    fn matches(&self, coding: &Coding) -> bool {
        self.system == coding.system && self.code == coding.code && self.version == coding.version
    }

    fn validate(&self) -> Result<(), OmopProjectionError> {
        if self.system.trim().is_empty() || self.code.trim().is_empty() {
            return Err(OmopProjectionError::InvalidSourceCoding);
        }
        if self.code.as_bytes().len() > OMOP_SOURCE_VALUE_MAX_BYTES {
            return Err(OmopProjectionError::SourceValueTooLong {
                value: self.code.clone(),
            });
        }
        if self.version.as_ref().is_some_and(|version| version.trim().is_empty()) {
            return Err(OmopProjectionError::InvalidSourceCoding);
        }
        Ok(())
    }
}

/// Explicit mapping from one UCUM code to one Standard Concept in the OMOP Unit
/// domain. The mapper is responsible for supplying a vocabulary-valid concept ID.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OmopUnitBindingV1 {
    pub ucum_code: String,
    pub unit_concept_id: i64,
}

impl OmopUnitBindingV1 {
    fn validate(&self) -> Result<(), OmopProjectionError> {
        if self.ucum_code.trim().is_empty() || self.unit_concept_id <= 0 {
            return Err(OmopProjectionError::InvalidUnitBinding);
        }
        if self.ucum_code.as_bytes().len() > OMOP_SOURCE_VALUE_MAX_BYTES {
            return Err(OmopProjectionError::SourceValueTooLong {
                value: self.ucum_code.clone(),
            });
        }
        Ok(())
    }
}

/// One explicit source-concept -> OMOP MEASUREMENT mapping.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OmopMeasurementConceptBindingV1 {
    pub source: SourceCodingKeyV1,
    /// Standard concept in the OMOP Measurement domain.
    pub measurement_concept_id: i64,
    /// Source concept ID in the loaded OMOP vocabulary, or 0 when the source code
    /// has no OMOP source concept mapping.
    pub measurement_source_concept_id: i64,
    /// OMOP Measurement Type Concept describing provenance.
    pub measurement_type_concept_id: i64,
    pub units: Vec<OmopUnitBindingV1>,
}

impl OmopMeasurementConceptBindingV1 {
    fn validate(&self) -> Result<(), OmopProjectionError> {
        self.source.validate()?;
        if self.measurement_concept_id <= 0
            || self.measurement_source_concept_id < 0
            || self.measurement_type_concept_id <= 0
            || self.units.is_empty()
        {
            return Err(OmopProjectionError::InvalidConceptBinding);
        }
        let mut units = HashSet::new();
        for unit in &self.units {
            unit.validate()?;
            if !units.insert(unit.ucum_code.as_str()) {
                return Err(OmopProjectionError::DuplicateUnitBinding {
                    code: unit.ucum_code.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Versioned, research-only mapping policy.
///
/// `vocabulary_snapshot_digest` is supplied by the deployment/research pipeline and
/// should identify the exact OHDSI vocabulary snapshot used to obtain the concept
/// IDs. This crate does not ship or license those vocabularies.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OmopMeasurementProjectionPolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    pub cdm_version: String,
    pub vocabulary_release_id: String,
    pub vocabulary_snapshot_digest: [u8; 32],
    pub concept_bindings: Vec<OmopMeasurementConceptBindingV1>,
}

impl OmopMeasurementProjectionPolicyV1 {
    pub fn validate(&self) -> Result<(), OmopProjectionError> {
        if self.schema_version != OMOP_PROJECTION_VERSION {
            return Err(OmopProjectionError::UnsupportedSchemaVersion {
                found: self.schema_version,
            });
        }
        if self.cdm_version != OMOP_CDM_VERSION {
            return Err(OmopProjectionError::UnsupportedCdmVersion {
                found: self.cdm_version.clone(),
            });
        }
        if self.policy_id.trim().is_empty()
            || self.vocabulary_release_id.trim().is_empty()
            || self.vocabulary_snapshot_digest == [0; 32]
            || self.concept_bindings.is_empty()
        {
            return Err(OmopProjectionError::InvalidPolicy);
        }

        let mut source_keys = HashSet::new();
        for binding in &self.concept_bindings {
            binding.validate()?;
            if !source_keys.insert(binding.source.clone()) {
                return Err(OmopProjectionError::DuplicateConceptBinding {
                    system: binding.source.system.clone(),
                    code: binding.source.code.clone(),
                });
            }
        }
        Ok(())
    }

    /// Serializer-independent domain-separated identity of this mapping policy.
    pub fn digest_v1(&self) -> Result<[u8; 32], OmopProjectionError> {
        self.validate()?;
        let mut bindings = self.concept_bindings.clone();
        bindings.sort_by(|left, right| left.source.cmp(&right.source));

        let mut writer = CanonicalWriter::default();
        writer.u16(self.schema_version);
        writer.string(&self.policy_id)?;
        writer.string(&self.cdm_version)?;
        writer.string(&self.vocabulary_release_id)?;
        writer.bytes32(&self.vocabulary_snapshot_digest);
        writer.len(bindings.len())?;
        for binding in bindings {
            encode_source_key(&mut writer, &binding.source)?;
            writer.i64(binding.measurement_concept_id);
            writer.i64(binding.measurement_source_concept_id);
            writer.i64(binding.measurement_type_concept_id);
            let mut units = binding.units.clone();
            units.sort();
            writer.len(units.len())?;
            for unit in units {
                writer.string(&unit.ucum_code)?;
                writer.i64(unit.unit_concept_id);
            }
        }
        Ok(domain_digest(POLICY_DERIVE_CONTEXT, POLICY_TAG, &writer.finish()))
    }
}

/// Explicit subject -> OMOP PERSON binding supplied by a research ETL boundary.
///
/// This crate does not derive `person_id` from patient identifiers. Deployments may
/// therefore use a privacy-preserving/pseudonymous PERSON mapping appropriate to the
/// research environment.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OmopPersonBindingV1 {
    pub subject: SubjectRef,
    pub person_id: i64,
    pub binding_namespace: String,
    pub binding_evidence_digest: [u8; 32],
}

impl OmopPersonBindingV1 {
    fn validate(&self) -> Result<(), OmopProjectionError> {
        if self.subject.resource_type.trim().is_empty()
            || self.subject.id.trim().is_empty()
            || self.person_id <= 0
            || self.binding_namespace.trim().is_empty()
            || self.binding_evidence_digest == [0; 32]
        {
            return Err(OmopProjectionError::InvalidPersonBinding);
        }
        Ok(())
    }
}

/// Research projection corresponding to the meaningful fields of an OMOP CDM 5.5
/// MEASUREMENT record. Database surrogate IDs, visit/provider joins, and physical
/// persistence remain responsibilities of a downstream ETL adapter.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OmopMeasurementProjectionV1 {
    pub schema_version: u16,
    pub authority: String,
    pub cdm_version: String,
    pub person_id: i64,
    pub measurement_concept_id: i64,
    pub measurement_type_concept_id: i64,
    pub value_as_number: f64,
    pub unit_concept_id: i64,
    pub measurement_datetime_micros: i64,
    pub measurement_source_value: String,
    pub measurement_source_concept_id: i64,
    pub unit_source_value: String,
    pub source_fact_id: String,
    pub source_fact_snapshot_digest: [u8; 32],
    pub projection_policy_digest: [u8; 32],
    pub vocabulary_snapshot_digest: [u8; 32],
    pub person_binding_namespace: String,
    pub person_binding_evidence_digest: [u8; 32],
}

impl OmopMeasurementProjectionV1 {
    /// Domain-separated identity of the exact research projection.
    pub fn digest_v1(&self) -> Result<[u8; 32], OmopProjectionError> {
        if self.authority != "ResearchProjectionOnly" || self.cdm_version != OMOP_CDM_VERSION {
            return Err(OmopProjectionError::InvalidProjection);
        }
        if !self.value_as_number.is_finite() {
            return Err(OmopProjectionError::InvalidProjection);
        }
        let mut writer = CanonicalWriter::default();
        writer.u16(self.schema_version);
        writer.string(&self.authority)?;
        writer.string(&self.cdm_version)?;
        writer.i64(self.person_id);
        writer.i64(self.measurement_concept_id);
        writer.i64(self.measurement_type_concept_id);
        writer.f64(self.value_as_number);
        writer.i64(self.unit_concept_id);
        writer.i64(self.measurement_datetime_micros);
        writer.string(&self.measurement_source_value)?;
        writer.i64(self.measurement_source_concept_id);
        writer.string(&self.unit_source_value)?;
        writer.string(&self.source_fact_id)?;
        writer.bytes32(&self.source_fact_snapshot_digest);
        writer.bytes32(&self.projection_policy_digest);
        writer.bytes32(&self.vocabulary_snapshot_digest);
        writer.string(&self.person_binding_namespace)?;
        writer.bytes32(&self.person_binding_evidence_digest);
        Ok(domain_digest(
            PROJECTION_DERIVE_CONTEXT,
            PROJECTION_TAG,
            &writer.finish(),
        ))
    }
}

/// Project one quantitative ClinicalFact into a research-only OMOP MEASUREMENT
/// representation.
pub fn project_measurement_v1(
    fact: &ClinicalFact,
    person: &OmopPersonBindingV1,
    policy: &OmopMeasurementProjectionPolicyV1,
) -> Result<OmopMeasurementProjectionV1, OmopProjectionError> {
    fact.validate_machine_actionable()?;
    person.validate()?;
    policy.validate()?;

    if fact.subject != person.subject {
        return Err(OmopProjectionError::SubjectMismatch);
    }

    let quantity = match &fact.value {
        ClinicalValue::Quantity(quantity) => quantity,
        _ => return Err(OmopProjectionError::UnsupportedClinicalValue),
    };
    if quantity.system != UCUM_SYSTEM {
        return Err(OmopProjectionError::NonUcumQuantity);
    }

    let mut matches = Vec::new();
    for binding in &policy.concept_bindings {
        for coding in &fact.concept.coding {
            if binding.source.matches(coding) {
                matches.push(binding);
            }
        }
    }
    let binding = match matches.as_slice() {
        [] => return Err(OmopProjectionError::MissingConceptMapping),
        [binding] => *binding,
        _ => return Err(OmopProjectionError::AmbiguousConceptMapping),
    };

    let unit_matches: Vec<&OmopUnitBindingV1> = binding
        .units
        .iter()
        .filter(|unit| unit.ucum_code == quantity.code)
        .collect();
    let unit = match unit_matches.as_slice() {
        [] => {
            return Err(OmopProjectionError::MissingUnitMapping {
                code: quantity.code.clone(),
            })
        }
        [unit] => *unit,
        _ => return Err(OmopProjectionError::AmbiguousUnitMapping),
    };

    let snapshot = clinical_fact_snapshot_digest_v1(fact)?;
    let policy_digest = policy.digest_v1()?;

    Ok(OmopMeasurementProjectionV1 {
        schema_version: OMOP_PROJECTION_VERSION,
        authority: "ResearchProjectionOnly".to_string(),
        cdm_version: OMOP_CDM_VERSION.to_string(),
        person_id: person.person_id,
        measurement_concept_id: binding.measurement_concept_id,
        measurement_type_concept_id: binding.measurement_type_concept_id,
        value_as_number: quantity.value,
        unit_concept_id: unit.unit_concept_id,
        measurement_datetime_micros: fact.effective_at_micros,
        measurement_source_value: binding.source.code.clone(),
        measurement_source_concept_id: binding.measurement_source_concept_id,
        unit_source_value: quantity.code.clone(),
        source_fact_id: fact.fact_id.clone(),
        source_fact_snapshot_digest: snapshot.into_bytes(),
        projection_policy_digest: policy_digest,
        vocabulary_snapshot_digest: policy.vocabulary_snapshot_digest,
        person_binding_namespace: person.binding_namespace.clone(),
        person_binding_evidence_digest: person.binding_evidence_digest,
    })
}

#[derive(Debug, Error)]
pub enum OmopProjectionError {
    #[error("clinical fact is not machine-actionable: {0}")]
    ClinicalSemantics(#[from] mycelix_clinical_semantics::ClinicalSemanticsError),
    #[error("clinical fact snapshot failed: {0}")]
    ClinicalFactSnapshot(#[from] ClinicalFactSnapshotError),
    #[error("unsupported projection schema version {found}")]
    UnsupportedSchemaVersion { found: u16 },
    #[error("unsupported OMOP CDM version {found}; v1 supports 5.5")]
    UnsupportedCdmVersion { found: String },
    #[error("projection policy is incomplete")]
    InvalidPolicy,
    #[error("source coding is incomplete or invalid")]
    InvalidSourceCoding,
    #[error("OMOP concept binding is invalid")]
    InvalidConceptBinding,
    #[error("OMOP unit binding is invalid")]
    InvalidUnitBinding,
    #[error("duplicate source concept binding for {system}|{code}")]
    DuplicateConceptBinding { system: String, code: String },
    #[error("duplicate UCUM unit binding for {code}")]
    DuplicateUnitBinding { code: String },
    #[error("source value exceeds OMOP v5.5 varchar(50) projection boundary: {value}")]
    SourceValueTooLong { value: String },
    #[error("OMOP person binding is invalid")]
    InvalidPersonBinding,
    #[error("ClinicalFact subject does not match OMOP person binding")]
    SubjectMismatch,
    #[error("v1 projects only quantitative ClinicalValue::Quantity facts")]
    UnsupportedClinicalValue,
    #[error("quantitative source fact is not UCUM-bound")]
    NonUcumQuantity,
    #[error("no explicit OMOP measurement concept mapping matched the fact")]
    MissingConceptMapping,
    #[error("more than one OMOP concept mapping matched the fact")]
    AmbiguousConceptMapping,
    #[error("no explicit OMOP unit mapping exists for UCUM code {code}")]
    MissingUnitMapping { code: String },
    #[error("more than one OMOP unit mapping matched the fact")]
    AmbiguousUnitMapping,
    #[error("projection is malformed")]
    InvalidProjection,
    #[error("canonical length overflow")]
    LengthOverflow,
}

#[derive(Default)]
struct CanonicalWriter {
    bytes: Vec<u8>,
}

impl CanonicalWriter {
    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn f64(&mut self, value: f64) {
        self.u64(value.to_bits());
    }

    fn bytes32(&mut self, value: &[u8; 32]) {
        self.bytes.extend_from_slice(value);
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), OmopProjectionError> {
        self.u32(u32::try_from(value.len()).map_err(|_| OmopProjectionError::LengthOverflow)?);
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), OmopProjectionError> {
        self.bytes(value.as_bytes())
    }

    fn option_string(&mut self, value: Option<&str>) -> Result<(), OmopProjectionError> {
        match value {
            Some(value) => {
                self.u8(1);
                self.string(value)
            }
            None => {
                self.u8(0);
                Ok(())
            }
        }
    }

    fn len(&mut self, value: usize) -> Result<(), OmopProjectionError> {
        self.u32(u32::try_from(value).map_err(|_| OmopProjectionError::LengthOverflow)?);
        Ok(())
    }
}

fn encode_source_key(
    writer: &mut CanonicalWriter,
    key: &SourceCodingKeyV1,
) -> Result<(), OmopProjectionError> {
    writer.string(&key.system)?;
    writer.string(&key.code)?;
    writer.option_string(key.version.as_deref())?;
    Ok(())
}

fn domain_digest(context: &str, tag: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(&(tag.len() as u16).to_be_bytes());
    hasher.update(tag);
    hasher.update(&(payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_semantics::{
        CodeableConcept, FactProvenance, Quantity, Uncertainty,
    };

    fn fact() -> ClinicalFact {
        ClinicalFact {
            fact_id: "fact-lab-1".into(),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: "patient-1".into(),
            },
            concept: CodeableConcept {
                coding: vec![Coding {
                    system: "urn:example:lab".into(),
                    code: "analyte-a".into(),
                    display: Some("Fixture analyte".into()),
                    version: Some("2026-09".into()),
                }],
                text: None,
            },
            value: ClinicalValue::Quantity(Quantity::ucum(13.2, "g/dL")),
            effective_at_micros: 1_700_000_000_000_000,
            provenance: FactProvenance {
                source_system: "fixture-ehr".into(),
                source_resource_type: "Observation".into(),
                source_resource_id: "obs-1".into(),
                source_version: Some("v1".into()),
                recorded_at_micros: 1_700_000_001_000_000,
                asserted_by: Some("fixture-lab".into()),
                transformation: None,
            },
            uncertainty: Some(Uncertainty {
                confidence: None,
                interpretation: Some("fixture only".into()),
            }),
        }
    }

    fn person() -> OmopPersonBindingV1 {
        OmopPersonBindingV1 {
            subject: fact().subject,
            person_id: 42,
            binding_namespace: "research-cohort-fixture/v1".into(),
            binding_evidence_digest: [3; 32],
        }
    }

    fn policy() -> OmopMeasurementProjectionPolicyV1 {
        OmopMeasurementProjectionPolicyV1 {
            schema_version: OMOP_PROJECTION_VERSION,
            policy_id: "fixture-omop-55/v1".into(),
            cdm_version: OMOP_CDM_VERSION.into(),
            vocabulary_release_id: "fixture-vocab-2026-09".into(),
            vocabulary_snapshot_digest: [7; 32],
            concept_bindings: vec![OmopMeasurementConceptBindingV1 {
                source: SourceCodingKeyV1 {
                    system: "urn:example:lab".into(),
                    code: "analyte-a".into(),
                    version: Some("2026-09".into()),
                },
                measurement_concept_id: 1001,
                measurement_source_concept_id: 1002,
                measurement_type_concept_id: 1003,
                units: vec![OmopUnitBindingV1 {
                    ucum_code: "g/dL".into(),
                    unit_concept_id: 2001,
                }],
            }],
        }
    }

    #[test]
    fn exact_mapping_projects_research_measurement() {
        let projection = project_measurement_v1(&fact(), &person(), &policy()).unwrap();
        assert_eq!(projection.authority, "ResearchProjectionOnly");
        assert_eq!(projection.cdm_version, "5.5");
        assert_eq!(projection.person_id, 42);
        assert_eq!(projection.measurement_concept_id, 1001);
        assert_eq!(projection.unit_concept_id, 2001);
        assert_eq!(projection.value_as_number, 13.2);
        assert_ne!(projection.digest_v1().unwrap(), [0; 32]);
    }

    #[test]
    fn concept_version_drift_fails_closed() {
        let mut changed = fact();
        changed.concept.coding[0].version = Some("2026-10".into());
        assert!(matches!(
            project_measurement_v1(&changed, &person(), &policy()),
            Err(OmopProjectionError::MissingConceptMapping)
        ));
    }

    #[test]
    fn ambiguous_concept_mapping_fails_closed() {
        let mut changed = fact();
        changed.concept.coding.push(changed.concept.coding[0].clone());
        assert!(matches!(
            project_measurement_v1(&changed, &person(), &policy()),
            Err(OmopProjectionError::AmbiguousConceptMapping)
        ));
    }

    #[test]
    fn unit_substitution_fails_closed() {
        let mut changed = fact();
        changed.value = ClinicalValue::Quantity(Quantity::ucum(13.2, "mg/dL"));
        assert!(matches!(
            project_measurement_v1(&changed, &person(), &policy()),
            Err(OmopProjectionError::MissingUnitMapping { .. })
        ));
    }

    #[test]
    fn patient_rebinding_fails_closed() {
        let mut binding = person();
        binding.subject.id = "patient-2".into();
        assert!(matches!(
            project_measurement_v1(&fact(), &binding, &policy()),
            Err(OmopProjectionError::SubjectMismatch)
        ));
    }

    #[test]
    fn non_quantity_fact_is_not_coerced() {
        let mut changed = fact();
        changed.value = ClinicalValue::Integer(13);
        assert!(matches!(
            project_measurement_v1(&changed, &person(), &policy()),
            Err(OmopProjectionError::UnsupportedClinicalValue)
        ));
    }

    #[test]
    fn fact_mutation_changes_projection_identity() {
        let baseline = project_measurement_v1(&fact(), &person(), &policy())
            .unwrap()
            .digest_v1()
            .unwrap();
        let mut changed = fact();
        changed.value = ClinicalValue::Quantity(Quantity::ucum(13.3, "g/dL"));
        let mutated = project_measurement_v1(&changed, &person(), &policy())
            .unwrap()
            .digest_v1()
            .unwrap();
        assert_ne!(baseline, mutated);
    }

    #[test]
    fn vocabulary_snapshot_changes_policy_and_projection_identity() {
        let baseline_policy = policy();
        let baseline_projection = project_measurement_v1(&fact(), &person(), &baseline_policy)
            .unwrap()
            .digest_v1()
            .unwrap();
        let mut changed_policy = baseline_policy.clone();
        changed_policy.vocabulary_snapshot_digest = [8; 32];
        assert_ne!(
            baseline_policy.digest_v1().unwrap(),
            changed_policy.digest_v1().unwrap()
        );
        let changed_projection = project_measurement_v1(&fact(), &person(), &changed_policy)
            .unwrap()
            .digest_v1()
            .unwrap();
        assert_ne!(baseline_projection, changed_projection);
    }

    #[test]
    fn cdm_version_is_pinned() {
        let mut changed = policy();
        changed.cdm_version = "6.0".into();
        assert!(matches!(
            changed.validate(),
            Err(OmopProjectionError::UnsupportedCdmVersion { .. })
        ));
    }
}
