#![deny(unsafe_code)]
//! Evidence-grade laboratory result semantics for Mycelix Health.
//!
//! A laboratory result is more than a code, number, and unit. This crate binds
//! analyte terminology, specimen context, assay/method identity, reference
//! intervals, analytical limits, provenance, uncertainty, correction lineage,
//! and a serializer-independent artifact identity.
//!
//! The semantic core is intentionally terminology-release agnostic. Adapters and
//! policies must provide the exact LOINC release/version they accept.

use mycelix_clinical_semantics::{
    ClinicalFact, ClinicalSemanticsError, ClinicalValue, CodeableConcept, Coding, FactProvenance,
    Quantity, SubjectRef, TransformationProvenance, Uncertainty, UCUM_SYSTEM,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const LABORATORY_RESULT_VERSION: u16 = 1;
pub const LOINC_SYSTEM: &str = "http://loinc.org";

const SNAPSHOT_TAG: &[u8] = b"mycelix/clinical-laboratory-result/v1";
const DERIVE_KEY_CONTEXT: &str = "mycelix.health.clinical-laboratory-result.v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MissingReasonV1 {
    NotProvided,
    Unknown,
    NotApplicable,
    Redacted,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EvidenceFieldV1<T> {
    Known(T),
    Missing(MissingReasonV1),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminologyCodeV1 {
    pub system: String,
    pub code: String,
    pub version: String,
    pub display: Option<String>,
}

impl TerminologyCodeV1 {
    fn validate(&self) -> Result<(), LaboratorySemanticError> {
        if self.system.trim().is_empty() {
            return Err(LaboratorySemanticError::MissingTerminologySystem);
        }
        if self.code.trim().is_empty() {
            return Err(LaboratorySemanticError::MissingTerminologyCode);
        }
        if self.version.trim().is_empty() {
            return Err(LaboratorySemanticError::MissingTerminologyVersion);
        }
        Ok(())
    }

    fn as_codeable_concept(&self) -> CodeableConcept {
        CodeableConcept {
            coding: vec![Coding {
                system: self.system.clone(),
                code: self.code.clone(),
                display: self.display.clone(),
                version: Some(self.version.clone()),
            }],
            text: self.display.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LaboratoryResultStatusV1 {
    Registered,
    Partial,
    Preliminary,
    Final,
    Amended,
    Corrected,
    Appended,
    Cancelled,
    EnteredInError,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LaboratoryValueV1 {
    Quantity(Quantity),
    Coded(CodeableConcept),
}

impl LaboratoryValueV1 {
    fn validate(&self) -> Result<(), LaboratorySemanticError> {
        match self {
            Self::Quantity(quantity) => quantity.validate_machine_actionable()?,
            Self::Coded(concept) => concept.validate_machine_actionable()?,
        }
        Ok(())
    }

    fn as_clinical_value(&self) -> ClinicalValue {
        match self {
            Self::Quantity(value) => ClinicalValue::Quantity(value.clone()),
            Self::Coded(value) => ClinicalValue::CodeableConcept(value.clone()),
        }
    }

    fn quantity(&self) -> Option<&Quantity> {
        match self {
            Self::Quantity(value) => Some(value),
            Self::Coded(_) => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpecimenIdentityV1 {
    pub specimen_id: String,
    pub specimen_type: CodeableConcept,
    pub collected_at_micros: Option<i64>,
    pub received_at_micros: Option<i64>,
}

impl SpecimenIdentityV1 {
    fn validate(&self) -> Result<(), LaboratorySemanticError> {
        if self.specimen_id.trim().is_empty() {
            return Err(LaboratorySemanticError::MissingSpecimenId);
        }
        self.specimen_type.validate_machine_actionable()?;
        if let (Some(collected), Some(received)) =
            (self.collected_at_micros, self.received_at_micros)
        {
            if collected > received {
                return Err(LaboratorySemanticError::InvalidLaboratoryTimeline);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AssayMethodV1 {
    pub method: CodeableConcept,
    pub assay_name: Option<String>,
    pub manufacturer: Option<String>,
    pub analyzer_model: Option<String>,
    pub lot_or_reagent_id: Option<String>,
}

impl AssayMethodV1 {
    fn validate(&self) -> Result<(), LaboratorySemanticError> {
        self.method.validate_machine_actionable()?;
        if self
            .assay_name
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
            || self
                .manufacturer
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            || self
                .analyzer_model
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            || self
                .lot_or_reagent_id
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(LaboratorySemanticError::EmptyOptionalIdentityField);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceArtifactIdentityV1 {
    pub namespace: String,
    pub artifact_id: String,
    pub version: String,
    pub digest: [u8; 32],
}

impl EvidenceArtifactIdentityV1 {
    fn validate(&self) -> Result<(), LaboratorySemanticError> {
        if self.namespace.trim().is_empty()
            || self.artifact_id.trim().is_empty()
            || self.version.trim().is_empty()
        {
            return Err(LaboratorySemanticError::IncompleteEvidenceIdentity);
        }
        if self.digest == [0; 32] {
            return Err(LaboratorySemanticError::ZeroEvidenceDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReferencePopulationV1 {
    pub description: String,
    pub source: EvidenceArtifactIdentityV1,
}

impl ReferencePopulationV1 {
    fn validate(&self) -> Result<(), LaboratorySemanticError> {
        if self.description.trim().is_empty() {
            return Err(LaboratorySemanticError::MissingReferencePopulationContext);
        }
        self.source.validate()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReferenceIntervalV1 {
    pub low: Option<Quantity>,
    pub high: Option<Quantity>,
    pub population: ReferencePopulationV1,
}

impl ReferenceIntervalV1 {
    fn validate(&self, result_quantity: Option<&Quantity>) -> Result<(), LaboratorySemanticError> {
        if self.low.is_none() && self.high.is_none() {
            return Err(LaboratorySemanticError::EmptyReferenceInterval);
        }
        self.population.validate()?;
        if let Some(low) = &self.low {
            low.validate_machine_actionable()?;
        }
        if let Some(high) = &self.high {
            high.validate_machine_actionable()?;
        }
        if let (Some(low), Some(high)) = (&self.low, &self.high) {
            ensure_same_unit(low, high)?;
            if low.value > high.value {
                return Err(LaboratorySemanticError::InvertedReferenceInterval);
            }
        }
        if let Some(result) = result_quantity {
            if let Some(low) = &self.low {
                ensure_same_unit(result, low)?;
            }
            if let Some(high) = &self.high {
                ensure_same_unit(result, high)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DetectionLimitsV1 {
    pub lower_detection: Option<Quantity>,
    pub lower_quantification: Option<Quantity>,
    pub upper_quantification: Option<Quantity>,
    pub upper_detection: Option<Quantity>,
}

impl DetectionLimitsV1 {
    fn validate(&self, result_quantity: &Quantity) -> Result<(), LaboratorySemanticError> {
        let limits = [
            self.lower_detection.as_ref(),
            self.lower_quantification.as_ref(),
            self.upper_quantification.as_ref(),
            self.upper_detection.as_ref(),
        ];
        if limits.iter().all(Option::is_none) {
            return Err(LaboratorySemanticError::EmptyDetectionLimits);
        }
        for limit in limits.into_iter().flatten() {
            limit.validate_machine_actionable()?;
            ensure_same_unit(result_quantity, limit)?;
        }
        if let (Some(lod), Some(loq)) = (&self.lower_detection, &self.lower_quantification) {
            if lod.value > loq.value {
                return Err(LaboratorySemanticError::InvalidDetectionLimitOrdering);
            }
        }
        if let (Some(uoq), Some(uod)) = (&self.upper_quantification, &self.upper_detection) {
            if uoq.value > uod.value {
                return Err(LaboratorySemanticError::InvalidDetectionLimitOrdering);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InterpretationFlagV1 {
    pub interpretation: CodeableConcept,
    pub source: String,
}

impl InterpretationFlagV1 {
    fn validate(&self) -> Result<(), LaboratorySemanticError> {
        self.interpretation.validate_machine_actionable()?;
        if self.source.trim().is_empty() {
            return Err(LaboratorySemanticError::MissingInterpretationSource);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PriorLaboratoryResultV1 {
    pub result_id: String,
    pub snapshot_digest: [u8; 32],
}

impl PriorLaboratoryResultV1 {
    fn validate(&self, current_result_id: &str) -> Result<(), LaboratorySemanticError> {
        if self.result_id.trim().is_empty() || self.result_id == current_result_id {
            return Err(LaboratorySemanticError::InvalidSupersededResult);
        }
        if self.snapshot_digest == [0; 32] {
            return Err(LaboratorySemanticError::ZeroEvidenceDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LaboratoryProvenanceV1 {
    pub source_system: String,
    pub source_resource_id: String,
    pub source_version: Option<String>,
    pub laboratory_id: String,
    pub accession_id: Option<String>,
    pub analyzed_at_micros: Option<i64>,
    pub resulted_at_micros: i64,
    pub performer: Option<String>,
    pub transformation: Option<TransformationProvenance>,
}

impl LaboratoryProvenanceV1 {
    fn validate(&self) -> Result<(), LaboratorySemanticError> {
        if self.source_system.trim().is_empty()
            || self.source_resource_id.trim().is_empty()
            || self.laboratory_id.trim().is_empty()
        {
            return Err(LaboratorySemanticError::IncompleteLaboratoryProvenance);
        }
        if let Some(analyzed) = self.analyzed_at_micros {
            if analyzed > self.resulted_at_micros {
                return Err(LaboratorySemanticError::InvalidLaboratoryTimeline);
            }
        }
        if self
            .accession_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
            || self
                .performer
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(LaboratorySemanticError::EmptyOptionalIdentityField);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LaboratoryResultV1 {
    pub schema_version: u16,
    pub result_id: String,
    pub subject: SubjectRef,
    pub analyte: TerminologyCodeV1,
    pub value: LaboratoryValueV1,
    pub status: LaboratoryResultStatusV1,
    pub specimen: EvidenceFieldV1<SpecimenIdentityV1>,
    pub assay: EvidenceFieldV1<AssayMethodV1>,
    pub reference_intervals: Vec<ReferenceIntervalV1>,
    pub detection_limits: Option<DetectionLimitsV1>,
    pub interpretations: Vec<InterpretationFlagV1>,
    pub provenance: LaboratoryProvenanceV1,
    pub uncertainty: Option<Uncertainty>,
    pub supersedes: Option<PriorLaboratoryResultV1>,
}

impl LaboratoryResultV1 {
    pub fn validate(&self) -> Result<(), LaboratorySemanticError> {
        if self.schema_version != LABORATORY_RESULT_VERSION {
            return Err(LaboratorySemanticError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.result_id.trim().is_empty() {
            return Err(LaboratorySemanticError::MissingResultId);
        }
        if self.subject.resource_type.trim().is_empty() || self.subject.id.trim().is_empty() {
            return Err(LaboratorySemanticError::InvalidSubject);
        }
        self.analyte.validate()?;
        if self.analyte.system != LOINC_SYSTEM {
            return Err(LaboratorySemanticError::AnalyteMustUseLoinc);
        }
        self.value.validate()?;
        match &self.specimen {
            EvidenceFieldV1::Known(specimen) => specimen.validate()?,
            EvidenceFieldV1::Missing(_) => {}
        }
        match &self.assay {
            EvidenceFieldV1::Known(assay) => assay.validate()?,
            EvidenceFieldV1::Missing(_) => {}
        }
        for interval in &self.reference_intervals {
            interval.validate(self.value.quantity())?;
        }
        if let Some(limits) = &self.detection_limits {
            let quantity = self
                .value
                .quantity()
                .ok_or(LaboratorySemanticError::DetectionLimitsRequireQuantity)?;
            limits.validate(quantity)?;
        }
        for interpretation in &self.interpretations {
            interpretation.validate()?;
        }
        self.provenance.validate()?;
        if let EvidenceFieldV1::Known(specimen) = &self.specimen {
            if let Some(collected) = specimen.collected_at_micros {
                if collected > self.provenance.resulted_at_micros {
                    return Err(LaboratorySemanticError::InvalidLaboratoryTimeline);
                }
                if let Some(analyzed) = self.provenance.analyzed_at_micros {
                    if collected > analyzed {
                        return Err(LaboratorySemanticError::InvalidLaboratoryTimeline);
                    }
                }
            }
            if let (Some(received), Some(analyzed)) =
                (specimen.received_at_micros, self.provenance.analyzed_at_micros)
            {
                if received > analyzed {
                    return Err(LaboratorySemanticError::InvalidLaboratoryTimeline);
                }
            }
        }
        if let Some(uncertainty) = &self.uncertainty {
            if let Some(confidence) = uncertainty.confidence {
                if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
                    return Err(LaboratorySemanticError::InvalidConfidence);
                }
            }
        }
        match self.status {
            LaboratoryResultStatusV1::Amended
            | LaboratoryResultStatusV1::Corrected
            | LaboratoryResultStatusV1::Appended => {
                let prior = self
                    .supersedes
                    .as_ref()
                    .ok_or(LaboratorySemanticError::CorrectionRequiresSupersession)?;
                prior.validate(&self.result_id)?;
            }
            _ => {
                if let Some(prior) = &self.supersedes {
                    prior.validate(&self.result_id)?;
                }
            }
        }
        Ok(())
    }

    pub fn to_clinical_fact(&self) -> Result<LaboratoryClinicalFactProjectionV1, LaboratorySemanticError> {
        self.validate()?;
        let effective_at_micros = match &self.specimen {
            EvidenceFieldV1::Known(specimen) => specimen
                .collected_at_micros
                .or(self.provenance.analyzed_at_micros)
                .unwrap_or(self.provenance.resulted_at_micros),
            EvidenceFieldV1::Missing(_) => self
                .provenance
                .analyzed_at_micros
                .unwrap_or(self.provenance.resulted_at_micros),
        };
        let fact = ClinicalFact {
            fact_id: self.result_id.clone(),
            subject: self.subject.clone(),
            concept: self.analyte.as_codeable_concept(),
            value: self.value.as_clinical_value(),
            effective_at_micros,
            provenance: FactProvenance {
                source_system: self.provenance.source_system.clone(),
                source_resource_type: "LaboratoryResult".to_string(),
                source_resource_id: self.provenance.source_resource_id.clone(),
                source_version: self.provenance.source_version.clone(),
                recorded_at_micros: self.provenance.resulted_at_micros,
                asserted_by: self.provenance.performer.clone(),
                transformation: self.provenance.transformation.clone(),
            },
            uncertainty: self.uncertainty.clone(),
        };
        fact.validate_machine_actionable()?;
        Ok(LaboratoryClinicalFactProjectionV1 {
            fact,
            laboratory_snapshot_digest: laboratory_result_snapshot_digest_v1(self)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LaboratoryClinicalFactProjectionV1 {
    pub fact: ClinicalFact,
    pub laboratory_snapshot_digest: LaboratoryResultSnapshotDigestV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LaboratoryResultSnapshotDigestV1([u8; 32]);

impl LaboratoryResultSnapshotDigestV1 {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

pub fn laboratory_result_snapshot_digest_v1(
    result: &LaboratoryResultV1,
) -> Result<LaboratoryResultSnapshotDigestV1, LaboratorySemanticError> {
    let bytes = laboratory_result_snapshot_bytes_v1(result)?;
    let mut hasher = blake3::Hasher::new_derive_key(DERIVE_KEY_CONTEXT);
    hasher.update(&LABORATORY_RESULT_VERSION.to_be_bytes());
    hasher.update(&(SNAPSHOT_TAG.len() as u16).to_be_bytes());
    hasher.update(SNAPSHOT_TAG);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    Ok(LaboratoryResultSnapshotDigestV1(*hasher.finalize().as_bytes()))
}

pub fn laboratory_result_snapshot_bytes_v1(
    result: &LaboratoryResultV1,
) -> Result<Vec<u8>, LaboratorySemanticError> {
    result.validate()?;
    let mut writer = CanonicalWriter::default();
    writer.u16(LABORATORY_RESULT_VERSION);
    writer.bytes(SNAPSHOT_TAG)?;
    encode_result(&mut writer, result)?;
    Ok(writer.finish())
}

fn ensure_same_unit(left: &Quantity, right: &Quantity) -> Result<(), LaboratorySemanticError> {
    if left.system != UCUM_SYSTEM
        || right.system != UCUM_SYSTEM
        || left.system != right.system
        || left.code != right.code
    {
        return Err(LaboratorySemanticError::IncompatibleUnits);
    }
    Ok(())
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

    fn bytes(&mut self, value: &[u8]) -> Result<(), LaboratorySemanticError> {
        let len = u32::try_from(value.len()).map_err(|_| LaboratorySemanticError::LengthOverflow)?;
        self.u32(len);
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), LaboratorySemanticError> {
        self.bytes(value.as_bytes())
    }

    fn option_string(&mut self, value: Option<&str>) -> Result<(), LaboratorySemanticError> {
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

    fn option_i64(&mut self, value: Option<i64>) {
        match value {
            Some(value) => {
                self.u8(1);
                self.i64(value);
            }
            None => self.u8(0),
        }
    }

    fn vector_len(&mut self, len: usize) -> Result<(), LaboratorySemanticError> {
        self.u32(u32::try_from(len).map_err(|_| LaboratorySemanticError::LengthOverflow)?);
        Ok(())
    }
}

fn encode_result(writer: &mut CanonicalWriter, result: &LaboratoryResultV1) -> Result<(), LaboratorySemanticError> {
    writer.string(&result.result_id)?;
    encode_subject(writer, &result.subject)?;
    encode_terminology(writer, &result.analyte)?;
    encode_value(writer, &result.value)?;
    writer.u8(result.status as u8);
    encode_evidence_field_specimen(writer, &result.specimen)?;
    encode_evidence_field_assay(writer, &result.assay)?;
    writer.vector_len(result.reference_intervals.len())?;
    for interval in &result.reference_intervals {
        encode_reference_interval(writer, interval)?;
    }
    match &result.detection_limits {
        Some(limits) => {
            writer.u8(1);
            encode_detection_limits(writer, limits)?;
        }
        None => writer.u8(0),
    }
    writer.vector_len(result.interpretations.len())?;
    for interpretation in &result.interpretations {
        encode_interpretation(writer, interpretation)?;
    }
    encode_provenance(writer, &result.provenance)?;
    encode_uncertainty(writer, result.uncertainty.as_ref())?;
    match &result.supersedes {
        Some(prior) => {
            writer.u8(1);
            writer.string(&prior.result_id)?;
            writer.bytes(&prior.snapshot_digest)?;
        }
        None => writer.u8(0),
    }
    Ok(())
}

fn encode_subject(writer: &mut CanonicalWriter, subject: &SubjectRef) -> Result<(), LaboratorySemanticError> {
    writer.string(&subject.resource_type)?;
    writer.string(&subject.id)?;
    Ok(())
}

fn encode_terminology(writer: &mut CanonicalWriter, value: &TerminologyCodeV1) -> Result<(), LaboratorySemanticError> {
    writer.string(&value.system)?;
    writer.string(&value.code)?;
    writer.string(&value.version)?;
    writer.option_string(value.display.as_deref())?;
    Ok(())
}

fn encode_value(writer: &mut CanonicalWriter, value: &LaboratoryValueV1) -> Result<(), LaboratorySemanticError> {
    match value {
        LaboratoryValueV1::Quantity(quantity) => {
            writer.u8(0);
            encode_quantity(writer, quantity)?;
        }
        LaboratoryValueV1::Coded(concept) => {
            writer.u8(1);
            encode_concept(writer, concept)?;
        }
    }
    Ok(())
}

fn encode_quantity(writer: &mut CanonicalWriter, value: &Quantity) -> Result<(), LaboratorySemanticError> {
    writer.f64(value.value);
    writer.option_string(value.display_unit.as_deref())?;
    writer.string(&value.system)?;
    writer.string(&value.code)?;
    Ok(())
}

fn encode_concept(writer: &mut CanonicalWriter, value: &CodeableConcept) -> Result<(), LaboratorySemanticError> {
    writer.vector_len(value.coding.len())?;
    for coding in &value.coding {
        writer.string(&coding.system)?;
        writer.string(&coding.code)?;
        writer.option_string(coding.display.as_deref())?;
        writer.option_string(coding.version.as_deref())?;
    }
    writer.option_string(value.text.as_deref())?;
    Ok(())
}

fn encode_missing(writer: &mut CanonicalWriter, value: MissingReasonV1) {
    writer.u8(value as u8);
}

fn encode_evidence_field_specimen(
    writer: &mut CanonicalWriter,
    value: &EvidenceFieldV1<SpecimenIdentityV1>,
) -> Result<(), LaboratorySemanticError> {
    match value {
        EvidenceFieldV1::Known(specimen) => {
            writer.u8(1);
            writer.string(&specimen.specimen_id)?;
            encode_concept(writer, &specimen.specimen_type)?;
            writer.option_i64(specimen.collected_at_micros);
            writer.option_i64(specimen.received_at_micros);
        }
        EvidenceFieldV1::Missing(reason) => {
            writer.u8(0);
            encode_missing(writer, *reason);
        }
    }
    Ok(())
}

fn encode_evidence_field_assay(
    writer: &mut CanonicalWriter,
    value: &EvidenceFieldV1<AssayMethodV1>,
) -> Result<(), LaboratorySemanticError> {
    match value {
        EvidenceFieldV1::Known(assay) => {
            writer.u8(1);
            encode_concept(writer, &assay.method)?;
            writer.option_string(assay.assay_name.as_deref())?;
            writer.option_string(assay.manufacturer.as_deref())?;
            writer.option_string(assay.analyzer_model.as_deref())?;
            writer.option_string(assay.lot_or_reagent_id.as_deref())?;
        }
        EvidenceFieldV1::Missing(reason) => {
            writer.u8(0);
            encode_missing(writer, *reason);
        }
    }
    Ok(())
}

fn encode_artifact(writer: &mut CanonicalWriter, value: &EvidenceArtifactIdentityV1) -> Result<(), LaboratorySemanticError> {
    writer.string(&value.namespace)?;
    writer.string(&value.artifact_id)?;
    writer.string(&value.version)?;
    writer.bytes(&value.digest)?;
    Ok(())
}

fn encode_reference_interval(writer: &mut CanonicalWriter, value: &ReferenceIntervalV1) -> Result<(), LaboratorySemanticError> {
    encode_optional_quantity(writer, value.low.as_ref())?;
    encode_optional_quantity(writer, value.high.as_ref())?;
    writer.string(&value.population.description)?;
    encode_artifact(writer, &value.population.source)?;
    Ok(())
}

fn encode_optional_quantity(writer: &mut CanonicalWriter, value: Option<&Quantity>) -> Result<(), LaboratorySemanticError> {
    match value {
        Some(value) => {
            writer.u8(1);
            encode_quantity(writer, value)?;
        }
        None => writer.u8(0),
    }
    Ok(())
}

fn encode_detection_limits(writer: &mut CanonicalWriter, value: &DetectionLimitsV1) -> Result<(), LaboratorySemanticError> {
    encode_optional_quantity(writer, value.lower_detection.as_ref())?;
    encode_optional_quantity(writer, value.lower_quantification.as_ref())?;
    encode_optional_quantity(writer, value.upper_quantification.as_ref())?;
    encode_optional_quantity(writer, value.upper_detection.as_ref())?;
    Ok(())
}

fn encode_interpretation(writer: &mut CanonicalWriter, value: &InterpretationFlagV1) -> Result<(), LaboratorySemanticError> {
    encode_concept(writer, &value.interpretation)?;
    writer.string(&value.source)?;
    Ok(())
}

fn encode_provenance(writer: &mut CanonicalWriter, value: &LaboratoryProvenanceV1) -> Result<(), LaboratorySemanticError> {
    writer.string(&value.source_system)?;
    writer.string(&value.source_resource_id)?;
    writer.option_string(value.source_version.as_deref())?;
    writer.string(&value.laboratory_id)?;
    writer.option_string(value.accession_id.as_deref())?;
    writer.option_i64(value.analyzed_at_micros);
    writer.i64(value.resulted_at_micros);
    writer.option_string(value.performer.as_deref())?;
    match &value.transformation {
        Some(transformation) => {
            writer.u8(1);
            writer.string(&transformation.software)?;
            writer.string(&transformation.version)?;
            writer.string(&transformation.operation)?;
            writer.vector_len(transformation.input_fact_ids.len())?;
            for id in &transformation.input_fact_ids {
                writer.string(id)?;
            }
        }
        None => writer.u8(0),
    }
    Ok(())
}

fn encode_uncertainty(writer: &mut CanonicalWriter, value: Option<&Uncertainty>) -> Result<(), LaboratorySemanticError> {
    match value {
        Some(value) => {
            writer.u8(1);
            match value.confidence {
                Some(confidence) => {
                    writer.u8(1);
                    writer.f64(confidence);
                }
                None => writer.u8(0),
            }
            writer.option_string(value.interpretation.as_deref())?;
        }
        None => writer.u8(0),
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum LaboratorySemanticError {
    #[error("unsupported laboratory-result schema version: {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("laboratory result id is missing")]
    MissingResultId,
    #[error("laboratory result subject is invalid")]
    InvalidSubject,
    #[error("terminology system is missing")]
    MissingTerminologySystem,
    #[error("terminology code is missing")]
    MissingTerminologyCode,
    #[error("terminology version/release is missing")]
    MissingTerminologyVersion,
    #[error("laboratory analyte identity must use LOINC")]
    AnalyteMustUseLoinc,
    #[error("specimen id is missing")]
    MissingSpecimenId,
    #[error("optional identity field was present but empty")]
    EmptyOptionalIdentityField,
    #[error("reference population context is missing")]
    MissingReferencePopulationContext,
    #[error("reference interval has no low or high boundary")]
    EmptyReferenceInterval,
    #[error("reference interval is inverted")]
    InvertedReferenceInterval,
    #[error("laboratory quantities use incompatible units")]
    IncompatibleUnits,
    #[error("detection/quantification limits are empty")]
    EmptyDetectionLimits,
    #[error("detection/quantification limits require a quantitative result")]
    DetectionLimitsRequireQuantity,
    #[error("detection/quantification limit ordering is invalid")]
    InvalidDetectionLimitOrdering,
    #[error("evidence artifact identity is incomplete")]
    IncompleteEvidenceIdentity,
    #[error("evidence digest may not be all zeroes")]
    ZeroEvidenceDigest,
    #[error("interpretation/flag source is missing")]
    MissingInterpretationSource,
    #[error("laboratory provenance is incomplete")]
    IncompleteLaboratoryProvenance,
    #[error("laboratory event timeline is inconsistent")]
    InvalidLaboratoryTimeline,
    #[error("corrected/amended/appended result requires exact supersession evidence")]
    CorrectionRequiresSupersession,
    #[error("superseded laboratory result identity is invalid")]
    InvalidSupersededResult,
    #[error("confidence must be finite and within [0, 1]")]
    InvalidConfidence,
    #[error("canonical framing length exceeds v1 limit")]
    LengthOverflow,
    #[error(transparent)]
    ClinicalSemantics(#[from] ClinicalSemanticsError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn concept(system: &str, code: &str) -> CodeableConcept {
        CodeableConcept {
            coding: vec![Coding {
                system: system.to_string(),
                code: code.to_string(),
                display: None,
                version: Some("2026".to_string()),
            }],
            text: None,
        }
    }

    fn reference_source() -> EvidenceArtifactIdentityV1 {
        EvidenceArtifactIdentityV1 {
            namespace: "lab/reference-interval/v1".to_string(),
            artifact_id: "adult-fasting-glucose".to_string(),
            version: "2026-01".to_string(),
            digest: [7; 32],
        }
    }

    fn valid_result() -> LaboratoryResultV1 {
        LaboratoryResultV1 {
            schema_version: LABORATORY_RESULT_VERSION,
            result_id: "lab-result-1".to_string(),
            subject: SubjectRef {
                resource_type: "Patient".to_string(),
                id: "patient-a".to_string(),
            },
            analyte: TerminologyCodeV1 {
                system: LOINC_SYSTEM.to_string(),
                code: "14771-0".to_string(),
                version: "2.83".to_string(),
                display: Some("Glucose [Moles/volume] in Serum or Plasma".to_string()),
            },
            value: LaboratoryValueV1::Quantity(Quantity::ucum(5.5, "mmol/L")),
            status: LaboratoryResultStatusV1::Final,
            specimen: EvidenceFieldV1::Known(SpecimenIdentityV1 {
                specimen_id: "specimen-1".to_string(),
                specimen_type: concept("http://snomed.info/sct", "119364003"),
                collected_at_micros: Some(100),
                received_at_micros: Some(110),
            }),
            assay: EvidenceFieldV1::Known(AssayMethodV1 {
                method: concept("http://snomed.info/sct", "702659008"),
                assay_name: Some("hexokinase".to_string()),
                manufacturer: Some("Example Lab".to_string()),
                analyzer_model: Some("Analyzer-1".to_string()),
                lot_or_reagent_id: Some("lot-42".to_string()),
            }),
            reference_intervals: vec![ReferenceIntervalV1 {
                low: Some(Quantity::ucum(3.9, "mmol/L")),
                high: Some(Quantity::ucum(5.6, "mmol/L")),
                population: ReferencePopulationV1 {
                    description: "adult fasting reference population".to_string(),
                    source: reference_source(),
                },
            }],
            detection_limits: Some(DetectionLimitsV1 {
                lower_detection: Some(Quantity::ucum(0.1, "mmol/L")),
                lower_quantification: Some(Quantity::ucum(0.2, "mmol/L")),
                upper_quantification: Some(Quantity::ucum(40.0, "mmol/L")),
                upper_detection: Some(Quantity::ucum(50.0, "mmol/L")),
            }),
            interpretations: vec![InterpretationFlagV1 {
                interpretation: concept("http://terminology.hl7.org/CodeSystem/v3-ObservationInterpretation", "N"),
                source: "laboratory instrument".to_string(),
            }],
            provenance: LaboratoryProvenanceV1 {
                source_system: "https://lab.example/fhir".to_string(),
                source_resource_id: "Observation/obs-1".to_string(),
                source_version: Some("5".to_string()),
                laboratory_id: "lab-a".to_string(),
                accession_id: Some("accession-1".to_string()),
                analyzed_at_micros: Some(120),
                resulted_at_micros: 130,
                performer: Some("Organization/lab-a".to_string()),
                transformation: Some(TransformationProvenance {
                    software: "lab-adapter".to_string(),
                    version: "1".to_string(),
                    operation: "FHIR laboratory result -> LaboratoryResultV1".to_string(),
                    input_fact_ids: vec![],
                }),
            },
            uncertainty: None,
            supersedes: None,
        }
    }

    #[test]
    fn valid_result_has_stable_snapshot_identity() {
        let result = valid_result();
        let first = laboratory_result_snapshot_digest_v1(&result).unwrap();
        let second = laboratory_result_snapshot_digest_v1(&result).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn unit_substitution_changes_snapshot_identity() {
        let original = valid_result();
        let mut changed = original.clone();
        changed.value = LaboratoryValueV1::Quantity(Quantity::ucum(5.5, "mg/dL"));
        changed.reference_intervals.clear();
        changed.detection_limits = None;
        assert_ne!(
            laboratory_result_snapshot_digest_v1(&original).unwrap(),
            laboratory_result_snapshot_digest_v1(&changed).unwrap()
        );
    }

    #[test]
    fn method_substitution_changes_snapshot_identity() {
        let original = valid_result();
        let mut changed = original.clone();
        let EvidenceFieldV1::Known(assay) = &mut changed.assay else {
            panic!("expected known assay");
        };
        assay.assay_name = Some("glucose-oxidase".to_string());
        assert_ne!(
            laboratory_result_snapshot_digest_v1(&original).unwrap(),
            laboratory_result_snapshot_digest_v1(&changed).unwrap()
        );
    }

    #[test]
    fn reference_context_substitution_changes_snapshot_identity() {
        let original = valid_result();
        let mut changed = original.clone();
        changed.reference_intervals[0].population.description = "pediatric reference population".to_string();
        assert_ne!(
            laboratory_result_snapshot_digest_v1(&original).unwrap(),
            laboratory_result_snapshot_digest_v1(&changed).unwrap()
        );
    }

    #[test]
    fn non_loinc_analyte_is_rejected() {
        let mut result = valid_result();
        result.analyte.system = "http://snomed.info/sct".to_string();
        assert!(matches!(
            result.validate(),
            Err(LaboratorySemanticError::AnalyteMustUseLoinc)
        ));
    }

    #[test]
    fn terminology_release_may_not_be_implicit() {
        let mut result = valid_result();
        result.analyte.version.clear();
        assert!(matches!(
            result.validate(),
            Err(LaboratorySemanticError::MissingTerminologyVersion)
        ));
    }

    #[test]
    fn corrected_result_requires_exact_supersession_evidence() {
        let mut result = valid_result();
        result.status = LaboratoryResultStatusV1::Corrected;
        assert!(matches!(
            result.validate(),
            Err(LaboratorySemanticError::CorrectionRequiresSupersession)
        ));
        result.supersedes = Some(PriorLaboratoryResultV1 {
            result_id: "lab-result-0".to_string(),
            snapshot_digest: [9; 32],
        });
        result.validate().unwrap();
    }

    #[test]
    fn explicit_missing_assay_is_preserved_without_defaulting() {
        let mut result = valid_result();
        result.assay = EvidenceFieldV1::Missing(MissingReasonV1::NotProvided);
        result.validate().unwrap();
        let digest = laboratory_result_snapshot_digest_v1(&result).unwrap();
        assert_ne!(digest.into_bytes(), [0; 32]);
    }

    #[test]
    fn cross_unit_reference_interval_is_rejected() {
        let mut result = valid_result();
        result.reference_intervals[0].high = Some(Quantity::ucum(100.0, "mg/dL"));
        assert!(matches!(
            result.validate(),
            Err(LaboratorySemanticError::IncompatibleUnits)
        ));
    }

    #[test]
    fn impossible_specimen_timeline_is_rejected() {
        let mut result = valid_result();
        let EvidenceFieldV1::Known(specimen) = &mut result.specimen else {
            panic!("expected known specimen");
        };
        specimen.received_at_micros = Some(125);
        assert!(matches!(
            result.validate(),
            Err(LaboratorySemanticError::InvalidLaboratoryTimeline)
        ));
    }

    #[test]
    fn clinical_fact_projection_preserves_analyte_value_and_subject() {
        let result = valid_result();
        let projection = result.to_clinical_fact().unwrap();
        assert_eq!(projection.fact.subject.id, "patient-a");
        assert_eq!(projection.fact.concept.coding[0].system, LOINC_SYSTEM);
        assert_eq!(projection.fact.concept.coding[0].code, "14771-0");
        let ClinicalValue::Quantity(quantity) = projection.fact.value else {
            panic!("expected quantitative ClinicalFact");
        };
        assert_eq!(quantity.code, "mmol/L");
        assert_eq!(quantity.value, 5.5);
    }
}
