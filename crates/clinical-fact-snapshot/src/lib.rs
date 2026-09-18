#![deny(unsafe_code)]
//! Serializer-independent canonical identity for validated [`ClinicalFact`] snapshots.
//!
//! Clinical evidence must not depend on incidental JSON formatting, object-key order,
//! or serializer-version behavior. This crate therefore defines an explicit v1 byte
//! framing over the typed Mycelix clinical semantics kernel.
//!
//! The snapshot digest proves exact artifact identity only. It does not prove that a
//! fact is true, current, sufficient for a decision, or authorized for clinical use.

use mycelix_clinical_semantics::{
    ClinicalFact, ClinicalSemanticsError, ClinicalValue, CodeableConcept, Coding, FactProvenance,
    Quantity, Range, Ratio, SubjectRef, TransformationProvenance, Uncertainty,
};
use thiserror::Error;

/// Version of the canonical ClinicalFact snapshot framing.
pub const CLINICAL_FACT_SNAPSHOT_VERSION: u16 = 1;

const SNAPSHOT_TAG: &[u8] = b"mycelix/clinical-fact-snapshot/v1";
const DERIVE_KEY_CONTEXT: &str = "mycelix.health.clinical-fact-snapshot.v1";

/// Domain-separated identity of one exact validated ClinicalFact snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClinicalFactSnapshotDigestV1([u8; 32]);

impl ClinicalFactSnapshotDigestV1 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Encode one machine-actionable ClinicalFact using the exact v1 binary framing.
///
/// Framing rules are deliberately independent of serde:
///
/// - integers use big-endian bytes;
/// - `f64` values use exact IEEE-754 `to_bits()` bytes;
/// - strings/byte slices and vectors are length-prefixed;
/// - options have an explicit presence byte;
/// - enums have fixed one-byte discriminants;
/// - vector order is preserved as part of the exact snapshot.
pub fn clinical_fact_snapshot_bytes_v1(
    fact: &ClinicalFact,
) -> Result<Vec<u8>, ClinicalFactSnapshotError> {
    fact.validate_machine_actionable()?;

    let mut writer = CanonicalWriter::default();
    writer.u16(CLINICAL_FACT_SNAPSHOT_VERSION);
    writer.bytes(SNAPSHOT_TAG)?;
    encode_clinical_fact(&mut writer, fact)?;
    Ok(writer.finish())
}

/// Compute the domain-separated digest of one exact validated ClinicalFact snapshot.
pub fn clinical_fact_snapshot_digest_v1(
    fact: &ClinicalFact,
) -> Result<ClinicalFactSnapshotDigestV1, ClinicalFactSnapshotError> {
    let bytes = clinical_fact_snapshot_bytes_v1(fact)?;
    let mut hasher = blake3::Hasher::new_derive_key(DERIVE_KEY_CONTEXT);
    hasher.update(&CLINICAL_FACT_SNAPSHOT_VERSION.to_be_bytes());
    hasher.update(&(SNAPSHOT_TAG.len() as u16).to_be_bytes());
    hasher.update(SNAPSHOT_TAG);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    Ok(ClinicalFactSnapshotDigestV1(*hasher.finalize().as_bytes()))
}

