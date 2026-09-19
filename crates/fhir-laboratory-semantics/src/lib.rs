#![deny(unsafe_code)]
//! Strict FHIR R4 laboratory-report projection into Mycelix laboratory semantics.
//!
//! This adapter resolves a bounded `DiagnosticReport -> Observation -> Specimen`
//! graph and refuses ambiguous or lossy mappings. It is not a general FHIR
//! validator and it grants no clinical authority.

use chrono::DateTime;
use mycelix_clinical_laboratory_semantics::{
    AssayMethodV1, EvidenceFieldV1, InterpretationFlagV1, LaboratoryProvenanceV1,
    LaboratoryResultStatusV1, LaboratoryResultV1, LaboratoryValueV1, MissingReasonV1,
    PriorLaboratoryResultV1, ReferenceIntervalV1, ReferencePopulationV1, SpecimenIdentityV1,
    TerminologyCodeV1, LABORATORY_RESULT_VERSION, LOINC_SYSTEM,
};
use mycelix_clinical_semantics::{
    CodeableConcept, Coding, Quantity, SubjectRef, TransformationProvenance,
};
use serde_json::Value;
use std::collections::HashSet;
use thiserror::Error;

pub const ADAPTER_NAME: &str = "mycelix-fhir-laboratory-semantics";
pub const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, PartialEq)]
pub struct ReferencePopulationBindingV1 {
    pub observation_id: String,
    pub range_index: u32,
    pub population: ReferencePopulationV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PriorResultBindingV1 {
    pub observation_id: String,
    pub prior: PriorLaboratoryResultV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FhirLaboratoryProjectionPolicyV1 {
    pub source_system: String,
    pub expected_patient_id: String,
    pub diagnostic_report_id: String,
    pub laboratory_id: String,
    pub accepted_loinc_release: String,
    pub reference_population_bindings: Vec<ReferencePopulationBindingV1>,
    pub prior_result_bindings: Vec<PriorResultBindingV1>,
}

impl FhirLaboratoryProjectionPolicyV1 {
    fn validate(&self) -> Result<(), FhirLaboratoryError> {
        for value in [
            &self.source_system,
            &self.expected_patient_id,
            &self.diagnostic_report_id,
            &self.laboratory_id,
            &self.accepted_loinc_release,
        ] {
            if value.trim().is_empty() {
                return Err(FhirLaboratoryError::InvalidPolicy);
            }
        }

        let mut range_keys = HashSet::new();
        for binding in &self.reference_population_bindings {
            if binding.observation_id.trim().is_empty()
                || !range_keys.insert((binding.observation_id.clone(), binding.range_index))
            {
                return Err(FhirLaboratoryError::InvalidPolicy);
            }
        }

        let mut prior_keys = HashSet::new();
        for binding in &self.prior_result_bindings {
            if binding.observation_id.trim().is_empty()
                || !prior_keys.insert(binding.observation_id.clone())
            {
                return Err(FhirLaboratoryError::InvalidPolicy);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FhirLaboratoryProjectionV1 {
    pub diagnostic_report_id: String,
    pub results: Vec<LaboratoryResultV1>,
}

pub fn project_laboratory_bundle_strict(
    bundle: &Value,
    policy: &FhirLaboratoryProjectionPolicyV1,
) -> Result<FhirLaboratoryProjectionV1, FhirLaboratoryError> {
    policy.validate()?;
    let entries = bundle
        .get("entry")
        .and_then(Value::as_array)
        .ok_or(FhirLaboratoryError::BundleMissingEntries)?;

    let report = unique_resource(entries, "DiagnosticReport", &policy.diagnostic_report_id)?;
    bind_patient_reference(
        report.get("subject").and_then(|value| value.get("reference")).and_then(Value::as_str),
        &policy.expected_patient_id,
    )?;
    validate_report_status(report)?;
    validate_report_performer(report, &policy.laboratory_id)?;

    let report_issued = report
        .get("issued")
        .and_then(Value::as_str)
        .map(parse_datetime)
        .transpose()?;
    let accession_id = report
        .get("identifier")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("value"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let report_performer = first_reference(report.get("performer"));

    let result_refs = report
        .get("result")
        .and_then(Value::as_array)
        .ok_or(FhirLaboratoryError::ReportMissingResults)?;
    if result_refs.is_empty() {
        return Err(FhirLaboratoryError::ReportMissingResults);
    }

    let mut seen_observations = HashSet::new();
    let mut used_range_bindings = HashSet::new();
    let mut used_prior_bindings = HashSet::new();
    let mut results = Vec::with_capacity(result_refs.len());

    for reference in result_refs {
        let reference = reference
            .get("reference")
            .and_then(Value::as_str)
            .ok_or(FhirLaboratoryError::InvalidReference)?;
        let observation_id = exact_relative_reference(reference, "Observation")?;
        if !seen_observations.insert(observation_id.to_string()) {
            return Err(FhirLaboratoryError::DuplicateReportResultReference(
                observation_id.to_string(),
            ));
        }
        let observation = unique_resource(entries, "Observation", observation_id)?;
        let result = project_observation(
            entries,
            report,
            report_issued,
            report_performer.as_deref(),
            accession_id.as_deref(),
            observation,
            observation_id,
            policy,
            &mut used_range_bindings,
            &mut used_prior_bindings,
        )?;
        results.push(result);
    }

    for binding in &policy.reference_population_bindings {
        if !used_range_bindings.contains(&(binding.observation_id.clone(), binding.range_index)) {
            return Err(FhirLaboratoryError::UnusedReferencePopulationBinding {
                observation_id: binding.observation_id.clone(),
                range_index: binding.range_index,
            });
        }
    }
    for binding in &policy.prior_result_bindings {
        if !used_prior_bindings.contains(&binding.observation_id) {
            return Err(FhirLaboratoryError::UnusedPriorResultBinding(
                binding.observation_id.clone(),
            ));
        }
    }

    Ok(FhirLaboratoryProjectionV1 {
        diagnostic_report_id: policy.diagnostic_report_id.clone(),
        results,
    })
}

#[allow(clippy::too_many_arguments)]
fn project_observation(
    entries: &[Value],
    _report: &Value,
    report_issued: Option<i64>,
    report_performer: Option<&str>,
    accession_id: Option<&str>,
    observation: &Value,
    observation_id: &str,
    policy: &FhirLaboratoryProjectionPolicyV1,
    used_range_bindings: &mut HashSet<(String, u32)>,
    used_prior_bindings: &mut HashSet<String>,
) -> Result<LaboratoryResultV1, FhirLaboratoryError> {
    bind_patient_reference(
        observation
            .get("subject")
            .and_then(|value| value.get("reference"))
            .and_then(Value::as_str),
        &policy.expected_patient_id,
    )?;

    let analyte = parse_loinc_analyte(observation, &policy.accepted_loinc_release)?;
    let value = parse_laboratory_value(observation)?;
    let status = parse_result_status(observation)?;
    let effective_at = parse_required_datetime_field(observation, "effectiveDateTime")?;

    let specimen_reference = observation
        .get("specimen")
        .and_then(|value| value.get("reference"))
        .and_then(Value::as_str)
        .ok_or(FhirLaboratoryError::MissingSpecimenReference)?;
    let specimen_id = exact_relative_reference(specimen_reference, "Specimen")?;
    let specimen_resource = unique_resource(entries, "Specimen", specimen_id)?;
    bind_patient_reference(
        specimen_resource
            .get("subject")
            .and_then(|value| value.get("reference"))
            .and_then(Value::as_str),
        &policy.expected_patient_id,
    )?;
    let specimen = parse_specimen(specimen_resource, specimen_id)?;
    if specimen.collected_at_micros != Some(effective_at) {
        return Err(FhirLaboratoryError::EffectiveTimeNotRepresentable {
            observation_id: observation_id.to_string(),
        });
    }

    let assay = match observation.get("method") {
        Some(method) => EvidenceFieldV1::Known(AssayMethodV1 {
            method: parse_codeable_concept(method)?,
            assay_name: None,
            manufacturer: None,
            analyzer_model: None,
            lot_or_reagent_id: None,
        }),
        None => EvidenceFieldV1::Missing(MissingReasonV1::NotProvided),
    };

    let reference_intervals = parse_reference_ranges(
        observation,
        observation_id,
        policy,
        used_range_bindings,
    )?;

    let interpretations = observation
        .get("interpretation")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    Ok(InterpretationFlagV1 {
                        interpretation: parse_codeable_concept(item)?,
                        source: "FHIR Observation.interpretation".to_string(),
                    })
                })
                .collect::<Result<Vec<_>, FhirLaboratoryError>>()
        })
        .transpose()?
        .unwrap_or_default();

    let prior = policy
        .prior_result_bindings
        .iter()
        .find(|binding| binding.observation_id == observation_id)
        .map(|binding| {
            used_prior_bindings.insert(binding.observation_id.clone());
            binding.prior.clone()
        });
    if matches!(
        status,
        LaboratoryResultStatusV1::Amended
            | LaboratoryResultStatusV1::Corrected
            | LaboratoryResultStatusV1::Appended
    ) && prior.is_none()
    {
        return Err(FhirLaboratoryError::CorrectionMissingPriorBinding(
            observation_id.to_string(),
        ));
    }

    let source_version = observation
        .get("meta")
        .and_then(|value| value.get("versionId"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let performer = first_reference(observation.get("performer"))
        .or_else(|| report_performer.map(str::to_string));
    let resulted_at_micros = observation
        .get("issued")
        .and_then(Value::as_str)
        .map(parse_datetime)
        .transpose()?
        .or(report_issued)
        .ok_or(FhirLaboratoryError::MissingResultedTime)?;

    let result = LaboratoryResultV1 {
        schema_version: LABORATORY_RESULT_VERSION,
        result_id: format!("fhir:{}:Observation:{}", policy.source_system, observation_id),
        subject: SubjectRef {
            resource_type: "Patient".to_string(),
            id: policy.expected_patient_id.clone(),
        },
        analyte,
        value,
        status,
        specimen: EvidenceFieldV1::Known(specimen),
        assay,
        reference_intervals,
        detection_limits: None,
        interpretations,
        provenance: LaboratoryProvenanceV1 {
            source_system: policy.source_system.clone(),
            source_resource_id: format!("Observation/{observation_id}"),
            source_version,
            laboratory_id: policy.laboratory_id.clone(),
            accession_id: accession_id.map(str::to_string),
            analyzed_at_micros: None,
            resulted_at_micros,
            performer,
            transformation: Some(TransformationProvenance {
                software: ADAPTER_NAME.to_string(),
                version: ADAPTER_VERSION.to_string(),
                operation: "FHIR R4 laboratory report -> LaboratoryResultV1".to_string(),
                input_fact_ids: vec![
                    format!("DiagnosticReport/{}", policy.diagnostic_report_id),
                    format!("Observation/{observation_id}"),
                    format!("Specimen/{specimen_id}"),
                ],
            }),
        },
        uncertainty: None,
        supersedes: prior,
    };
    result.validate()?;
    Ok(result)
}

fn parse_loinc_analyte(
    observation: &Value,
    accepted_release: &str,
) -> Result<TerminologyCodeV1, FhirLaboratoryError> {
    let codings = observation
        .get("code")
        .and_then(|value| value.get("coding"))
        .and_then(Value::as_array)
        .ok_or(FhirLaboratoryError::MissingObservationCode)?;
    let mut loinc = codings.iter().filter(|coding| {
        coding.get("system").and_then(Value::as_str) == Some(LOINC_SYSTEM)
    });
    let coding = loinc.next().ok_or(FhirLaboratoryError::MissingLoincCoding)?;
    if loinc.next().is_some() {
        return Err(FhirLaboratoryError::AmbiguousLoincCoding);
    }
    let code = required_string(coding, "code")?;
    let version = required_string(coding, "version")?;
    if version != accepted_release {
        return Err(FhirLaboratoryError::LoincReleaseMismatch {
            expected: accepted_release.to_string(),
            found: version,
        });
    }
    Ok(TerminologyCodeV1 {
        system: LOINC_SYSTEM.to_string(),
        code,
        version: accepted_release.to_string(),
        display: coding.get("display").and_then(Value::as_str).map(str::to_string),
    })
}

fn parse_laboratory_value(observation: &Value) -> Result<LaboratoryValueV1, FhirLaboratoryError> {
    if let Some(value) = observation.get("valueQuantity") {
        return Ok(LaboratoryValueV1::Quantity(parse_quantity(value)?));
    }
    if let Some(value) = observation.get("valueCodeableConcept") {
        return Ok(LaboratoryValueV1::Coded(parse_codeable_concept(value)?));
    }
    Err(FhirLaboratoryError::UnsupportedLaboratoryValue)
}

fn parse_result_status(observation: &Value) -> Result<LaboratoryResultStatusV1, FhirLaboratoryError> {
    match required_string(observation, "status")?.as_str() {
        "registered" => Ok(LaboratoryResultStatusV1::Registered),
        "preliminary" => Ok(LaboratoryResultStatusV1::Preliminary),
        "final" => Ok(LaboratoryResultStatusV1::Final),
        "amended" => Ok(LaboratoryResultStatusV1::Amended),
        "corrected" => Ok(LaboratoryResultStatusV1::Corrected),
        "cancelled" => Ok(LaboratoryResultStatusV1::Cancelled),
        "entered-in-error" => Ok(LaboratoryResultStatusV1::EnteredInError),
        "unknown" => Ok(LaboratoryResultStatusV1::Unknown),
        other => Err(FhirLaboratoryError::UnsupportedObservationStatus(other.to_string())),
    }
}

fn parse_specimen(resource: &Value, specimen_id: &str) -> Result<SpecimenIdentityV1, FhirLaboratoryError> {
    let specimen_type = resource
        .get("type")
        .ok_or(FhirLaboratoryError::MissingSpecimenType)
        .and_then(parse_codeable_concept)?;
    let collected_at_micros = resource
        .get("collection")
        .and_then(|value| value.get("collectedDateTime"))
        .and_then(Value::as_str)
        .map(parse_datetime)
        .transpose()?;
    if collected_at_micros.is_none() {
        return Err(FhirLaboratoryError::MissingSpecimenCollectionTime);
    }
    let received_at_micros = resource
        .get("receivedTime")
        .and_then(Value::as_str)
        .map(parse_datetime)
        .transpose()?;
    Ok(SpecimenIdentityV1 {
        specimen_id: specimen_id.to_string(),
        specimen_type,
        collected_at_micros,
        received_at_micros,
    })
}

fn parse_reference_ranges(
    observation: &Value,
    observation_id: &str,
    policy: &FhirLaboratoryProjectionPolicyV1,
    used: &mut HashSet<(String, u32)>,
) -> Result<Vec<ReferenceIntervalV1>, FhirLaboratoryError> {
    let Some(ranges) = observation.get("referenceRange").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut output = Vec::with_capacity(ranges.len());
    for (index, range) in ranges.iter().enumerate() {
        let index = u32::try_from(index).map_err(|_| FhirLaboratoryError::TooManyReferenceRanges)?;
        let binding = policy
            .reference_population_bindings
            .iter()
            .find(|binding| binding.observation_id == observation_id && binding.range_index == index)
            .ok_or_else(|| FhirLaboratoryError::MissingReferencePopulationBinding {
                observation_id: observation_id.to_string(),
                range_index: index,
            })?;
        used.insert((observation_id.to_string(), index));
        let low = range.get("low").map(parse_quantity).transpose()?;
        let high = range.get("high").map(parse_quantity).transpose()?;
        output.push(ReferenceIntervalV1 {
            low,
            high,
            population: binding.population.clone(),
        });
    }
    Ok(output)
}

fn validate_report_status(report: &Value) -> Result<(), FhirLaboratoryError> {
    let status = required_string(report, "status")?;
    if matches!(status.as_str(), "cancelled" | "entered-in-error") {
        return Err(FhirLaboratoryError::UnusableDiagnosticReportStatus(status));
    }
    Ok(())
}

fn validate_report_performer(report: &Value, expected: &str) -> Result<(), FhirLaboratoryError> {
    if let Some(performers) = report.get("performer").and_then(Value::as_array) {
        let references: Vec<_> = performers
            .iter()
            .filter_map(|value| value.get("reference").and_then(Value::as_str))
            .collect();
        if !references.is_empty() && !references.iter().any(|value| *value == expected) {
            return Err(FhirLaboratoryError::LaboratoryPerformerMismatch {
                expected: expected.to_string(),
                found: references.into_iter().map(str::to_string).collect(),
            });
        }
    }
    Ok(())
}

fn bind_patient_reference(reference: Option<&str>, expected_patient_id: &str) -> Result<(), FhirLaboratoryError> {
    let reference = reference.ok_or(FhirLaboratoryError::MissingPatientReference)?;
    let found = exact_relative_reference(reference, "Patient")?;
    if found != expected_patient_id {
        return Err(FhirLaboratoryError::PatientMismatch {
            expected: expected_patient_id.to_string(),
            found: found.to_string(),
        });
    }
    Ok(())
}

fn unique_resource<'a>(
    entries: &'a [Value],
    resource_type: &str,
    id: &str,
) -> Result<&'a Value, FhirLaboratoryError> {
    let mut found = entries.iter().filter_map(|entry| entry.get("resource")).filter(|resource| {
        resource.get("resourceType").and_then(Value::as_str) == Some(resource_type)
            && resource.get("id").and_then(Value::as_str) == Some(id)
    });
    let resource = found.next().ok_or_else(|| FhirLaboratoryError::ResourceNotFound {
        resource_type: resource_type.to_string(),
        id: id.to_string(),
    })?;
    if found.next().is_some() {
        return Err(FhirLaboratoryError::DuplicateResourceIdentity {
            resource_type: resource_type.to_string(),
            id: id.to_string(),
        });
    }
    Ok(resource)
}

fn exact_relative_reference<'a>(reference: &'a str, expected_type: &str) -> Result<&'a str, FhirLaboratoryError> {
    let Some((resource_type, id)) = reference.split_once('/') else {
        return Err(FhirLaboratoryError::InvalidReference);
    };
    if resource_type != expected_type || id.trim().is_empty() || id.contains('/') {
        return Err(FhirLaboratoryError::InvalidReference);
    }
    Ok(id)
}

fn parse_quantity(value: &Value) -> Result<Quantity, FhirLaboratoryError> {
    let number = value
        .get("value")
        .and_then(Value::as_f64)
        .ok_or(FhirLaboratoryError::InvalidQuantity)?;
    let system = required_string(value, "system")?;
    let code = required_string(value, "code")?;
    Ok(Quantity {
        value: number,
        display_unit: value.get("unit").and_then(Value::as_str).map(str::to_string),
        system,
        code,
    })
}

fn parse_codeable_concept(value: &Value) -> Result<CodeableConcept, FhirLaboratoryError> {
    let coding_values = value
        .get("coding")
        .and_then(Value::as_array)
        .ok_or(FhirLaboratoryError::UncodedConcept)?;
    if coding_values.is_empty() {
        return Err(FhirLaboratoryError::UncodedConcept);
    }
    let mut coding = Vec::with_capacity(coding_values.len());
    for item in coding_values {
        coding.push(Coding {
            system: required_string(item, "system")?,
            code: required_string(item, "code")?,
            display: item.get("display").and_then(Value::as_str).map(str::to_string),
            version: item.get("version").and_then(Value::as_str).map(str::to_string),
        });
    }
    Ok(CodeableConcept {
        coding,
        text: value.get("text").and_then(Value::as_str).map(str::to_string),
    })
}

fn parse_required_datetime_field(resource: &Value, field: &'static str) -> Result<i64, FhirLaboratoryError> {
    let value = resource
        .get(field)
        .and_then(Value::as_str)
        .ok_or(FhirLaboratoryError::MissingDateTime(field))?;
    parse_datetime(value)
}

fn parse_datetime(value: &str) -> Result<i64, FhirLaboratoryError> {
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.timestamp_micros())
        .map_err(|_| FhirLaboratoryError::InvalidDateTime(value.to_string()))
}

