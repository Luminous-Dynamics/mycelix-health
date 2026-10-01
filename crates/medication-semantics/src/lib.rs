#![deny(unsafe_code)]
//! Typed medication-order semantics for Mycelix-Health.
//!
//! This crate separates a structured medication request from free-text SIG
//! instructions and, critically, separates FHIR request *intent* from clinical
//! authority. An `order` intent can be an order-like request, but this crate does
//! not verify practitioner credentials or authorize dispensing/administration.

use mycelix_clinical_semantics::{
    ClinicalSemanticsError, ClinicalValue, CodeableConcept, Quantity, Range, Ratio, SubjectRef,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MedicationRequestStatus {
    Active,
    OnHold,
    Cancelled,
    Completed,
    EnteredInError,
    Stopped,
    Draft,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MedicationRequestIntent {
    Proposal,
    Plan,
    Order,
    OriginalOrder,
    ReflexOrder,
    FillerOrder,
    InstanceOrder,
    Option,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MedicationWorkflowClass {
    /// Suggestion/request that does not itself represent an active order.
    NonAuthorizingProposal,
    /// Planned future action, not an active medication order.
    NonAuthorizingPlan,
    /// Optional order set choice, not an active medication order by itself.
    NonAuthorizingOption,
    /// Active order-like request. External practitioner authority still must be verified.
    ActiveOrderIntent,
    /// Order-like request currently on hold.
    OnHoldOrderIntent,
    /// Historical/inactive order-like request.
    InactiveOrderIntent,
    /// Draft has not crossed the order workflow boundary.
    Draft,
    /// Source explicitly says this request should never have existed.
    EnteredInError,
    /// Source cannot establish current status.
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DoseAmount {
    Quantity(Quantity),
    Range(Range),
}

impl DoseAmount {
    fn validate(&self) -> Result<(), MedicationSemanticsError> {
        match self {
            Self::Quantity(quantity) => quantity.validate_machine_actionable()?,
            Self::Range(range) => ClinicalValue::Range(range.clone()).validate_machine_actionable()?,
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RateAmount {
    Quantity(Quantity),
    Range(Range),
    Ratio(Ratio),
}

impl RateAmount {
    fn validate(&self) -> Result<(), MedicationSemanticsError> {
        match self {
            Self::Quantity(quantity) => quantity.validate_machine_actionable()?,
            Self::Range(range) => ClinicalValue::Range(range.clone()).validate_machine_actionable()?,
            Self::Ratio(ratio) => ClinicalValue::Ratio(ratio.clone()).validate_machine_actionable()?,
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AdministrationTiming {
    /// Give exactly once.
    OneTime,
    /// Give `frequency` times per UCUM-coded period (e.g. 2 times / 1 day).
    Scheduled {
        frequency: u32,
        period: Quantity,
    },
    /// Give only when the coded clinical indication applies.
    AsNeeded {
        indication: CodeableConcept,
        min_interval: Option<Quantity>,
    },
}

impl AdministrationTiming {
    fn validate(&self) -> Result<(), MedicationSemanticsError> {
        match self {
            Self::OneTime => Ok(()),
            Self::Scheduled { frequency, period } => {
                if *frequency == 0 {
                    return Err(MedicationSemanticsError::ZeroFrequency);
                }
                validate_time_quantity(period)
            }
            Self::AsNeeded {
                indication,
                min_interval,
            } => {
                indication.validate_machine_actionable()?;
                if let Some(interval) = min_interval {
                    validate_time_quantity(interval)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DosageInstruction {
    pub sequence: Option<i32>,
    /// Human-readable SIG retained for display/audit. It is never a substitute
    /// for the typed fields below at the strict machine-actionable boundary.
    pub narrative_sig: Option<String>,
    pub patient_instruction: Option<String>,
    pub timing: AdministrationTiming,
    pub route: CodeableConcept,
    pub method: Option<CodeableConcept>,
    pub site: Option<CodeableConcept>,
    pub dose: DoseAmount,
    pub rate: Option<RateAmount>,
    pub max_dose_per_period: Option<Ratio>,
    pub max_dose_per_administration: Option<Quantity>,
    pub max_dose_per_lifetime: Option<Quantity>,
}

impl DosageInstruction {
    pub fn validate_machine_actionable(&self) -> Result<(), MedicationSemanticsError> {
        if matches!(self.sequence, Some(sequence) if sequence < 0) {
            return Err(MedicationSemanticsError::InvalidSequence);
        }
        self.route.validate_machine_actionable()?;
        if let Some(method) = &self.method {
            method.validate_machine_actionable()?;
        }
        if let Some(site) = &self.site {
            site.validate_machine_actionable()?;
        }
        self.dose.validate()?;
        self.timing.validate()?;
        if let Some(rate) = &self.rate {
            rate.validate()?;
        }
        if let Some(maximum) = &self.max_dose_per_period {
            ClinicalValue::Ratio(maximum.clone()).validate_machine_actionable()?;
            validate_time_quantity(&maximum.denominator)?;
        }
        if let Some(maximum) = &self.max_dose_per_administration {
            maximum.validate_machine_actionable()?;
        }
        if let Some(maximum) = &self.max_dose_per_lifetime {
            maximum.validate_machine_actionable()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequesterAuthorityRef {
    /// Practitioner/PractitionerRole/Organization reference supplied by the source.
    pub requester_reference: String,
    /// Optional Mycelix credential/attestation reference. Presence does not itself
    /// prove validity; the authority layer must verify it before action.
    pub credential_reference: Option<String>,
}

impl RequesterAuthorityRef {
    fn validate(&self) -> Result<(), MedicationSemanticsError> {
        if self.requester_reference.trim().is_empty() {
            return Err(MedicationSemanticsError::MissingRequester);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MedicationOrderProvenance {
    pub source_system: String,
    pub source_resource_id: String,
    pub source_version: Option<String>,
    pub recorded_at_micros: i64,
}

impl MedicationOrderProvenance {
    fn validate(&self) -> Result<(), MedicationSemanticsError> {
        if self.source_system.trim().is_empty() || self.source_resource_id.trim().is_empty() {
            return Err(MedicationSemanticsError::IncompleteProvenance);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MedicationOrder {
    pub order_id: String,
    pub subject: SubjectRef,
    pub medication: CodeableConcept,
    /// Optional typed product strength such as 500 mg / 1 tablet.
    pub product_strength: Option<Ratio>,
    pub status: MedicationRequestStatus,
    pub intent: MedicationRequestIntent,
    pub dosage: Vec<DosageInstruction>,
    pub dispense_quantity: Option<Quantity>,
    pub refills_authorized: Option<u32>,
    pub requester: Option<RequesterAuthorityRef>,
    pub authored_at_micros: Option<i64>,
    pub provenance: MedicationOrderProvenance,
}

impl MedicationOrder {
    /// Validate that medication semantics are explicit enough for deterministic
    /// interpretation. This does not prove clinical correctness or requester authority.
    pub fn validate_machine_actionable(&self) -> Result<(), MedicationSemanticsError> {
        if self.order_id.trim().is_empty() {
            return Err(MedicationSemanticsError::MissingOrderId);
        }
        if self.subject.resource_type.trim().is_empty() || self.subject.id.trim().is_empty() {
            return Err(MedicationSemanticsError::InvalidSubject);
        }
        self.medication.validate_machine_actionable()?;
        self.provenance.validate()?;

        if let Some(strength) = &self.product_strength {
            ClinicalValue::Ratio(strength.clone()).validate_machine_actionable()?;
        }
        if let Some(quantity) = &self.dispense_quantity {
            quantity.validate_machine_actionable()?;
        }

        // An active/on-hold order-like request with no typed dosage is unsafe to
        // interpret even when a free-text SIG exists elsewhere.
        if matches!(
            self.workflow_class(),
            MedicationWorkflowClass::ActiveOrderIntent | MedicationWorkflowClass::OnHoldOrderIntent
        ) && self.dosage.is_empty()
        {
            return Err(MedicationSemanticsError::MissingTypedDosage);
        }

        for dosage in &self.dosage {
            dosage.validate_machine_actionable()?;
        }

        if matches!(
            self.workflow_class(),
            MedicationWorkflowClass::ActiveOrderIntent | MedicationWorkflowClass::OnHoldOrderIntent
        ) {
            self.requester
                .as_ref()
                .ok_or(MedicationSemanticsError::MissingRequester)?
                .validate()?;
        }

        Ok(())
    }

    /// Classify workflow meaning without pretending that a source `intent=order`
    /// is itself cryptographic/legal proof of prescriber authority.
    pub fn workflow_class(&self) -> MedicationWorkflowClass {
        if self.status == MedicationRequestStatus::EnteredInError {
            return MedicationWorkflowClass::EnteredInError;
        }
        if self.status == MedicationRequestStatus::Unknown {
            return MedicationWorkflowClass::Unknown;
        }
        if self.status == MedicationRequestStatus::Draft {
            return MedicationWorkflowClass::Draft;
        }

        match self.intent {
            MedicationRequestIntent::Proposal => MedicationWorkflowClass::NonAuthorizingProposal,
            MedicationRequestIntent::Plan => MedicationWorkflowClass::NonAuthorizingPlan,
            MedicationRequestIntent::Option => MedicationWorkflowClass::NonAuthorizingOption,
            MedicationRequestIntent::Order
            | MedicationRequestIntent::OriginalOrder
            | MedicationRequestIntent::ReflexOrder
            | MedicationRequestIntent::FillerOrder
            | MedicationRequestIntent::InstanceOrder => match self.status {
                MedicationRequestStatus::Active => MedicationWorkflowClass::ActiveOrderIntent,
                MedicationRequestStatus::OnHold => MedicationWorkflowClass::OnHoldOrderIntent,
                MedicationRequestStatus::Cancelled
                | MedicationRequestStatus::Completed
                | MedicationRequestStatus::Stopped => MedicationWorkflowClass::InactiveOrderIntent,
                MedicationRequestStatus::EnteredInError => MedicationWorkflowClass::EnteredInError,
                MedicationRequestStatus::Draft => MedicationWorkflowClass::Draft,
                MedicationRequestStatus::Unknown => MedicationWorkflowClass::Unknown,
            },
        }
    }

    /// Strict precondition for entering a later prescription-authority workflow.
    /// The caller must still verify practitioner credentials, scope, signatures,
    /// jurisdiction, and any controlled-substance requirements.
    pub fn validate_active_order_candidate(&self) -> Result<(), MedicationSemanticsError> {
        self.validate_machine_actionable()?;
        if self.workflow_class() != MedicationWorkflowClass::ActiveOrderIntent {
            return Err(MedicationSemanticsError::NotActiveOrderIntent);
        }
        Ok(())
    }
}

fn validate_time_quantity(quantity: &Quantity) -> Result<(), MedicationSemanticsError> {
    quantity.validate_machine_actionable()?;
    if !matches!(quantity.code.as_str(), "s" | "min" | "h" | "d" | "wk" | "mo" | "a") {
        return Err(MedicationSemanticsError::NonTimePeriodUnit(
            quantity.code.clone(),
        ));
    }
    if quantity.value <= 0.0 {
        return Err(MedicationSemanticsError::NonPositiveTimePeriod);
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MedicationSemanticsError {
    #[error("medication order id is required")]
    MissingOrderId,
    #[error("medication subject must have resource type and id")]
    InvalidSubject,
    #[error("active/on-hold order-like requests require typed dosage semantics")]
    MissingTypedDosage,
    #[error("active/on-hold order-like requests require a requester reference")]
    MissingRequester,
    #[error("medication order provenance is incomplete")]
    IncompleteProvenance,
    #[error("dosage sequence cannot be negative")]
    InvalidSequence,
    #[error("scheduled dosage frequency must be greater than zero")]
    ZeroFrequency,
    #[error("timing period must use a UCUM time unit, got {0}")]
    NonTimePeriodUnit(String),
    #[error("timing period must be positive")]
    NonPositiveTimePeriod,
    #[error("medication request is not an active order intent")]
    NotActiveOrderIntent,
    #[error(transparent)]
    ClinicalSemantics(#[from] ClinicalSemanticsError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_semantics::{Coding, UCUM_SYSTEM};

    fn concept(system: &str, code: &str) -> CodeableConcept {
        CodeableConcept {
            coding: vec![Coding {
                system: system.into(),
                code: code.into(),
                display: None,
                version: None,
            }],
            text: None,
        }
    }

    fn quantity(value: f64, code: &str) -> Quantity {
        Quantity {
            value,
            display_unit: Some(code.into()),
            system: UCUM_SYSTEM.into(),
            code: code.into(),
        }
    }

    fn valid_order() -> MedicationOrder {
        MedicationOrder {
            order_id: "medreq-1".into(),
            subject: SubjectRef {
                resource_type: "Patient".into(),
                id: "patient-a".into(),
            },
            medication: concept("http://www.nlm.nih.gov/research/umls/rxnorm", "860975"),
            product_strength: Some(Ratio {
                numerator: quantity(500.0, "mg"),
                denominator: quantity(1.0, "{tablet}"),
            }),
            status: MedicationRequestStatus::Active,
            intent: MedicationRequestIntent::Order,
            dosage: vec![DosageInstruction {
                sequence: Some(1),
                narrative_sig: Some("Take one tablet twice daily".into()),
                patient_instruction: None,
                timing: AdministrationTiming::Scheduled {
                    frequency: 2,
                    period: quantity(1.0, "d"),
                },
                route: concept("http://snomed.info/sct", "26643006"),
                method: None,
                site: None,
                dose: DoseAmount::Quantity(quantity(1.0, "{tablet}")),
                rate: None,
                max_dose_per_period: Some(Ratio {
                    numerator: quantity(2.0, "{tablet}"),
                    denominator: quantity(1.0, "d"),
                }),
                max_dose_per_administration: Some(quantity(1.0, "{tablet}")),
                max_dose_per_lifetime: None,
            }],
            dispense_quantity: Some(quantity(60.0, "{tablet}")),
            refills_authorized: Some(1),
            requester: Some(RequesterAuthorityRef {
                requester_reference: "Practitioner/123".into(),
                credential_reference: Some("mycelix:credential:abc".into()),
            }),
            authored_at_micros: Some(1_700_000_000_000_000),
            provenance: MedicationOrderProvenance {
                source_system: "https://ehr.example/fhir".into(),
                source_resource_id: "medreq-1".into(),
                source_version: Some("7".into()),
                recorded_at_micros: 1_700_000_000_100_000,
            },
        }
    }

    #[test]
    fn typed_active_order_is_machine_actionable_candidate() {
        let order = valid_order();
        assert_eq!(order.validate_machine_actionable(), Ok(()));
        assert_eq!(order.workflow_class(), MedicationWorkflowClass::ActiveOrderIntent);
        assert_eq!(order.validate_active_order_candidate(), Ok(()));
    }

    #[test]
    fn proposal_never_becomes_active_order_intent() {
        let mut order = valid_order();
        order.intent = MedicationRequestIntent::Proposal;
        assert_eq!(
            order.workflow_class(),
            MedicationWorkflowClass::NonAuthorizingProposal
        );
        assert_eq!(
            order.validate_active_order_candidate(),
            Err(MedicationSemanticsError::NotActiveOrderIntent)
        );
    }

    #[test]
    fn entered_in_error_never_becomes_active_order_intent() {
        let mut order = valid_order();
        order.status = MedicationRequestStatus::EnteredInError;
        assert_eq!(order.workflow_class(), MedicationWorkflowClass::EnteredInError);
        assert_eq!(
            order.validate_active_order_candidate(),
            Err(MedicationSemanticsError::NotActiveOrderIntent)
        );
    }

    #[test]
    fn narrative_sig_cannot_replace_typed_dose_and_timing() {
        let mut order = valid_order();
        order.dosage.clear();
        assert_eq!(
            order.validate_machine_actionable(),
            Err(MedicationSemanticsError::MissingTypedDosage)
        );
    }

    #[test]
    fn non_ucum_dose_is_rejected() {
        let mut order = valid_order();
        let DoseAmount::Quantity(dose) = &mut order.dosage[0].dose else {
            panic!("expected quantity dose")
        };
        dose.system = "http://example.invalid/units".into();
        assert!(matches!(
            order.validate_machine_actionable(),
            Err(MedicationSemanticsError::ClinicalSemantics(
                ClinicalSemanticsError::NonUcumQuantity { .. }
            ))
        ));
    }

    #[test]
    fn zero_frequency_is_rejected() {
        let mut order = valid_order();
        order.dosage[0].timing = AdministrationTiming::Scheduled {
            frequency: 0,
            period: quantity(1.0, "d"),
        };
        assert_eq!(
            order.validate_machine_actionable(),
            Err(MedicationSemanticsError::ZeroFrequency)
        );
    }

    #[test]
    fn dose_unit_cannot_masquerade_as_timing_period() {
        let mut order = valid_order();
        order.dosage[0].timing = AdministrationTiming::Scheduled {
            frequency: 2,
            period: quantity(1.0, "mg"),
        };
        assert_eq!(
            order.validate_machine_actionable(),
            Err(MedicationSemanticsError::NonTimePeriodUnit("mg".into()))
        );
    }

    #[test]
    fn as_needed_timing_requires_coded_indication() {
        let mut order = valid_order();
        order.dosage[0].timing = AdministrationTiming::AsNeeded {
            indication: CodeableConcept {
                coding: vec![],
                text: Some("when needed".into()),
            },
            min_interval: None,
        };
        assert!(matches!(
            order.validate_machine_actionable(),
            Err(MedicationSemanticsError::ClinicalSemantics(
                ClinicalSemanticsError::UncodedConcept
            ))
        ));
    }

    #[test]
    fn max_dose_period_denominator_must_be_time() {
        let mut order = valid_order();
        order.dosage[0].max_dose_per_period = Some(Ratio {
            numerator: quantity(2.0, "{tablet}"),
            denominator: quantity(1.0, "mg"),
        });
        assert_eq!(
            order.validate_machine_actionable(),
            Err(MedicationSemanticsError::NonTimePeriodUnit("mg".into()))
        );
    }

    #[test]
    fn on_hold_order_is_not_active_candidate() {
        let mut order = valid_order();
        order.status = MedicationRequestStatus::OnHold;
        assert_eq!(
            order.workflow_class(),
            MedicationWorkflowClass::OnHoldOrderIntent
        );
        assert_eq!(
            order.validate_active_order_candidate(),
            Err(MedicationSemanticsError::NotActiveOrderIntent)
        );
    }

    #[test]
    fn active_order_requires_requester_reference() {
        let mut order = valid_order();
        order.requester = None;
        assert_eq!(
            order.validate_machine_actionable(),
            Err(MedicationSemanticsError::MissingRequester)
        );
    }
}