/// Recompute and compare a claimed snapshot digest against the supplied typed fact.
pub fn verify_clinical_fact_snapshot_digest_v1(
    fact: &ClinicalFact,
    claimed: ClinicalFactSnapshotDigestV1,
) -> Result<ClinicalFactSnapshotDigestV1, ClinicalFactSnapshotError> {
    let computed = clinical_fact_snapshot_digest_v1(fact)?;
    if computed != claimed {
        return Err(ClinicalFactSnapshotError::DigestMismatch);
    }
    Ok(computed)
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

    fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
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

    fn bytes(&mut self, value: &[u8]) -> Result<(), ClinicalFactSnapshotError> {
        let len =
            u32::try_from(value.len()).map_err(|_| ClinicalFactSnapshotError::LengthOverflow)?;
        self.u32(len);
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), ClinicalFactSnapshotError> {
        self.bytes(value.as_bytes())
    }

    fn option_string(&mut self, value: Option<&str>) -> Result<(), ClinicalFactSnapshotError> {
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

    fn option_f64(&mut self, value: Option<f64>) {
        match value {
            Some(value) => {
                self.u8(1);
                self.f64(value);
            }
            None => self.u8(0),
        }
    }

    fn vector_len(&mut self, len: usize) -> Result<(), ClinicalFactSnapshotError> {
        self.u32(u32::try_from(len).map_err(|_| ClinicalFactSnapshotError::LengthOverflow)?);
        Ok(())
    }
}

fn encode_clinical_fact(
    writer: &mut CanonicalWriter,
    fact: &ClinicalFact,
) -> Result<(), ClinicalFactSnapshotError> {
    writer.string(&fact.fact_id)?;
    encode_subject(writer, &fact.subject)?;
    encode_codeable_concept(writer, &fact.concept)?;
    encode_clinical_value(writer, &fact.value)?;
    writer.i64(fact.effective_at_micros);
    encode_provenance(writer, &fact.provenance)?;
    encode_uncertainty_option(writer, fact.uncertainty.as_ref())?;
    Ok(())
}

fn encode_subject(
    writer: &mut CanonicalWriter,
    subject: &SubjectRef,
) -> Result<(), ClinicalFactSnapshotError> {
    writer.string(&subject.resource_type)?;
    writer.string(&subject.id)?;
    Ok(())
}

fn encode_codeable_concept(
    writer: &mut CanonicalWriter,
    concept: &CodeableConcept,
) -> Result<(), ClinicalFactSnapshotError> {
    writer.vector_len(concept.coding.len())?;
    for coding in &concept.coding {
        encode_coding(writer, coding)?;
    }
    writer.option_string(concept.text.as_deref())?;
    Ok(())
}

fn encode_coding(
    writer: &mut CanonicalWriter,
    coding: &Coding,
) -> Result<(), ClinicalFactSnapshotError> {
    writer.string(&coding.system)?;
    writer.string(&coding.code)?;
    writer.option_string(coding.display.as_deref())?;
    writer.option_string(coding.version.as_deref())?;
    Ok(())
}

fn encode_quantity(
    writer: &mut CanonicalWriter,
    quantity: &Quantity,
) -> Result<(), ClinicalFactSnapshotError> {
    writer.f64(quantity.value);
    writer.option_string(quantity.display_unit.as_deref())?;
    writer.string(&quantity.system)?;
    writer.string(&quantity.code)?;
    Ok(())
}

fn encode_quantity_option(
    writer: &mut CanonicalWriter,
    quantity: Option<&Quantity>,
) -> Result<(), ClinicalFactSnapshotError> {
    match quantity {
        Some(quantity) => {
            writer.u8(1);
            encode_quantity(writer, quantity)
        }
        None => {
            writer.u8(0);
            Ok(())
        }
    }
}

fn encode_range(
    writer: &mut CanonicalWriter,
    range: &Range,
) -> Result<(), ClinicalFactSnapshotError> {
    encode_quantity_option(writer, range.low.as_ref())?;
    encode_quantity_option(writer, range.high.as_ref())?;
    Ok(())
}

fn encode_ratio(
    writer: &mut CanonicalWriter,
    ratio: &Ratio,
) -> Result<(), ClinicalFactSnapshotError> {
    encode_quantity(writer, &ratio.numerator)?;
    encode_quantity(writer, &ratio.denominator)?;
    Ok(())
}

fn encode_clinical_value(
    writer: &mut CanonicalWriter,
    value: &ClinicalValue,
) -> Result<(), ClinicalFactSnapshotError> {
    match value {
        ClinicalValue::Quantity(value) => {
            writer.u8(0);
            encode_quantity(writer, value)?;
        }
        ClinicalValue::CodeableConcept(value) => {
            writer.u8(1);
            encode_codeable_concept(writer, value)?;
        }
        ClinicalValue::Range(value) => {
            writer.u8(2);
            encode_range(writer, value)?;
        }
        ClinicalValue::Ratio(value) => {
            writer.u8(3);
            encode_ratio(writer, value)?;
        }
        ClinicalValue::Boolean(value) => {
            writer.u8(4);
            writer.bool(*value);
        }
        ClinicalValue::Integer(value) => {
            writer.u8(5);
            writer.i64(*value);
        }
        ClinicalValue::Decimal(value) => {
            writer.u8(6);
            writer.f64(*value);
        }
        ClinicalValue::DateTimeMicros(value) => {
            writer.u8(7);
            writer.i64(*value);
        }
        ClinicalValue::Reference(value) => {
            writer.u8(8);
            encode_subject(writer, value)?;
        }
        ClinicalValue::Narrative {
            text,
            reason_not_typed,
        } => {
            // Current machine-actionable validation rejects Narrative before this
            // point. The tag is still reserved in v1 so a future validation change
            // cannot silently renumber subsequent variants.
            writer.u8(9);
            writer.string(text)?;
            writer.string(reason_not_typed)?;
        }
    }
    Ok(())
}

fn encode_provenance(
    writer: &mut CanonicalWriter,
    provenance: &FactProvenance,
) -> Result<(), ClinicalFactSnapshotError> {
    writer.string(&provenance.source_system)?;
    writer.string(&provenance.source_resource_type)?;
    writer.string(&provenance.source_resource_id)?;
    writer.option_string(provenance.source_version.as_deref())?;
    writer.i64(provenance.recorded_at_micros);
    writer.option_string(provenance.asserted_by.as_deref())?;
    encode_transformation_option(writer, provenance.transformation.as_ref())?;
    Ok(())
}

fn encode_transformation_option(
    writer: &mut CanonicalWriter,
    transformation: Option<&TransformationProvenance>,
) -> Result<(), ClinicalFactSnapshotError> {
    match transformation {
        Some(transformation) => {
            writer.u8(1);
            writer.string(&transformation.software)?;
            writer.string(&transformation.version)?;
            writer.string(&transformation.operation)?;
            writer.vector_len(transformation.input_fact_ids.len())?;
            for input in &transformation.input_fact_ids {
                writer.string(input)?;
            }
        }
        None => writer.u8(0),
    }
    Ok(())
}

fn encode_uncertainty_option(
    writer: &mut CanonicalWriter,
    uncertainty: Option<&Uncertainty>,
) -> Result<(), ClinicalFactSnapshotError> {
    match uncertainty {
        Some(uncertainty) => {
            writer.u8(1);
            writer.option_f64(uncertainty.confidence);
            writer.option_string(uncertainty.interpretation.as_deref())?;
        }
        None => writer.u8(0),
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ClinicalFactSnapshotError {
    #[error("clinical fact is not machine-actionable: {0}")]
    InvalidClinicalFact(#[from] ClinicalSemanticsError),
    #[error("canonical field length exceeds the v1 framing limit")]
    LengthOverflow,
    #[error("clinical fact snapshot digest mismatch")]
    DigestMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_semantics::{
        CodeableConcept, Coding, FactProvenance, Quantity, SubjectRef, TransformationProvenance,
        UCUM_SYSTEM,
    };

    fn fact() -> ClinicalFact {
        ClinicalFact {
            fact_id: "fact-hgb-1".into(),
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

    #[test]
    fn identical_fact_has_deterministic_snapshot_identity() {
        let left = fact();
        let right = left.clone();
        assert_eq!(
            clinical_fact_snapshot_bytes_v1(&left).unwrap(),
            clinical_fact_snapshot_bytes_v1(&right).unwrap()
        );
        assert_eq!(
            clinical_fact_snapshot_digest_v1(&left).unwrap(),
            clinical_fact_snapshot_digest_v1(&right).unwrap()
        );
    }

    #[test]
    fn subject_substitution_changes_identity() {
        let left = fact();
        let mut right = left.clone();
        right.subject.id = "patient-b".into();
        assert_ne!(
            clinical_fact_snapshot_digest_v1(&left).unwrap(),
            clinical_fact_snapshot_digest_v1(&right).unwrap()
        );
    }

    #[test]
    fn provenance_substitution_changes_identity() {
        let left = fact();
        let mut right = left.clone();
        right.provenance.source_resource_id = "obs-2".into();
        assert_ne!(
            clinical_fact_snapshot_digest_v1(&left).unwrap(),
            clinical_fact_snapshot_digest_v1(&right).unwrap()
        );
    }

    #[test]
    fn transformation_input_substitution_changes_identity() {
        let left = fact();
        let mut right = left.clone();
        right
            .provenance
            .transformation
            .as_mut()
            .unwrap()
            .input_fact_ids[0] = "different-input".into();
        assert_ne!(
            clinical_fact_snapshot_digest_v1(&left).unwrap(),
            clinical_fact_snapshot_digest_v1(&right).unwrap()
        );
    }

    #[test]
    fn uncertainty_substitution_changes_identity() {
        let left = fact();
        let mut right = left.clone();
        right.uncertainty.as_mut().unwrap().confidence = Some(0.97);
        assert_ne!(
            clinical_fact_snapshot_digest_v1(&left).unwrap(),
            clinical_fact_snapshot_digest_v1(&right).unwrap()
        );
    }

    #[test]
    fn narrative_fact_cannot_obtain_snapshot_identity() {
        let mut invalid = fact();
        invalid.value = ClinicalValue::Narrative {
            text: "free text".into(),
            reason_not_typed: "source lacked coding".into(),
        };
        assert!(matches!(
            clinical_fact_snapshot_digest_v1(&invalid),
            Err(ClinicalFactSnapshotError::InvalidClinicalFact(_))
        ));
    }

    #[test]
    fn non_ucum_quantity_cannot_obtain_snapshot_identity() {
        let mut invalid = fact();
        invalid.value = ClinicalValue::Quantity(Quantity {
            value: 13.7,
            display_unit: Some("g/dL".into()),
            system: "urn:not-ucum".into(),
            code: "g/dL".into(),
        });
        assert!(matches!(
            clinical_fact_snapshot_digest_v1(&invalid),
            Err(ClinicalFactSnapshotError::InvalidClinicalFact(_))
        ));
    }

    #[test]
    fn snapshot_digest_verification_detects_substitution() {
        let original = fact();
        let claimed = clinical_fact_snapshot_digest_v1(&original).unwrap();
        let mut substituted = original.clone();
        substituted.effective_at_micros += 1;
        assert!(matches!(
            verify_clinical_fact_snapshot_digest_v1(&substituted, claimed),
            Err(ClinicalFactSnapshotError::DigestMismatch)
        ));
    }

    #[test]
    fn canonical_framing_preserves_exact_float_bits() {
        let positive_zero = ClinicalFact {
            value: ClinicalValue::Decimal(0.0),
            ..fact()
        };
        let negative_zero = ClinicalFact {
            value: ClinicalValue::Decimal(-0.0),
            ..fact()
        };
        assert_ne!(
            clinical_fact_snapshot_digest_v1(&positive_zero).unwrap(),
            clinical_fact_snapshot_digest_v1(&negative_zero).unwrap()
        );
    }

    #[test]
    fn test_fixture_uses_ucum() {
        let ClinicalValue::Quantity(quantity) = &fact().value else {
            panic!("fixture must be quantity");
        };
        assert_eq!(quantity.system, UCUM_SYSTEM);
    }
}
