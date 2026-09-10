#![deny(unsafe_code)]
//! Strict FHIR R4 MedicationRequest projection for Mycelix-Health.
//!
//! This adapter deliberately accepts a conservative activation-safe subset. A
//! syntactically valid MedicationRequest may still be rejected when projecting it
//! would discard safety-significant semantics or leave requester identity ambiguous.

use chrono::DateTime;
use mycelix_clinical_authority::PrincipalBinding;
use mycelix_clinical_integrity::{
    hash_canonical_bytes, DigestDomain, IntegrityError, StoredDigest, VerifiedDigest,
};
use mycelix_clinical_semantics::{
    bind_fhir_patient_reference, ClinicalSemanticsError, ClinicalValue, CodeableConcept, Coding,
    Quantity, Range, Ratio, SubjectBinding, SubjectRef, UCUM_SYSTEM,
};
use mycelix_medication_semantics::{
    AdministrationTiming, DosageInstruction, DoseAmount, MedicationOrder,
    MedicationOrderProvenance, MedicationRequestIntent, MedicationRequestStatus,
    MedicationSemanticsError, RateAmount, RequesterAuthorityRef,
};
use mycelix_provider_principal::{
    resolve_practitioner_principal, ExternalIdentifier, PractitionerReferenceInput,
    PrincipalResolutionError, ResolvedPractitionerPrincipal, VerifiedProviderPrincipalBinding,
};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

pub const ADAPTER_NAME: &str = "mycelix-fhir-medication-semantics";
pub const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");
const ARTIFACT_SCHEMA_TAG: &[u8] = b"mycelix-health/fhir-medication-request-artifact-v1";

/// Exact dispense-validity semantics retained from FHIR `dispenseRequest.validityPeriod`.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct DispenseValidityPeriod {
    pub start_micros: Option<i64>,
    pub end_micros: Option<i64>,
}

impl DispenseValidityPeriod {
    fn validate(&self) -> Result<(), FhirMedicationError> {
        if self.start_micros.is_none() && self.end_micros.is_none() {
            return Err(FhirMedicationError::EmptyDispenseValidityPeriod);
        }
        if let (Some(start), Some(end)) = (self.start_micros, self.end_micros) {
            if end <= start {
                return Err(FhirMedicationError::InvalidDispenseValidityPeriod);
            }
        }
        Ok(())
    }

    pub fn validate_active_at(&self, at_micros: i64) -> Result<(), FhirMedicationError> {
        self.validate()?;
        if self.start_micros.is_some_and(|start| at_micros < start) {
            return Err(FhirMedicationError::DispenseValidityNotStarted);
        }
        if self.end_micros.is_some_and(|end| at_micros >= end) {
            return Err(FhirMedicationError::DispenseValidityExpired);
        }
        Ok(())
    }
}

/// High-assurance projection artifact.
///
/// This type is serializable for exact artifact hashing, but intentionally does
/// not implement `Deserialize`: wire JSON cannot manufacture a resolved requester
/// artifact. Construction requires an in-process `ResolvedPractitionerPrincipal`.
#[derive(Serialize)]
pub struct MedicationRequestArtifact {
    order: MedicationOrder,
    requester_principal: [u8; 32],
    requester_provider_record_digest: StoredDigest,
    requester_author_binding_evidence_digest: StoredDigest,
    requester_status_evidence_digest: StoredDigest,
    requester_resolved_at_micros: i64,
    indications: Vec<CodeableConcept>,
    dispense_validity: Option<DispenseValidityPeriod>,
    notes: Vec<String>,
}

impl MedicationRequestArtifact {
    pub fn order(&self) -> &MedicationOrder {
        &self.order
    }

    pub fn requester_principal(&self) -> PrincipalBinding {
        PrincipalBinding(self.requester_principal)
    }

    pub fn requester_provider_record_digest(&self) -> StoredDigest {
        self.requester_provider_record_digest
    }

    pub fn requester_author_binding_evidence_digest(&self) -> StoredDigest {
        self.requester_author_binding_evidence_digest
    }

    pub fn requester_status_evidence_digest(&self) -> StoredDigest {
        self.requester_status_evidence_digest
    }

    pub fn requester_resolved_at_micros(&self) -> i64 {
        self.requester_resolved_at_micros
    }

    pub fn indications(&self) -> &[CodeableConcept] {
        &self.indications
    }