fn required_string(value: &Value, field: &'static str) -> Result<String, FhirLaboratoryError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or(FhirLaboratoryError::MissingField(field))
}

fn first_reference(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("reference"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

#[derive(Debug, Error)]
pub enum FhirLaboratoryError {
    #[error("FHIR Bundle.entry is required")]
    BundleMissingEntries,
    #[error("FHIR laboratory projection policy is incomplete or ambiguous")]
    InvalidPolicy,
    #[error("resource not found: {resource_type}/{id}")]
    ResourceNotFound { resource_type: String, id: String },
    #[error("duplicate FHIR resource identity: {resource_type}/{id}")]
    DuplicateResourceIdentity { resource_type: String, id: String },
    #[error("FHIR reference must be an exact relative ResourceType/id reference")]
    InvalidReference,
    #[error("patient reference is missing")]
    MissingPatientReference,
    #[error("patient mismatch: expected {expected}, found {found}")]
    PatientMismatch { expected: String, found: String },
    #[error("DiagnosticReport.result is required and must be non-empty")]
    ReportMissingResults,
    #[error("DiagnosticReport result references Observation/{0} more than once")]
    DuplicateReportResultReference(String),
    #[error("DiagnosticReport status cannot be projected as active laboratory evidence: {0}")]
    UnusableDiagnosticReportStatus(String),
    #[error("DiagnosticReport performer does not contain expected laboratory {expected}; found {found:?}")]
    LaboratoryPerformerMismatch { expected: String, found: Vec<String> },
    #[error("Observation.code.coding is required")]
    MissingObservationCode,
    #[error("Observation has no LOINC coding")]
    MissingLoincCoding,
    #[error("Observation has more than one LOINC coding")]
    AmbiguousLoincCoding,
    #[error("LOINC release mismatch: expected {expected}, found {found}")]
    LoincReleaseMismatch { expected: String, found: String },
    #[error("unsupported laboratory Observation value[x]")]
    UnsupportedLaboratoryValue,
    #[error("unsupported Observation.status: {0}")]
    UnsupportedObservationStatus(String),
    #[error("Observation.specimen reference is required for strict v1 projection")]
    MissingSpecimenReference,
    #[error("Specimen.type is required")]
    MissingSpecimenType,
    #[error("Specimen.collection.collectedDateTime is required for strict v1 projection")]
    MissingSpecimenCollectionTime,
    #[error("Observation effectiveDateTime cannot be represented without loss for {observation_id}; v1 requires equality with specimen collection time")]
    EffectiveTimeNotRepresentable { observation_id: String },
    #[error("reference range {range_index} for Observation/{observation_id} lacks an explicit reference-population binding")]
    MissingReferencePopulationBinding { observation_id: String, range_index: u32 },
    #[error("reference-population binding is unused: Observation/{observation_id} range {range_index}")]
    UnusedReferencePopulationBinding { observation_id: String, range_index: u32 },
    #[error("prior-result binding is unused: Observation/{0}")]
    UnusedPriorResultBinding(String),
    #[error("corrected/amended Observation/{0} lacks exact prior-result binding")]
    CorrectionMissingPriorBinding(String),
    #[error("too many FHIR reference ranges for v1 index representation")]
    TooManyReferenceRanges,
    #[error("required FHIR field is missing: {0}")]
    MissingField(&'static str),
    #[error("FHIR quantity is invalid")]
    InvalidQuantity,
    #[error("FHIR CodeableConcept requires at least one coding")]
    UncodedConcept,
    #[error("required FHIR date/time field is missing: {0}")]
    MissingDateTime(&'static str),
    #[error("invalid FHIR dateTime: {0}")]
    InvalidDateTime(String),
    #[error("laboratory result requires Observation.issued or DiagnosticReport.issued")]
    MissingResultedTime,
    #[error(transparent)]
    Laboratory(#[from] mycelix_clinical_laboratory_semantics::LaboratorySemanticError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_laboratory_semantics::{
        EvidenceArtifactIdentityV1, LaboratoryValueV1, ReferencePopulationV1,
    };
    use serde_json::json;

    fn reference_population() -> ReferencePopulationV1 {
        ReferencePopulationV1 {
            description: "adult fasting reference population".to_string(),
            source: EvidenceArtifactIdentityV1 {
                namespace: "lab/reference-interval/v1".to_string(),
                artifact_id: "adult-fasting-glucose".to_string(),
                version: "2026-01".to_string(),
                digest: [7; 32],
            },
        }
    }

    fn policy() -> FhirLaboratoryProjectionPolicyV1 {
        FhirLaboratoryProjectionPolicyV1 {
            source_system: "https://lab.example/fhir".to_string(),
            expected_patient_id: "patient-a".to_string(),
            diagnostic_report_id: "report-1".to_string(),
            laboratory_id: "Organization/lab-a".to_string(),
            accepted_loinc_release: "2.83".to_string(),
            reference_population_bindings: vec![ReferencePopulationBindingV1 {
                observation_id: "obs-1".to_string(),
                range_index: 0,
                population: reference_population(),
            }],
            prior_result_bindings: vec![],
        }
    }

    fn bundle() -> Value {
        json!({
            "resourceType": "Bundle",
            "entry": [
                {"resource": {
                    "resourceType": "Patient",
                    "id": "patient-a"
                }},
                {"resource": {
                    "resourceType": "Specimen",
                    "id": "spec-1",
                    "subject": {"reference": "Patient/patient-a"},
                    "type": {"coding": [{
                        "system": "http://snomed.info/sct",
                        "code": "119364003",
                        "display": "Serum specimen"
                    }]},
                    "collection": {"collectedDateTime": "2026-09-19T08:00:00+02:00"},
                    "receivedTime": "2026-09-19T08:15:00+02:00"
                }},
                {"resource": {
                    "resourceType": "Observation",
                    "id": "obs-1",
                    "meta": {"versionId": "5"},
                    "status": "final",
                    "subject": {"reference": "Patient/patient-a"},
                    "code": {"coding": [{
                        "system": "http://loinc.org",
                        "code": "14771-0",
                        "version": "2.83",
                        "display": "Glucose [Moles/volume] in Serum or Plasma"
                    }]},
                    "effectiveDateTime": "2026-09-19T08:00:00+02:00",
                    "issued": "2026-09-19T09:00:00+02:00",
                    "specimen": {"reference": "Specimen/spec-1"},
                    "method": {"coding": [{
                        "system": "http://snomed.info/sct",
                        "code": "702659008",
                        "display": "Hexokinase method"
                    }]},
                    "valueQuantity": {
                        "value": 5.5,
                        "unit": "mmol/L",
                        "system": "http://unitsofmeasure.org",
                        "code": "mmol/L"
                    },
                    "referenceRange": [{
                        "low": {"value": 3.9, "system": "http://unitsofmeasure.org", "code": "mmol/L"},
                        "high": {"value": 5.6, "system": "http://unitsofmeasure.org", "code": "mmol/L"}
                    }]
                }},
                {"resource": {
                    "resourceType": "DiagnosticReport",
                    "id": "report-1",
                    "status": "final",
                    "subject": {"reference": "Patient/patient-a"},
                    "issued": "2026-09-19T09:05:00+02:00",
                    "performer": [{"reference": "Organization/lab-a"}],
                    "identifier": [{"value": "accession-1"}],
                    "result": [{"reference": "Observation/obs-1"}]
                }}
            ]
        })
    }

    #[test]
    fn projects_complete_laboratory_context() {
        let projected = project_laboratory_bundle_strict(&bundle(), &policy()).unwrap();
        assert_eq!(projected.results.len(), 1);
        let result = &projected.results[0];
        assert_eq!(result.analyte.code, "14771-0");
        assert_eq!(result.analyte.version, "2.83");
        assert_eq!(result.reference_intervals.len(), 1);
        assert_eq!(result.provenance.laboratory_id, "Organization/lab-a");
        let LaboratoryValueV1::Quantity(quantity) = &result.value else {
            panic!("expected quantity");
        };
        assert_eq!(quantity.value, 5.5);
        assert_eq!(quantity.code, "mmol/L");
    }

    #[test]
    fn cross_patient_observation_is_rejected() {
        let mut value = bundle();
        value["entry"][2]["resource"]["subject"]["reference"] =
            Value::String("Patient/patient-b".to_string());
        assert!(matches!(
            project_laboratory_bundle_strict(&value, &policy()),
            Err(FhirLaboratoryError::PatientMismatch { .. })
        ));
    }

    #[test]
    fn unresolved_specimen_is_rejected_not_downgraded_to_missing() {
        let mut value = bundle();
        value["entry"][2]["resource"]["specimen"]["reference"] =
            Value::String("Specimen/missing".to_string());
        assert!(matches!(
            project_laboratory_bundle_strict(&value, &policy()),
            Err(FhirLaboratoryError::ResourceNotFound { .. })
        ));
    }

    #[test]
    fn loinc_release_mismatch_is_rejected() {
        let mut value = bundle();
        value["entry"][2]["resource"]["code"]["coding"][0]["version"] =
            Value::String("2.82".to_string());
        assert!(matches!(
            project_laboratory_bundle_strict(&value, &policy()),
            Err(FhirLaboratoryError::LoincReleaseMismatch { .. })
        ));
    }

    #[test]
    fn effective_time_must_equal_specimen_collection_in_v1() {
        let mut value = bundle();
        value["entry"][2]["resource"]["effectiveDateTime"] =
            Value::String("2026-09-19T08:01:00+02:00".to_string());
        assert!(matches!(
            project_laboratory_bundle_strict(&value, &policy()),
            Err(FhirLaboratoryError::EffectiveTimeNotRepresentable { .. })
        ));
    }

    #[test]
    fn reference_range_requires_external_population_binding() {
        let mut projection_policy = policy();
        projection_policy.reference_population_bindings.clear();
        assert!(matches!(
            project_laboratory_bundle_strict(&bundle(), &projection_policy),
            Err(FhirLaboratoryError::MissingReferencePopulationBinding { .. })
        ));
    }

    #[test]
    fn corrected_result_requires_exact_prior_binding() {
        let mut value = bundle();
        value["entry"][2]["resource"]["status"] = Value::String("corrected".to_string());
        assert!(matches!(
            project_laboratory_bundle_strict(&value, &policy()),
            Err(FhirLaboratoryError::CorrectionMissingPriorBinding(_))
        ));
    }
}