    pub fn dispense_validity(&self) -> Option<&DispenseValidityPeriod> {
        self.dispense_validity.as_ref()
    }

    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    pub fn validate_for_activation(&self, at_micros: i64) -> Result<(), FhirMedicationError> {
        self.order.validate_active_order_candidate()?;
        if let Some(validity) = &self.dispense_validity {
            validity.validate_active_at(at_micros)?;
        }
        for indication in &self.indications {
            indication.validate_machine_actionable()?;
        }
        Ok(())
    }

    /// Exact v1 artifact identity. The serializer/schema tag are part of the
    /// internal v1 contract; this is not a claim that arbitrary JSON is canonical.
    pub fn verified_digest(&self) -> Result<VerifiedDigest, FhirMedicationError> {
        let encoded = serde_json::to_vec(self)
            .map_err(|error| FhirMedicationError::ArtifactSerialization(error.to_string()))?;
        let mut framed = Vec::with_capacity(ARTIFACT_SCHEMA_TAG.len() + 1 + encoded.len());
        framed.extend_from_slice(ARTIFACT_SCHEMA_TAG);
        framed.push(0);
        framed.extend_from_slice(&encoded);
        Ok(hash_canonical_bytes(
            DigestDomain::MedicationRequestArtifact,
            &framed,
        )?)
    }
}

/// Project one FHIR R4 MedicationRequest that is intended to cross the active
/// prescribing workflow boundary.
///
/// v1 deliberately requires:
/// - active order-like status/intent;
/// - exact Patient subject binding;
/// - medicationCodeableConcept rather than unresolved medicationReference;
/// - a concrete Practitioner/PractitionerRole requester reference;
/// - unambiguous requester -> Mycelix principal resolution;
/// - typed dosage/route/timing and UCUM quantities;
/// - no safety-significant FHIR fields that this v1 artifact would silently drop.
pub fn project_active_medication_request_strict(
    resource: &Value,
    expected_patient_id: &str,
    source_system: &str,
    recorded_at_micros: i64,
    provider_bindings: &[VerifiedProviderPrincipalBinding],
) -> Result<MedicationRequestArtifact, FhirMedicationError> {
    let source_system = normalize_source_system(source_system)?;
    if resource.get("resourceType").and_then(Value::as_str) != Some("MedicationRequest") {
        return Err(FhirMedicationError::WrongResourceType);
    }

    reject_unsupported_root_semantics(resource)?;

    let resource_id = required_string(resource, "id")?;
    let status = parse_status(required_string(resource, "status")?.as_str())?;
    let intent = parse_intent(required_string(resource, "intent")?.as_str())?;

    let subject_reference = resource
        .get("subject")
        .and_then(|subject| subject.get("reference"))
        .and_then(Value::as_str);
    match bind_fhir_patient_reference(subject_reference, expected_patient_id) {
        SubjectBinding::Match => {}
        SubjectBinding::Mismatch { expected, found } => {
            return Err(FhirMedicationError::SubjectMismatch { expected, found });
        }
        SubjectBinding::Unresolved { reference, reason } => {
            return Err(FhirMedicationError::SubjectUnresolved {
                reference,
                reason: format!("{reason:?}"),
            });
        }
    }

    if resource.get("medicationReference").is_some() {
        return Err(FhirMedicationError::UnsupportedMedicationReference);
    }
    let medication = parse_codeable_concept(
        resource
            .get("medicationCodeableConcept")
            .ok_or(FhirMedicationError::MissingField("medicationCodeableConcept"))?,
    )?;

    let requester_value = resource
        .get("requester")
        .ok_or(FhirMedicationError::MissingField("requester"))?;
    let requester_reference = requester_value
        .get("reference")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or(FhirMedicationError::RequesterReferenceRequired)?
        .to_string();
    let requester_identifier = requester_value
        .get("identifier")
        .map(parse_external_identifier)
        .transpose()?;
    let requester_input = PractitionerReferenceInput {
        source_system: source_system.clone(),
        reference: Some(requester_reference.clone()),
        type_name: requester_value
            .get("type")
            .and_then(Value::as_str)
            .map(str::to_string),
        identifier: requester_identifier,
    };
    let resolved_requester = resolve_practitioner_principal(
        &requester_input,
        provider_bindings,
        recorded_at_micros,
    )?;

    let dosage_values = resource
        .get("dosageInstruction")
        .and_then(Value::as_array)
        .ok_or(FhirMedicationError::MissingTypedDosage)?;
    if dosage_values.is_empty() {
        return Err(FhirMedicationError::MissingTypedDosage);
    }
    let dosage = dosage_values
        .iter()
        .map(parse_dosage)
        .collect::<Result<Vec<_>, _>>()?;

    let (dispense_quantity, refills_authorized, dispense_validity) =
        parse_dispense_request(resource.get("dispenseRequest"))?;

    if resource
        .get("reasonReference")
        .and_then(Value::as_array)
        .is_some_and(|values| !values.is_empty())
    {
        return Err(FhirMedicationError::UnsupportedReasonReference);
    }
    let indications = resource
        .get("reasonCode")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .map(parse_codeable_concept)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    let authored_at_micros = resource
        .get("authoredOn")
        .and_then(Value::as_str)
        .map(parse_datetime)
        .transpose()?;
    let source_version = resource
        .get("meta")
        .and_then(|meta| meta.get("versionId"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let order = MedicationOrder {
        order_id: format!("fhir:{source_system}:MedicationRequest:{resource_id}"),
        subject: SubjectRef {
            resource_type: "Patient".into(),
            id: expected_patient_id.to_string(),
        },
        medication,
        product_strength: None,
        status,
        intent,
        dosage,
        dispense_quantity,
        refills_authorized,
        requester: Some(RequesterAuthorityRef {
            requester_reference,
            credential_reference: None,
        }),
        authored_at_micros,
        provenance: MedicationOrderProvenance {
            source_system,
            source_resource_id: resource_id,
            source_version,
            recorded_at_micros,
        },
    };
    order.validate_active_order_candidate()?;

    let notes = parse_annotation_texts(resource.get("note"))?;
    Ok(build_artifact(
        order,
        resolved_requester,
        indications,
        dispense_validity,
        notes,
    ))
}

fn build_artifact(
    order: MedicationOrder,
    requester: ResolvedPractitionerPrincipal,
    indications: Vec<CodeableConcept>,
    dispense_validity: Option<DispenseValidityPeriod>,
    notes: Vec<String>,
) -> MedicationRequestArtifact {
    MedicationRequestArtifact {
        order,
        requester_principal: requester.principal().0,
        requester_provider_record_digest: requester.provider_record_digest(),
        requester_author_binding_evidence_digest: requester.author_binding_evidence_digest(),
        requester_status_evidence_digest: requester.status_evidence_digest(),
        requester_resolved_at_micros: requester.resolved_at_micros(),
        indications,
        dispense_validity,
        notes,
    }
}

fn reject_unsupported_root_semantics(resource: &Value) -> Result<(), FhirMedicationError> {
    if resource.get("substitution").is_some() {
        return Err(FhirMedicationError::UnsupportedSafetySignificantField(
            "substitution",
        ));
    }
    if resource.get("priorPrescription").is_some() {
        return Err(FhirMedicationError::UnsupportedSafetySignificantField(
            "priorPrescription",
        ));
    }
    Ok(())
}

fn parse_status(value: &str) -> Result<MedicationRequestStatus, FhirMedicationError> {
    Ok(match value {
        "active" => MedicationRequestStatus::Active,
        "on-hold" => MedicationRequestStatus::OnHold,
        "cancelled" => MedicationRequestStatus::Cancelled,
        "completed" => MedicationRequestStatus::Completed,
        "entered-in-error" => MedicationRequestStatus::EnteredInError,
        "stopped" => MedicationRequestStatus::Stopped,
        "draft" => MedicationRequestStatus::Draft,
        "unknown" => MedicationRequestStatus::Unknown,
        other => return Err(FhirMedicationError::UnsupportedStatus(other.to_string())),
    })
}

fn parse_intent(value: &str) -> Result<MedicationRequestIntent, FhirMedicationError> {
    Ok(match value {
        "proposal" => MedicationRequestIntent::Proposal,
        "plan" => MedicationRequestIntent::Plan,
        "order" => MedicationRequestIntent::Order,
        "original-order" => MedicationRequestIntent::OriginalOrder,
        "reflex-order" => MedicationRequestIntent::ReflexOrder,
        "filler-order" => MedicationRequestIntent::FillerOrder,
        "instance-order" => MedicationRequestIntent::InstanceOrder,
        "option" => MedicationRequestIntent::Option,
        other => return Err(FhirMedicationError::UnsupportedIntent(other.to_string())),
    })
}

fn parse_dosage(value: &Value) -> Result<DosageInstruction, FhirMedicationError> {
    if value
        .get("additionalInstruction")
        .and_then(Value::as_array)
        .is_some_and(|values| !values.is_empty())
    {
        return Err(FhirMedicationError::UnsupportedSafetySignificantField(
            "dosageInstruction.additionalInstruction",
        ));
    }

    let route = parse_codeable_concept(
        value
            .get("route")
            .ok_or(FhirMedicationError::MissingField("dosageInstruction.route"))?,
    )?;
    let method = value.get("method").map(parse_codeable_concept).transpose()?;
    let site = value.get("site").map(parse_codeable_concept).transpose()?;
    let sequence = value
        .get("sequence")
        .and_then(Value::as_i64)
        .map(|number| i32::try_from(number).map_err(|_| FhirMedicationError::InvalidSequence))
        .transpose()?;

    let timing = parse_timing(value)?;
    let dose_and_rate = value
        .get("doseAndRate")
        .and_then(Value::as_array)
        .ok_or(FhirMedicationError::MissingDose)?;
    if dose_and_rate.len() != 1 {
        return Err(FhirMedicationError::UnsupportedDoseAndRateCardinality);
    }
    let dose_rate = &dose_and_rate[0];
    if dose_rate.get("type").is_some() {
        return Err(FhirMedicationError::UnsupportedSafetySignificantField(
            "dosageInstruction.doseAndRate.type",
        ));
    }
    let dose = parse_dose_amount(dose_rate)?;
    let rate = parse_rate_amount(dose_rate)?;

    let max_dose_per_period = value
        .get("maxDosePerPeriod")
        .map(parse_ratio)
        .transpose()?;
    let max_dose_per_administration = value
        .get("maxDosePerAdministration")
        .map(parse_quantity)
        .transpose()?;
    let max_dose_per_lifetime = value
        .get("maxDosePerLifetime")
        .map(parse_quantity)
        .transpose()?;

    Ok(DosageInstruction {
        sequence,
        narrative_sig: value.get("text").and_then(Value::as_str).map(str::to_string),
        patient_instruction: value
            .get("patientInstruction")
            .and_then(Value::as_str)
            .map(str::to_string),
        timing,
        route,
        method,
        site,
        dose,
        rate,
        max_dose_per_period,
        max_dose_per_administration,
        max_dose_per_lifetime,
    })
}

fn parse_timing(dosage: &Value) -> Result<AdministrationTiming, FhirMedicationError> {
    let as_needed_concept = dosage.get("asNeededCodeableConcept");
    let as_needed_boolean = dosage.get("asNeededBoolean").and_then(Value::as_bool);
    if as_needed_concept.is_some() && as_needed_boolean.is_some() {
        return Err(FhirMedicationError::ConflictingAsNeededChoice);
    }

    if let Some(concept) = as_needed_concept {
        if dosage.get("timing").is_some() {
            return Err(FhirMedicationError::UnsupportedPrnTimingConstraint);
        }
        return Ok(AdministrationTiming::AsNeeded {
            indication: parse_codeable_concept(concept)?,
            min_interval: None,
        });
    }
    if as_needed_boolean == Some(true) {
        return Err(FhirMedicationError::UncodedAsNeeded);
    }

    let timing = dosage
        .get("timing")
        .ok_or(FhirMedicationError::MissingTypedTiming)?;
    if timing.get("event").is_some() || timing.get("code").is_some() {
        return Err(FhirMedicationError::UnsupportedComplexTiming);
    }
    let repeat = timing
        .get("repeat")
        .ok_or(FhirMedicationError::MissingTypedTiming)?;

    const UNSUPPORTED_REPEAT_FIELDS: &[&str] = &[
        "boundsDuration",
        "boundsRange",
        "boundsPeriod",
        "count",
        "countMax",
        "duration",
        "durationMax",
        "durationUnit",
        "frequencyMax",
        "periodMax",
        "dayOfWeek",
        "timeOfDay",
        "when",
        "offset",
    ];
    if UNSUPPORTED_REPEAT_FIELDS
        .iter()
        .any(|field| repeat.get(*field).is_some())
    {
        return Err(FhirMedicationError::UnsupportedComplexTiming);
    }

    let frequency = repeat
        .get("frequency")
        .and_then(Value::as_u64)
        .ok_or(FhirMedicationError::MissingTypedTiming)
        .and_then(|value| {
            u32::try_from(value).map_err(|_| FhirMedicationError::InvalidTimingFrequency)
        })?;
    let period = repeat
        .get("period")
        .and_then(Value::as_f64)
        .ok_or(FhirMedicationError::MissingTypedTiming)?;
    let period_unit = repeat
        .get("periodUnit")
        .and_then(Value::as_str)
        .ok_or(FhirMedicationError::MissingTypedTiming)?;

    Ok(AdministrationTiming::Scheduled {
        frequency,
        period: Quantity {
            value: period,
            display_unit: Some(period_unit.to_string()),
            system: UCUM_SYSTEM.to_string(),
            code: period_unit.to_string(),
        },
    })
}

fn parse_dose_amount(value: &Value) -> Result<DoseAmount, FhirMedicationError> {
    match (value.get("doseQuantity"), value.get("doseRange")) {
        (Some(_), Some(_)) => Err(FhirMedicationError::ConflictingDoseChoice),
        (Some(quantity), None) => Ok(DoseAmount::Quantity(parse_quantity(quantity)?)),
        (None, Some(range)) => Ok(DoseAmount::Range(parse_range(range)?)),
        (None, None) => Err(FhirMedicationError::MissingDose),
    }
}

fn parse_rate_amount(value: &Value) -> Result<Option<RateAmount>, FhirMedicationError> {
    let choices = [
        value.get("rateQuantity").is_some(),
        value.get("rateRange").is_some(),
        value.get("rateRatio").is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if choices > 1 {
        return Err(FhirMedicationError::ConflictingRateChoice);
    }
    if let Some(quantity) = value.get("rateQuantity") {
        return Ok(Some(RateAmount::Quantity(parse_quantity(quantity)?)));
    }
    if let Some(range) = value.get("rateRange") {
        return Ok(Some(RateAmount::Range(parse_range(range)?)));
    }
    if let Some(ratio) = value.get("rateRatio") {
        return Ok(Some(RateAmount::Ratio(parse_ratio(ratio)?)));
    }
    Ok(None)
}

fn parse_dispense_request(
    value: Option<&Value>,
) -> Result<(Option<Quantity>, Option<u32>, Option<DispenseValidityPeriod>), FhirMedicationError>
{
    let Some(value) = value else {
        return Ok((None, None, None));
    };
    if value.get("expectedSupplyDuration").is_some() || value.get("performer").is_some() {
        return Err(FhirMedicationError::UnsupportedSafetySignificantField(
            "dispenseRequest.expectedSupplyDuration/performer",
        ));
    }

    let quantity = value.get("quantity").map(parse_quantity).transpose()?;
    let repeats = value
        .get("numberOfRepeatsAllowed")
        .and_then(Value::as_u64)
        .map(|number| {
            u32::try_from(number).map_err(|_| FhirMedicationError::InvalidRefillCount)
        })
        .transpose()?;
    let validity = value
        .get("validityPeriod")
        .map(parse_dispense_validity_period)
        .transpose()?;
    Ok((quantity, repeats, validity))
}

fn parse_dispense_validity_period(value: &Value) -> Result<DispenseValidityPeriod, FhirMedicationError> {
    let period = DispenseValidityPeriod {
        start_micros: value
            .get("start")
            .and_then(Value::as_str)
            .map(parse_datetime)
            .transpose()?,
        end_micros: value
            .get("end")
            .and_then(Value::as_str)
            .map(parse_datetime)
            .transpose()?,
    };
    period.validate()?;
    Ok(period)
}

fn parse_external_identifier(value: &Value) -> Result<ExternalIdentifier, FhirMedicationError> {
    Ok(ExternalIdentifier {
        system: value
            .get("system")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or(FhirMedicationError::InvalidRequesterIdentifier)?
            .to_string(),
        value: value
            .get("value")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or(FhirMedicationError::InvalidRequesterIdentifier)?
            .to_string(),
    })
}

fn parse_quantity(value: &Value) -> Result<Quantity, FhirMedicationError> {
    if value.get("comparator").is_some() {
        return Err(FhirMedicationError::UnsupportedQuantityComparator);
    }
    let quantity = Quantity {
        value: value
            .get("value")
            .and_then(Value::as_f64)
            .ok_or(FhirMedicationError::InvalidQuantity)?,
        display_unit: value.get("unit").and_then(Value::as_str).map(str::to_string),
        system: value
            .get("system")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        code: value
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    };
    quantity.validate_machine_actionable()?;
    Ok(quantity)
}

fn parse_range(value: &Value) -> Result<Range, FhirMedicationError> {
    let range = Range {
        low: value.get("low").map(parse_quantity).transpose()?,
        high: value.get("high").map(parse_quantity).transpose()?,
    };
    ClinicalValue::Range(range.clone()).validate_machine_actionable()?;
    Ok(range)
}

fn parse_ratio(value: &Value) -> Result<Ratio, FhirMedicationError> {
    let ratio = Ratio {
        numerator: parse_quantity(
            value
                .get("numerator")
                .ok_or(FhirMedicationError::InvalidRatio)?,
        )?,
        denominator: parse_quantity(
            value
                .get("denominator")
                .ok_or(FhirMedicationError::InvalidRatio)?,
        )?,
    };
    ClinicalValue::Ratio(ratio.clone()).validate_machine_actionable()?;
    Ok(ratio)
}

fn parse_codeable_concept(value: &Value) -> Result<CodeableConcept, FhirMedicationError> {
    let coding = value
        .get("coding")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| Coding {
                    system: item
                        .get("system")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    code: item
                        .get("code")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    display: item.get("display").and_then(Value::as_str).map(str::to_string),
                    version: item.get("version").and_then(Value::as_str).map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default();
    let concept = CodeableConcept {
        coding,
        text: value.get("text").and_then(Value::as_str).map(str::to_string),
    };
    concept.validate_machine_actionable()?;
    Ok(concept)
}

fn parse_annotation_texts(value: Option<&Value>) -> Result<Vec<String>, FhirMedicationError> {
    let Some(values) = value.and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    values
        .iter()
        .map(|annotation| {
            annotation
                .get("text")
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
                .map(str::to_string)
                .ok_or(FhirMedicationError::InvalidAnnotation)
        })
        .collect()
}

fn required_string(resource: &Value, field: &'static str) -> Result<String, FhirMedicationError> {
    resource
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or(FhirMedicationError::MissingField(field))
}

fn parse_datetime(value: &str) -> Result<i64, FhirMedicationError> {
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.timestamp_micros())
        .map_err(|_| FhirMedicationError::InvalidDateTime(value.to_string()))
}

fn normalize_source_system(value: &str) -> Result<String, FhirMedicationError> {
    let normalized = value.trim().trim_end_matches('/');
    if normalized.is_empty() {
        return Err(FhirMedicationError::EmptySourceSystem);
    }
    Ok(normalized.to_string())
}

#[derive(Debug, Error)]
pub enum FhirMedicationError {
    #[error("expected FHIR MedicationRequest resource")]
    WrongResourceType,
    #[error("FHIR source system is empty")]
    EmptySourceSystem,
    #[error("required FHIR field is missing: {0}")]
    MissingField(&'static str),
    #[error("unsupported MedicationRequest status: {0}")]
    UnsupportedStatus(String),
    #[error("unsupported MedicationRequest intent: {0}")]
    UnsupportedIntent(String),
    #[error("MedicationRequest subject mismatch: expected Patient/{expected}, found Patient/{found}")]
    SubjectMismatch { expected: String, found: String },
    #[error("MedicationRequest subject could not be resolved safely: {reference:?} ({reason})")]
    SubjectUnresolved {
        reference: Option<String>,
        reason: String,
    },
    #[error("MedicationRequest.medicationReference is not accepted by strict v1 projection")]
    UnsupportedMedicationReference,
    #[error("active MedicationRequest requires a concrete requester reference")]
    RequesterReferenceRequired,
    #[error("requester identifier must contain non-empty system and value")]
    InvalidRequesterIdentifier,
    #[error("active MedicationRequest requires non-empty typed dosageInstruction")]
    MissingTypedDosage,
    #[error("dosageInstruction requires typed timing")]
    MissingTypedTiming,
    #[error("unsupported complex FHIR Timing in strict v1 medication projection")]
    UnsupportedComplexTiming,
    #[error("asNeededBoolean=true is insufficient; strict v1 requires coded asNeededCodeableConcept")]
    UncodedAsNeeded,
    #[error("FHIR asNeeded[x] choice is conflicting")]
    ConflictingAsNeededChoice,
    #[error("coded PRN dosage with additional Timing constraints is not yet modeled in v1")]
    UnsupportedPrnTimingConstraint,
    #[error("timing frequency does not fit supported unsigned 32-bit range")]
    InvalidTimingFrequency,
    #[error("dosageInstruction requires exactly one doseAndRate element in strict v1")]
    UnsupportedDoseAndRateCardinality,
    #[error("dosageInstruction dose is missing")]
    MissingDose,
    #[error("dosageInstruction contains both doseQuantity and doseRange")]
    ConflictingDoseChoice,
    #[error("dosageInstruction contains multiple rate[x] choices")]
    ConflictingRateChoice,
    #[error("dosage sequence is outside supported range")]
    InvalidSequence,
    #[error("FHIR quantity requires a numeric value")]
    InvalidQuantity,
    #[error("FHIR quantity comparator cannot be represented losslessly by strict v1")]
    UnsupportedQuantityComparator,
    #[error("FHIR ratio requires numerator and denominator")]
    InvalidRatio,
    #[error("FHIR reasonReference is not projected by strict v1")]
    UnsupportedReasonReference,
    #[error("unsupported safety-significant field in strict v1: {0}")]
    UnsupportedSafetySignificantField(&'static str),
    #[error("dispense validityPeriod must contain start and/or end")]
    EmptyDispenseValidityPeriod,
    #[error("dispense validityPeriod end must be after start")]
    InvalidDispenseValidityPeriod,
    #[error("dispense validity period has not started")]
    DispenseValidityNotStarted,
    #[error("dispense validity period has expired")]
    DispenseValidityExpired,
    #[error("refill count exceeds supported range")]
    InvalidRefillCount,
    #[error("invalid FHIR dateTime: {0}")]
    InvalidDateTime(String),
    #[error("FHIR Annotation.note requires non-empty text")]
    InvalidAnnotation,
    #[error("failed to serialize exact medication artifact: {0}")]
    ArtifactSerialization(String),
    #[error(transparent)]
    ClinicalSemantics(#[from] ClinicalSemanticsError),
    #[error(transparent)]
    MedicationSemantics(#[from] MedicationSemanticsError),
    #[error(transparent)]
    PrincipalResolution(#[from] PrincipalResolutionError),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_integrity::hash_canonical_bytes;
    use mycelix_provider_principal::{
        ExternalReferenceBinding, ExternalResourceKey, VerifiedProviderPrincipalBinding,
    };
    use serde_json::json;

    fn digest(seed: u8) -> VerifiedDigest {
        hash_canonical_bytes(DigestDomain::ClinicalArtifact, &[seed]).unwrap()
    }

    fn binding() -> VerifiedProviderPrincipalBinding {
        VerifiedProviderPrincipalBinding::from_verified_provider_record(
            PrincipalBinding([7u8; 32]),
            vec![ExternalReferenceBinding {
                source_system: "https://ehr.example/fhir".into(),
                resource: ExternalResourceKey {
                    resource_type: "Practitioner".into(),
                    id: "prac-1".into(),
                },
            }],
            vec![ExternalIdentifier {
                system: "http://hl7.org/fhir/sid/us-npi".into(),
                value: "1234567893".into(),
            }],
            0,
            Some(9_000_000_000_000_000),
            None,
            digest(1),
            digest(2),
            digest(3),
        )
        .unwrap()
    }

    fn valid_request() -> Value {
        json!({
            "resourceType": "MedicationRequest",
            "id": "rx-1",
            "meta": { "versionId": "4" },
            "status": "active",
            "intent": "order",
            "medicationCodeableConcept": {
                "coding": [{
                    "system": "http://www.nlm.nih.gov/research/umls/rxnorm",
                    "code": "860975",
                    "display": "Example medication"
                }]
            },
            "subject": { "reference": "Patient/patient-a" },
            "authoredOn": "2026-09-10T08:00:00+02:00",
            "requester": {
                "reference": "Practitioner/prac-1",
                "type": "Practitioner",
                "identifier": {
                    "system": "http://hl7.org/fhir/sid/us-npi",
                    "value": "1234567893"
                }
            },
            "reasonCode": [{
                "coding": [{"system": "http://snomed.info/sct", "code": "73211009"}]
            }],
            "dosageInstruction": [{
                "sequence": 1,
                "text": "Take one tablet twice daily",
                "timing": { "repeat": { "frequency": 2, "period": 1, "periodUnit": "d" } },
                "route": {
                    "coding": [{"system": "http://snomed.info/sct", "code": "26643006"}]
                },
                "doseAndRate": [{
                    "doseQuantity": {
                        "value": 1,
                        "unit": "tablet",
                        "system": "http://unitsofmeasure.org",
                        "code": "{tablet}"
                    }
                }],
                "maxDosePerPeriod": {
                    "numerator": {
                        "value": 2,
                        "system": "http://unitsofmeasure.org",
                        "code": "{tablet}"
                    },
                    "denominator": {
                        "value": 1,
                        "system": "http://unitsofmeasure.org",
                        "code": "d"
                    }
                }
            }],
            "dispenseRequest": {
                "numberOfRepeatsAllowed": 1,
                "quantity": {
                    "value": 60,
                    "unit": "tablet",
                    "system": "http://unitsofmeasure.org",
                    "code": "{tablet}"
                },
                "validityPeriod": {
                    "start": "2026-09-10T00:00:00+02:00",
                    "end": "2026-10-10T00:00:00+02:00"
                }
            },
            "note": [{"text": "Reassess after follow-up."}]
        })
    }

    fn project(value: &Value) -> Result<MedicationRequestArtifact, FhirMedicationError> {
        let bindings = vec![binding()];
        project_active_medication_request_strict(
            value,
            "patient-a",
            "https://ehr.example/fhir/",
            1_789_000_000_000_000,
            &bindings,
        )
    }

    fn expect_error(
        result: Result<MedicationRequestArtifact, FhirMedicationError>,
    ) -> FhirMedicationError {
        match result {
            Ok(_) => panic!("expected strict medication projection failure"),
            Err(error) => error,
        }
    }

    #[test]
    fn projects_active_order_with_resolved_requester_and_typed_dose() {
        let artifact = project(&valid_request()).expect("valid request should project");
        assert_eq!(artifact.requester_principal(), PrincipalBinding([7u8; 32]));
        assert_eq!(artifact.order().status, MedicationRequestStatus::Active);
        assert_eq!(artifact.order().intent, MedicationRequestIntent::Order);
        assert_eq!(artifact.indications().len(), 1);
        assert!(artifact.verified_digest().is_ok());
    }

    #[test]
    fn cross_patient_request_is_rejected() {
        let mut value = valid_request();
        value["subject"]["reference"] = Value::String("Patient/patient-b".into());
        assert!(matches!(
            expect_error(project(&value)),
            FhirMedicationError::SubjectMismatch { .. }
        ));
    }

    #[test]
    fn unresolved_requester_is_rejected() {
        let mut value = valid_request();
        value["requester"]["reference"] = Value::String("Practitioner/unknown".into());
        assert!(matches!(
            expect_error(project(&value)),
            FhirMedicationError::PrincipalResolution(
                PrincipalResolutionError::UnresolvedPractitioner
            )
        ));
    }

    #[test]
    fn plan_cannot_cross_active_order_projection() {
        let mut value = valid_request();
        value["intent"] = Value::String("plan".into());
        assert!(matches!(
            expect_error(project(&value)),
            FhirMedicationError::MedicationSemantics(
                MedicationSemanticsError::NotActiveOrderIntent
            )
        ));
    }

    #[test]
    fn free_text_only_sig_is_rejected() {
        let mut value = valid_request();
        value["dosageInstruction"] = json!([{ "text": "Take as directed" }]);
        assert!(matches!(
            expect_error(project(&value)),
            FhirMedicationError::MissingField("dosageInstruction.route")
                | FhirMedicationError::MissingTypedTiming
                | FhirMedicationError::MissingDose
        ));
    }

    #[test]
    fn non_ucum_dose_is_rejected() {
        let mut value = valid_request();
        value["dosageInstruction"][0]["doseAndRate"][0]["doseQuantity"]["system"] =
            Value::String("https://example.invalid/units".into());
        assert!(matches!(
            expect_error(project(&value)),
            FhirMedicationError::ClinicalSemantics(ClinicalSemanticsError::NonUcumQuantity { .. })
        ));
    }

    #[test]
    fn quantity_comparator_is_not_silently_dropped() {
        let mut value = valid_request();
        value["dosageInstruction"][0]["doseAndRate"][0]["doseQuantity"]["comparator"] =
            Value::String("<".into());
        assert!(matches!(
            expect_error(project(&value)),
            FhirMedicationError::UnsupportedQuantityComparator
        ));
    }

    #[test]
    fn uncoded_prn_is_rejected() {
        let mut value = valid_request();
        value["dosageInstruction"][0]
            .as_object_mut()
            .unwrap()
            .remove("timing");
        value["dosageInstruction"][0]["asNeededBoolean"] = Value::Bool(true);
        assert!(matches!(
            expect_error(project(&value)),
            FhirMedicationError::UncodedAsNeeded
        ));
    }

    #[test]
    fn frequency_max_is_not_silently_collapsed_to_frequency() {
        let mut value = valid_request();
        value["dosageInstruction"][0]["timing"]["repeat"]["frequencyMax"] =
            Value::Number(3.into());
        assert!(matches!(
            expect_error(project(&value)),
            FhirMedicationError::UnsupportedComplexTiming
        ));
    }

    #[test]
    fn dispense_validity_is_enforced_at_activation_time() {
        let artifact = project(&valid_request()).unwrap();
        let expired = DateTime::parse_from_rfc3339("2026-10-10T00:00:00+02:00")
            .unwrap()
            .timestamp_micros();
        assert!(matches!(
            artifact.validate_for_activation(expired),
            Err(FhirMedicationError::DispenseValidityExpired)
        ));
    }

    #[test]
    fn substitution_is_not_silently_discarded() {
        let mut value = valid_request();
        value["substitution"] = json!({"allowedBoolean": false});
        assert!(matches!(
            expect_error(project(&value)),
            FhirMedicationError::UnsupportedSafetySignificantField("substitution")
        ));
    }
}
