// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic-only, fail-closed FHIR R4 Claim projection into review-only claim evidence.
//!
//! This mapper intentionally supports a narrow, declared subset. It does not
//! fetch resources, establish the legality of processing, verify a publisher,
//! or authorize a clinical, coverage, payment, or enforcement decision.

use claims_integrity_core::{
    evaluate, ClaimEvidenceBundleV1, ClaimLineStatus, ClaimLineV1, EvaluationReportV1,
    EvaluationReportV1, InputCompleteness,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const ADAPTER_SCHEMA_VERSION: u16 = 1;
pub const MAPPING_PROFILE_VERSION: &str = "fhir-r4-claim-supported-subset-v1";
pub const CLAIM_PROFILE_CANONICAL: &str =
    "urn:luminous-dynamics:synthetic:fhir-r4:claim-review-v1";
pub const CLAIM_RESPONSE_PROFILE_CANONICAL: &str =
    "urn:luminous-dynamics:synthetic:fhir-r4:claimresponse-linkage-v1";
const POLICY_DOMAIN: &str =
    "luminous-dynamics/fhir-r4-claim-supported-subset-v1;fhir=4.0.1;";

/// Explicit mapping policy. Currency scales must be supplied by a reviewed
/// policy; the adapter never guesses minor-unit precision from a currency code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MappingPolicyV1 {
    pub code_set_version: String,
    pub currency_minor_unit_scales: BTreeMap<String, u8>,
}

impl MappingPolicyV1 {
    pub fn digest_sha256(&self) -> Result<String, AdapterError> {
        if !is_identifier(&self.code_set_version) || self.currency_minor_unit_scales.is_empty() {
            return Err(AdapterError::InvalidMappingPolicy);
        }
        let mut canonical = format!(
            "{POLICY_DOMAIN}mapping_profile={MAPPING_PROFILE_VERSION};code_set={};",
            self.code_set_version
        );
        for (currency, scale) in &self.currency_minor_unit_scales {
            if !is_currency_code(currency) || *scale > 6 {
                return Err(AdapterError::InvalidMappingPolicy);
            }
            canonical.push_str(currency);
            canonical.push('=');
            canonical.push_str(&scale.to_string());
            canonical.push(';');
        }
        Ok(sha256_hex(canonical.as_bytes()))
    }

    fn scale_for(&self, currency: &str) -> Option<u8> {
        self.currency_minor_unit_scales.get(currency).copied()
    }
}

/// Opaque local references minted at a privacy boundary. The adapter does not
/// derive beneficiary/provider identity from the identifiers in a FHIR payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MappingContextV1 {
    pub source_resource_ref: String,
    pub claim_ref: String,
    pub beneficiary_ref: String,
    pub provider_ref: String,
    /// For ClaimResponse only: an independently resolved opaque local Claim ref.
    pub linked_claim_ref: Option<String>,
}

/// A point within an exact captured FHIR resource payload. This is a local
/// selector hint; it is not yet a qualified Mycelix canonical selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MappedItemRefV1 {
    pub line_id: String,
    pub item_array_index: usize,
    pub item_sequence: u32,
    pub json_pointer: String,
    pub source_payload_sha256: String,
    pub coding_system: String,
    pub coding_version: Option<String>,
    pub coding_code: String,
    pub service_date_yyyymmdd: u32,
    pub amount_currency: String,
    pub amount_minor_units: u64,
}

/// Reasons why an item or resource could not enter the supported subset.
/// Reasons are machine-readable and intentionally do not contain source payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MappingIssueV1 {
    UnsupportedProfile,
    MissingSourceVersion,
    UnsupportedResourceExtension,
    UnsupportedClaimStatus,
    UnsupportedClaimUse,
    UnsupportedClaimContext,
    MissingRequiredClaimFields,
    MissingItems,
    InvalidOrMissingSequence,
    DuplicateSequence,
    UnsupportedItemFields,
    MissingProductOrService,
    MissingOrAmbiguousCoding,
    InvalidCoding,
    MissingServiceDate,
    ServicePeriodNotSupported,
    InvalidServiceDate,
    MissingNetMoney,
    InvalidNetMoney,
    UnsupportedCurrency,
    AmountPrecisionWouldBeLost,
    AmountOutOfRange,
    InvalidOpaqueReference,
    ClaimResponseNotMappedToServiceLines,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MappingStateV1 {
    Complete,
    Partial,
    Indeterminate,
    ResponseReferenceOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnmappedItemV1 {
    pub item_array_index: usize,
    pub item_sequence: Option<u32>,
    pub reason: MappingIssueV1,
    pub unsupported_field_names: Vec<String>,
}

/// Provenance and loss report attached to every mapping result. Raw FHIR IDs
/// are deliberately not copied here; source_resource_ref is a caller-supplied
/// opaque local token. The version/digest pair identifies the input revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterEnvelopeV1 {
    pub schema_version: u16,
    pub resource_type: String,
    pub source_resource_ref: String,
    pub source_version_id: Option<String>,
    pub source_last_updated: Option<String>,
    pub source_payload_sha256: String,
    pub source_payload_byte_length: usize,
    pub mapping_profile_version: String,
    pub policy_sha256: String,
    pub input_manifest_sha256: String,
    pub source_status: Option<String>,
    pub state: MappingStateV1,
    pub global_issues: Vec<MappingIssueV1>,
    pub mapped_items: Vec<MappedItemRefV1>,
    pub unmapped_items: Vec<UnmappedItemV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimAdapterOutputV1 {
    pub envelope: AdapterEnvelopeV1,
    pub bundle: ClaimEvidenceBundleV1,
    pub evaluation: EvaluationReportV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimResponseLinkageStateV1 {
    LinkedViaCallerSuppliedOpaqueRef,
    SourceRequestPresentButUnresolved,
    SourceRequestMissing,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimResponseProjectionV1 {
    pub envelope: AdapterEnvelopeV1,
    pub response_status: Option<String>,
    pub outcome: Option<String>,
    pub request_reference_present: bool,
    pub linked_claim_ref: Option<String>,
    pub linkage_state: ClaimResponseLinkageStateV1,
    /// Always false in V1: adjudication is not mapped into submitted service lines.
    pub service_lines_mapped: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterResultV1 {
    Claim(ClaimAdapterOutputV1),
    ClaimResponse(ClaimResponseProjectionV1),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterError {
    InvalidJson,
    UnsupportedResourceType,
    InvalidMappingPolicy,
    InvalidOpaqueReference,
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson => write!(f, "input is not valid FHIR JSON"),
            Self::UnsupportedResourceType => {
                write!(f, "resource type is not Claim or ClaimResponse")
            },
            Self::InvalidMappingPolicy => write!(f, "mapping policy is invalid or incomplete"),
            Self::InvalidOpaqueReference => {
                write!(f, "caller-supplied opaque reference is invalid")
            },
        }
    }
}

impl std::error::Error for AdapterError {}

/// Parse one exact UTF-8 FHIR JSON payload and either map a supported Claim
/// subset or return a reference-only ClaimResponse projection.
///
/// This function performs no I/O. Inputs must be synthetic during prototype
/// qualification. A valid result is not proof that the payload is authentic,
/// that references resolve, or that processing is legally authorized.
pub fn map_fhir_r4_json(
    raw_payload: &[u8],
    context: &MappingContextV1,
    policy: &MappingPolicyV1,
) -> Result<AdapterResultV1, AdapterError> {
    validate_context(context)?;
    let policy_sha256 = policy.digest_sha256()?;
    let value: Value =
        serde_json::from_slice(raw_payload).map_err(|_| AdapterError::InvalidJson)?;
    let object = value.as_object().ok_or(AdapterError::InvalidJson)?;
    let resource_type = object
        .get("resourceType")
        .and_then(Value::as_str)
        .ok_or(AdapterError::InvalidJson)?;

    let source_payload_sha256 = sha256_hex(raw_payload);
    let (source_version_id, source_last_updated) = source_versions(&value);
    let source_status = value
        .get("status")
        .and_then(Value::as_str)
        .map(str::to_string);
    let input_manifest_sha256 = manifest_digest(
        &context.source_resource_ref,
        source_version_id.as_deref(),
        &source_payload_sha256,
        &policy_sha256,
    );

    match resource_type {
        "Claim" => {
            let (envelope, bundle) = map_claim(
                &value,
                raw_payload.len(),
                context,
                policy,
                &policy_sha256,
                &source_payload_sha256,
                &input_manifest_sha256,
                source_version_id,
                source_last_updated,
                source_status,
            );
            let evaluation = evaluate(&bundle);
            Ok(AdapterResultV1::Claim(ClaimAdapterOutputV1 {
                envelope,
                bundle,
                evaluation,
            }))
        }
        "ClaimResponse" => {
            let mut global_issues = Vec::new();
            if !version_is_present(&source_version_id) {
                global_issues.push(MappingIssueV1::MissingSourceVersion);
            }
            if !profile_matches(&value, CLAIM_RESPONSE_PROFILE_CANONICAL) {
                global_issues.push(MappingIssueV1::UnsupportedProfile);
            }
            if contains_extension(&value) {
                global_issues.push(MappingIssueV1::UnsupportedResourceExtension);
            }
            if unsupported_meta_fields(&value) {
                global_issues.push(MappingIssueV1::UnsupportedClaimContext);
            }
            let request_reference_present = value
                .get("request")
                .and_then(|v| v.get("reference"))
                .and_then(Value::as_str)
                .is_some_and(|reference| !reference.trim().is_empty());
            let linked_claim_ref = if request_reference_present {
                context.linked_claim_ref.clone()
            } else {
                None
            };
            let linkage_state = if !request_reference_present {
                global_issues.push(MappingIssueV1::ClaimResponseNotMappedToServiceLines);
                ClaimResponseLinkageStateV1::SourceRequestMissing
            } else if linked_claim_ref.is_some() {
                ClaimResponseLinkageStateV1::LinkedViaCallerSuppliedOpaqueRef
            } else {
                ClaimResponseLinkageStateV1::SourceRequestPresentButUnresolved
            };
            let envelope = AdapterEnvelopeV1 {
                schema_version: ADAPTER_SCHEMA_VERSION,
                resource_type: "ClaimResponse".to_string(),
                source_resource_ref: context.source_resource_ref.clone(),
                source_version_id,
                source_last_updated,
                source_payload_sha256,
                source_payload_byte_length: raw_payload.len(),
                mapping_profile_version: MAPPING_PROFILE_VERSION.to_string(),
                policy_sha256,
                input_manifest_sha256,
                source_status,
                state: MappingStateV1::ResponseReferenceOnly,
                global_issues,
                mapped_items: Vec::new(),
                unmapped_items: Vec::new(),
            };
            Ok(AdapterResultV1::ClaimResponse(
                ClaimResponseProjectionV1 {
                    envelope,
                    response_status: value
                        .get("status")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    outcome: value
                        .get("outcome")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    request_reference_present,
                    linked_claim_ref,
                    linkage_state,
                    service_lines_mapped: false,
                },
            ))
        }
        _ => Err(AdapterError::UnsupportedResourceType),
    }
}

#[allow(clippy::too_many_arguments)]
fn map_claim(
    value: &Value,
    raw_payload_len: usize,
    context: &MappingContextV1,
    policy: &MappingPolicyV1,
    policy_sha256: &str,
    source_payload_sha256: &str,
    input_manifest_sha256: &str,
    source_version_id: Option<String>,
    source_last_updated: Option<String>,
    source_status: Option<String>,
) -> (AdapterEnvelopeV1, ClaimEvidenceBundleV1) {
    let mut global_issues = Vec::new();
    if !version_is_present(&source_version_id) {
        global_issues.push(MappingIssueV1::MissingSourceVersion);
    }
    if !profile_matches(value, CLAIM_PROFILE_CANONICAL) {
        global_issues.push(MappingIssueV1::UnsupportedProfile);
    }
    if contains_extension(value) {
        global_issues.push(MappingIssueV1::UnsupportedResourceExtension);
    }
    if source_status.as_deref() != Some("active") {
        global_issues.push(MappingIssueV1::UnsupportedClaimStatus);
    }
    if value.get("use").and_then(Value::as_str) != Some("claim") {
        global_issues.push(MappingIssueV1::UnsupportedClaimUse);
    }
    if unsupported_claim_context_fields(value) || unsupported_meta_fields(value) {
        global_issues.push(MappingIssueV1::UnsupportedClaimContext);
    }
    if !required_claim_fields_present(value) {
        global_issues.push(MappingIssueV1::MissingRequiredClaimFields);
    }

    let items = value.get("item").and_then(Value::as_array);
    if items.is_none() || items.is_some_and(Vec::is_empty) {
        global_issues.push(MappingIssueV1::MissingItems);
    }

    let mut mapped_items = Vec::new();
    let mut unmapped_items = Vec::new();
    let mut lines = Vec::new();

    let may_map = global_issues.is_empty();
    if may_map {
        let items = items.expect("mapping gate requires a non-empty item array");
        let mut sequences = BTreeSet::new();
        let mut duplicate_sequence = false;
        for item in items {
            match item.get("sequence").and_then(Value::as_u64) {
                Some(sequence) if sequence > 0 && sequence <= u32::MAX as u64 => {
                    if !sequences.insert(sequence as u32) {
                        duplicate_sequence = true;
                    }
                }
                _ => {}
            }
        }

        if duplicate_sequence {
            global_issues.push(MappingIssueV1::DuplicateSequence);
            for (array_index, item) in items.iter().enumerate() {
                unmapped_items.push(UnmappedItemV1 {
                    item_array_index: array_index,
                    item_sequence: item
                        .get("sequence")
                        .and_then(Value::as_u64)
                        .filter(|sequence| *sequence > 0 && *sequence <= u32::MAX as u64)
                        .map(|sequence| sequence as u32),
                    reason: MappingIssueV1::DuplicateSequence,
                    unsupported_field_names: Vec::new(),
                });
            }
        } else {
            for (array_index, item) in items.iter().enumerate() {
                match map_item(
                    item,
                    array_index,
                    context,
                    policy,
                    source_payload_sha256,
                ) {
                    Ok((line, item_ref)) => {
                        lines.push(line);
                        mapped_items.push(item_ref);
                    }
                    Err(unmapped) => unmapped_items.push(unmapped),
                }
            }
        }
    } else if let Some(items) = items {
        for (array_index, item) in items.iter().enumerate() {
            unmapped_items.push(UnmappedItemV1 {
                item_array_index: array_index,
                item_sequence: item
                    .get("sequence")
                    .and_then(Value::as_u64)
                    .filter(|sequence| *sequence > 0 && *sequence <= u32::MAX as u64)
                    .map(|sequence| sequence as u32),
                reason: global_issues
                    .first()
                    .copied()
                    .unwrap_or(MappingIssueV1::UnsupportedProfile),
                unsupported_field_names: Vec::new(),
            });
        }
    }

    let completeness = if !global_issues.is_empty() || lines.is_empty() {
        InputCompleteness::Unknown
    } else if !unmapped_items.is_empty() {
        InputCompleteness::Partial
    } else {
        InputCompleteness::Complete
    };

    // The claim core enforces its own strict identifier/date/digest contract.
    // This additional guard prevents partially invalid mapping output from
    // reaching it as an apparently complete bundle.
    let bundle = ClaimEvidenceBundleV1 {
        schema_version: claims_integrity_core::SCHEMA_VERSION,
        input_manifest_digest: input_manifest_sha256.to_string(),
        policy_digest: policy_sha256.to_string(),
        code_set_version: policy.code_set_version.clone(),
        completeness,
        lines,
    };

    let state = match completeness {
        InputCompleteness::Complete => MappingStateV1::Complete,
        InputCompleteness::Partial => MappingStateV1::Partial,
        InputCompleteness::Unknown => MappingStateV1::Indeterminate,
    };
    let envelope = AdapterEnvelopeV1 {
        schema_version: ADAPTER_SCHEMA_VERSION,
        resource_type: "Claim".to_string(),
        source_resource_ref: context.source_resource_ref.clone(),
        source_version_id,
        source_last_updated,
        source_payload_sha256: source_payload_sha256.to_string(),
        source_payload_byte_length: raw_payload_len,
        mapping_profile_version: MAPPING_PROFILE_VERSION.to_string(),
        policy_sha256: policy_sha256.to_string(),
        input_manifest_sha256: input_manifest_sha256.to_string(),
        source_status,
        state,
        global_issues,
        mapped_items,
        unmapped_items,
    };
    (envelope, bundle)
}

fn map_item(
    item: &Value,
    array_index: usize,
    context: &MappingContextV1,
    policy: &MappingPolicyV1,
    source_payload_sha256: &str,
) -> Result<(ClaimLineV1, MappedItemRefV1), UnmappedItemV1> {
    let item_sequence = item
        .get("sequence")
        .and_then(Value::as_u64)
        .filter(|sequence| *sequence > 0 && *sequence <= u32::MAX as u64)
        .map(|sequence| sequence as u32);
    let unmapped = |reason, unsupported_field_names| UnmappedItemV1 {
        item_array_index: array_index,
        item_sequence,
        reason,
        unsupported_field_names,
    };

    let Some(object) = item.as_object() else {
        return Err(unmapped(MappingIssueV1::UnsupportedItemFields, Vec::new()));
    };
    let allowed_fields = ["sequence", "productOrService", "servicedDate", "net"];
    let mut unsupported_fields: Vec<String> = object
        .keys()
        .filter(|key| !allowed_fields.contains(&key.as_str()))
        .cloned()
        .collect();
    unsupported_fields.sort();
    unsupported_fields.dedup();
    if !unsupported_fields.is_empty() {
        let reason = if unsupported_fields.iter().any(|field| field == "servicedPeriod") {
            MappingIssueV1::ServicePeriodNotSupported
        } else {
            MappingIssueV1::UnsupportedItemFields
        };
        return Err(unmapped(reason, unsupported_fields));
    }
    let Some(sequence) = item_sequence else {
        return Err(unmapped(
            MappingIssueV1::InvalidOrMissingSequence,
            Vec::new(),
        ));
    };

    let Some(codings) = item
        .get("productOrService")
        .and_then(|value| value.get("coding"))
        .and_then(Value::as_array)
    else {
        return Err(unmapped(
            MappingIssueV1::MissingProductOrService,
            Vec::new(),
        ));
    };
    if codings.len() != 1 {
        return Err(unmapped(
            MappingIssueV1::MissingOrAmbiguousCoding,
            Vec::new(),
        ));
    }
    let coding = &codings[0];
    let system = coding.get("system").and_then(Value::as_str);
    let code = coding.get("code").and_then(Value::as_str);
    let coding_version = coding
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_string);
    let (Some(system), Some(code)) = (system, code) else {
        return Err(unmapped(MappingIssueV1::InvalidCoding, Vec::new()));
    };
    if system.trim().is_empty()
        || system.len() > 512
        || code.trim().is_empty()
        || code.len() > 256
        || coding_version.as_deref().is_some_and(|version| {
            version.trim().is_empty() || version.len() > 128
        })
    {
        return Err(unmapped(MappingIssueV1::InvalidCoding, Vec::new()));
    }

    let Some(date_text) = item.get("servicedDate").and_then(Value::as_str) else {
        return Err(unmapped(MappingIssueV1::MissingServiceDate, Vec::new()));
    };
    let Some(service_date_yyyymmdd) = parse_iso_date(date_text) else {
        return Err(unmapped(MappingIssueV1::InvalidServiceDate, Vec::new()));
    };

    let Some(net) = item.get("net") else {
        return Err(unmapped(MappingIssueV1::MissingNetMoney, Vec::new()));
    };
    let Some(value_number) = net.get("value").and_then(Value::as_number) else {
        return Err(unmapped(MappingIssueV1::InvalidNetMoney, Vec::new()));
    };
    let Some(currency) = net.get("currency").and_then(Value::as_str) else {
        return Err(unmapped(MappingIssueV1::InvalidNetMoney, Vec::new()));
    };
    let Some(scale) = policy.scale_for(currency) else {
        return Err(unmapped(MappingIssueV1::UnsupportedCurrency, Vec::new()));
    };
    let amount_text = value_number.to_string();
    let amount_minor_units = match decimal_to_minor_units(&amount_text, scale) {
        Ok(amount) => amount,
        Err(DecimalError::PrecisionLoss) => {
            return Err(unmapped(
                MappingIssueV1::AmountPrecisionWouldBeLost,
                Vec::new(),
            ));
        }
        Err(DecimalError::InvalidOrOutOfRange) => {
            return Err(unmapped(MappingIssueV1::AmountOutOfRange, Vec::new()));
        }
    };

    // SHA-256 separates service-code systems/versions without collapsing equal
    // code strings from different terminology systems into one duplicate key.
    let code_identity = format!(
        "fhir-service-coding-v1\0{}\0{}\0{}",
        system,
        coding_version.as_deref().unwrap_or(""),
        code
    );
    let service_code = format!("fhir-{}", sha256_hex(code_identity.as_bytes()));
    let line_id = format!("claim-line-{sequence}");
    let line = ClaimLineV1 {
        line_id: line_id.clone(),
        claim_id: context.claim_ref.clone(),
        beneficiary_ref: context.beneficiary_ref.clone(),
        provider_ref: context.provider_ref.clone(),
        service_code,
        currency_code: currency.to_string(),
        service_date_yyyymmdd,
        amount_minor_units,
        status: ClaimLineStatus::Submitted,
        reverses_line_id: None,
        source_record_digest: source_payload_sha256.to_string(),
    };
    let item_ref = MappedItemRefV1 {
        line_id,
        item_array_index: array_index,
        item_sequence: sequence,
        json_pointer: format!("/item/{array_index}"),
        source_payload_sha256: source_payload_sha256.to_string(),
        coding_system: system.to_string(),
        coding_version,
        coding_code: code.to_string(),
        service_date_yyyymmdd,
        amount_currency: currency.to_string(),
        amount_minor_units,
    };
    Ok((line, item_ref))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DecimalError {
    PrecisionLoss,
    InvalidOrOutOfRange,
}

fn decimal_to_minor_units(value: &str, scale: u8) -> Result<u64, DecimalError> {
    if value.is_empty()
        || value.starts_with('-')
        || value.starts_with('+')
        || value.contains('e')
        || value.contains('E')
    {
        return Err(DecimalError::InvalidOrOutOfRange);
    }
    if scale > 6 {
        return Err(DecimalError::InvalidOrOutOfRange);
    }
    let mut parts = value.split('.');
    let whole = parts.next().ok_or(DecimalError::InvalidOrOutOfRange)?;
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(DecimalError::InvalidOrOutOfRange);
    }
    let scale_len = scale as usize;
    let (fraction_kept, excess) = if fraction.len() > scale_len {
        fraction.split_at(scale_len)
    } else {
        (fraction, "")
    };
    if excess.bytes().any(|byte| byte != b'0') {
        return Err(DecimalError::PrecisionLoss);
    }
    let whole_value = whole
        .parse::<u64>()
        .map_err(|_| DecimalError::InvalidOrOutOfRange)?;
    let factor = 10_u64
        .checked_pow(scale as u32)
        .ok_or(DecimalError::InvalidOrOutOfRange)?;
    let mut padded_fraction = fraction_kept.to_string();
    while padded_fraction.len() < scale_len {
        padded_fraction.push('0');
    }
    let fraction_value = if padded_fraction.is_empty() {
        0
    } else {
        padded_fraction
            .parse::<u64>()
            .map_err(|_| DecimalError::InvalidOrOutOfRange)?
    };
    whole_value
        .checked_mul(factor)
        .and_then(|whole_minor| whole_minor.checked_add(fraction_value))
        .ok_or(DecimalError::InvalidOrOutOfRange)
}

fn parse_iso_date(value: &str) -> Option<u32> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[0..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let year = value[0..4].parse::<u32>().ok()?;
    let month = value[5..7].parse::<u32>().ok()?;
    let day = value[8..10].parse::<u32>().ok()?;
    if year == 0 || !(1..=12).contains(&month) {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let last_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=last_day).contains(&day) {
        return None;
    }
    Some(year * 10_000 + month * 100 + day)
}

fn unsupported_claim_context_fields(value: &Value) -> bool {
    const ALLOWED: &[&str] = &[
        "resourceType",
        "id",
        "meta",
        "status",
        "type",
        "use",
        "patient",
        "created",
        "provider",
        "priority",
        "insurance",
        "item",
    ];
    value
        .as_object()
        .is_none_or(|object| object.keys().any(|key| !ALLOWED.contains(&key.as_str())))
}

fn unsupported_meta_fields(value: &Value) -> bool {
    const ALLOWED_META: &[&str] = &["versionId", "lastUpdated", "profile"];
    value
        .get("meta")
        .and_then(Value::as_object)
        .is_none_or(|meta| meta.keys().any(|key| !ALLOWED_META.contains(&key.as_str())))
}

fn required_claim_fields_present(value: &Value) -> bool {
    let has_type_coding = value
        .get("type")
        .and_then(|claim_type| claim_type.get("coding"))
        .and_then(Value::as_array)
        .is_some_and(|codings| {
            codings.iter().any(|coding| {
                coding.get("system").and_then(Value::as_str)
                    .is_some_and(|system| !system.trim().is_empty())
                    && coding.get("code").and_then(Value::as_str)
                        .is_some_and(|code| !code.trim().is_empty())
            })
        });
    let has_priority_coding = value
        .get("priority")
        .and_then(|priority| priority.get("coding"))
        .and_then(Value::as_array)
        .is_some_and(|codings| {
            codings.iter().any(|coding| {
                coding
                    .get("system")
                    .and_then(Value::as_str)
                    .is_some_and(|system| !system.trim().is_empty())
                    && coding
                        .get("code")
                        .and_then(Value::as_str)
                        .is_some_and(|code| !code.trim().is_empty())
            })
        });
    let has_patient = value
        .get("patient")
        .and_then(|patient| patient.get("reference"))
        .and_then(Value::as_str)
        .is_some_and(|reference| !reference.trim().is_empty());
    let has_provider = value
        .get("provider")
        .and_then(|provider| provider.get("reference"))
        .and_then(Value::as_str)
        .is_some_and(|reference| !reference.trim().is_empty());
    let has_created = value
        .get("created")
        .and_then(Value::as_str)
        .is_some_and(|created| !created.trim().is_empty() && created.len() <= 64);
    let has_insurance = value
        .get("insurance")
        .and_then(Value::as_array)
        .is_some_and(|insurance| {
            let mut seen_sequences = BTreeSet::new();
            let mut focal_count = 0usize;
            let entries_are_valid = !insurance.is_empty()
                && insurance.iter().all(|entry| {
                    let sequence = entry.get("sequence").and_then(Value::as_u64);
                    let sequence_is_valid = sequence.is_some_and(|sequence| {
                        sequence > 0 && seen_sequences.insert(sequence)
                    });
                    let focal = entry.get("focal").and_then(Value::as_bool);
                    if focal == Some(true) {
                        focal_count += 1;
                    }
                    sequence_is_valid
                        && focal.is_some()
                        && entry
                            .get("coverage")
                            .and_then(|coverage| coverage.get("reference"))
                            .and_then(Value::as_str)
                            .is_some_and(|reference| !reference.trim().is_empty())
                });
            entries_are_valid && focal_count == 1
        });
    has_type_coding
        && has_priority_coding
        && has_patient
        && has_provider
        && has_created
        && has_insurance
}

fn source_versions(value: &Value) -> (Option<String>, Option<String>) {
    (
        value
            .get("meta")
            .and_then(|meta| meta.get("versionId"))
            .and_then(Value::as_str)
            .map(str::to_string),
        value
            .get("meta")
            .and_then(|meta| meta.get("lastUpdated"))
            .and_then(Value::as_str)
            .map(str::to_string),
    )
}

fn version_is_present(version: &Option<String>) -> bool {
    version
        .as_deref()
        .is_some_and(|version| !version.trim().is_empty() && version.len() <= 128)
}

fn profile_matches(value: &Value, expected_profile: &str) -> bool {
    value
        .get("meta")
        .and_then(|meta| meta.get("profile"))
        .and_then(Value::as_array)
        .is_some_and(|profiles| {
            profiles.len() == 1 && profiles[0].as_str() == Some(expected_profile)
        })
}

fn contains_extension(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, nested)| {
            key == "extension"
                || key == "modifierExtension"
                || contains_extension(nested)
        }),
        Value::Array(values) => values.iter().any(contains_extension),
        _ => false,
    }
}

fn validate_context(context: &MappingContextV1) -> Result<(), AdapterError> {
    for token in [
        context.source_resource_ref.as_str(),
        context.claim_ref.as_str(),
        context.beneficiary_ref.as_str(),
        context.provider_ref.as_str(),
    ] {
        if !is_identifier(token) {
            return Err(AdapterError::InvalidOpaqueReference);
        }
    }
    if context
        .linked_claim_ref
        .as_deref()
        .is_some_and(|token| !is_identifier(token))
    {
        return Err(AdapterError::InvalidOpaqueReference);
    }
    Ok(())
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn is_currency_code(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn manifest_digest(
    source_resource_ref: &str,
    source_version_id: Option<&str>,
    payload_digest: &str,
    policy_digest: &str,
) -> String {
    let canonical = format!(
        "fhir-claim-manifest-v1\0{}\0{}\0{}\0{}",
        source_resource_ref,
        source_version_id.unwrap_or("missing-version"),
        payload_digest,
        policy_digest
    );
    sha256_hex(canonical.as_bytes())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims_integrity_core::{AnalysisScope, AnalysisStatus, SignalKind};

    const FIXTURE: &str = include_str!("../fixtures/synthetic-claim-duplicate.json");

    fn context() -> MappingContextV1 {
        MappingContextV1 {
            source_resource_ref: "source-claim-demo-001".to_string(),
            claim_ref: "claim-demo-001".to_string(),
            beneficiary_ref: "beneficiary-demo-001".to_string(),
            provider_ref: "provider-demo-001".to_string(),
            linked_claim_ref: None,
        }
    }

    fn policy() -> MappingPolicyV1 {
        MappingPolicyV1 {
            code_set_version: "synthetic-services-v1".to_string(),
            currency_minor_unit_scales: BTreeMap::from([
                ("USD".to_string(), 2),
                ("ZAR".to_string(), 2),
                ("JPY".to_string(), 0),
            ]),
        }
    }

    fn map_fixture() -> ClaimAdapterOutputV1 {
        match map_fhir_r4_json(FIXTURE.as_bytes(), &context(), &policy())
            .expect("synthetic fixture should parse")
        {
            AdapterResultV1::Claim(output) => output,
            AdapterResultV1::ClaimResponse(_) => panic!("fixture is a Claim"),
        }
    }

    #[test]
    fn supported_synthetic_claim_maps_to_review_only_bundle() {
        let output = map_fixture();
        assert_eq!(output.envelope.state, MappingStateV1::Complete);
        assert_eq!(output.envelope.mapped_items.len(), 2);
        assert_eq!(output.bundle.lines.len(), 2);
        assert_eq!(output.bundle.lines[0].amount_minor_units, 12_500);
        assert_eq!(output.bundle.lines[0].currency_code, "USD");
        assert_eq!(output.bundle.lines[0].service_date_yyyymmdd, 20261010);
        assert_eq!(output.envelope.mapped_items[0].coding_code, "SVC-100");
        assert!(output.bundle.lines[0].service_code.starts_with("fhir-"));
        assert_eq!(output.evaluation.status, AnalysisStatus::Complete);
        assert_eq!(output.evaluation.signals.len(), 1);
        assert_eq!(
            output.evaluation.signals[0].kind,
            SignalKind::PotentialDuplicateServiceLines
        );
        assert!(output
            .evaluation
            .signals
            .iter()
            .all(|signal| signal.decision_authority
                == claims_integrity_core::DecisionAuthority::NonAuthoritativeReviewLead));
    }

    #[test]
    fn claim_response_is_reference_only_and_never_a_service_line() {
        let value = serde_json::json!({
            "resourceType": "ClaimResponse",
            "id": "synthetic-response-001",
            "meta": {
                "versionId": "7",
                "lastUpdated": "2026-10-12T11:00:00Z",
                "profile": [CLAIM_RESPONSE_PROFILE_CANONICAL]
            },
            "status": "active",
            "outcome": "complete",
            "request": {"reference": "Claim/synthetic-claim-001"},
            "item": [{
                "itemSequence": 1,
                "adjudication": [{
                    "category": {"text": "benefit"},
                    "amount": {"value": 125.0, "currency": "USD"}
                }]
            }]
        });
        let mut ctx = context();
        ctx.linked_claim_ref = Some("claim-demo-001".to_string());
        let raw = value.to_string();
        let result = map_fhir_r4_json(raw.as_bytes(), &ctx, &policy()).expect("valid JSON");
        let AdapterResultV1::ClaimResponse(response) = result else {
            panic!("ClaimResponse must remain distinct");
        };
        assert_eq!(
            response.linkage_state,
            ClaimResponseLinkageStateV1::LinkedViaCallerSuppliedOpaqueRef
        );
        assert!(response.request_reference_present);
        assert_eq!(response.linked_claim_ref.as_deref(), Some("claim-demo-001"));
        assert!(!response.service_lines_mapped);
        assert!(response.envelope.mapped_items.is_empty());
        assert_eq!(response.envelope.state, MappingStateV1::ResponseReferenceOnly);
    }

    #[test]
    fn service_period_is_not_coerced_to_single_date() {
        let mut value: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        value["item"][0].as_object_mut().expect("item object").remove("servicedDate");
        value["item"][0]["servicedPeriod"] = serde_json::json!({
            "start": "2026-10-10",
            "end": "2026-10-12"
        });
        let output = match map_fhir_r4_json(
            value.to_string().as_bytes(),
            &context(),
            &policy()
        ).expect("JSON should parse") {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert_eq!(output.envelope.state, MappingStateV1::Partial);
        assert_eq!(output.bundle.completeness, InputCompleteness::Partial);
        assert_eq!(output.bundle.lines.len(), 1);
        assert!(output.envelope.unmapped_items.iter().any(|item| {
            item.reason == MappingIssueV1::ServicePeriodNotSupported
                && item.item_sequence == Some(1)
        }));
        assert_eq!(output.evaluation.status, AnalysisStatus::Indeterminate);
        assert_eq!(output.evaluation.scope, AnalysisScope::ObservedRecordsOnly);
    }

    #[test]
    fn unknown_profile_fails_closed() {
        let mut value: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        value["meta"]["profile"] = serde_json::json!(["http://example.org/unsupported-profile"]);
        let output = match map_fhir_r4_json(
            value.to_string().as_bytes(),
            &context(),
            &policy()
        ).expect("valid JSON should produce a fail-closed result") {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert_eq!(output.envelope.state, MappingStateV1::Indeterminate);
        assert!(output.envelope.global_issues.contains(&MappingIssueV1::UnsupportedProfile));
        assert!(output.bundle.lines.is_empty());
        assert_eq!(output.evaluation.status, AnalysisStatus::Indeterminate);
    }

    #[test]
    fn unknown_extensions_fail_closed() {
        let mut value: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        value["item"][0]["extension"] = serde_json::json!([{
            "url": "urn:example:unknown",
            "valueString": "synthetic"
        }]);
        let output = match map_fhir_r4_json(
            value.to_string().as_bytes(),
            &context(),
            &policy()
        ).expect("valid JSON should produce a fail-closed result") {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert!(output
            .envelope
            .global_issues
            .contains(&MappingIssueV1::UnsupportedResourceExtension));
        assert_eq!(output.envelope.state, MappingStateV1::Indeterminate);
        assert!(output.bundle.lines.is_empty());
    }

    #[test]
    fn unsupported_modifier_is_not_silently_dropped() {
        let mut value: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        value["item"][0]["modifier"] = serde_json::json!([{
            "coding": [{"system": "urn:example:modifier", "code": "special"}]
        }]);
        let output = match map_fhir_r4_json(
            value.to_string().as_bytes(),
            &context(),
            &policy()
        ).expect("valid JSON should map with explicit loss report") {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert_eq!(output.envelope.state, MappingStateV1::Partial);
        assert!(output.envelope.unmapped_items.iter().any(|item| {
            item.reason == MappingIssueV1::UnsupportedItemFields
                && item.unsupported_field_names.contains(&"modifier".to_string())
        }));
        assert_eq!(output.evaluation.status, AnalysisStatus::Indeterminate);
    }

    #[test]
    fn currencies_require_an_explicit_minor_unit_scale() {
        let mut value: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        value["item"][0]["net"]["currency"] = serde_json::json!("EUR");
        let output = match map_fhir_r4_json(
            value.to_string().as_bytes(),
            &context(),
            &policy()
        ).expect("valid JSON should map with explicit loss report") {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert!(output.envelope.unmapped_items.iter().any(|item| {
            item.reason == MappingIssueV1::UnsupportedCurrency
                && item.item_sequence == Some(1)
        }));
        assert_eq!(output.envelope.state, MappingStateV1::Partial);
    }

    #[test]
    fn excessive_decimal_precision_is_not_rounded() {
        let raw = FIXTURE.replace("125.00", "125.001");
        let output = match map_fhir_r4_json(
            raw.as_bytes(),
            &context(),
            &policy()
        ).expect("fixture variant should parse") {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert!(output.envelope.unmapped_items.iter().any(|item| {
            item.reason == MappingIssueV1::AmountPrecisionWouldBeLost
        }));
        assert_eq!(output.envelope.state, MappingStateV1::Partial);
    }

    #[test]
    fn mapped_service_code_identity_includes_system_and_version() {
        let base = map_fixture();
        let mut changed: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        changed["item"][0]["productOrService"]["coding"][0]["system"] =
            serde_json::json!("http://example.org/fhir/CodeSystem/other");
        let changed = match map_fhir_r4_json(
            changed.to_string().as_bytes(),
            &context(),
            &policy()
        ).expect("valid JSON") {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert_ne!(
            base.bundle.lines[0].service_code,
            changed.bundle.lines[0].service_code
        );
    }

    #[test]
    fn exact_payload_digest_changes_when_captured_bytes_change() {
        let compact = serde_json::to_string(&serde_json::from_str::<Value>(FIXTURE).expect("JSON"))
            .expect("compact serialization");
        let first = match map_fhir_r4_json(
            FIXTURE.as_bytes(),
            &context(),
            &policy()
        ).expect("fixture") {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        let second = match map_fhir_r4_json(
            compact.as_bytes(),
            &context(),
            &policy()
        ).expect("compacted fixture") {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert_ne!(first.envelope.source_payload_sha256, second.envelope.source_payload_sha256);
        assert_ne!(first.envelope.input_manifest_sha256, second.envelope.input_manifest_sha256);
    }

    #[test]
    fn duplicate_item_sequences_fail_closed_for_the_whole_claim() {
        let mut value: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        value["item"][1]["sequence"] = serde_json::json!(1);
        let output = match map_fhir_r4_json(
            value.to_string().as_bytes(),
            &context(),
            &policy()
        ).expect("valid JSON") {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert!(output.envelope.global_issues.contains(&MappingIssueV1::DuplicateSequence));
        assert!(output.bundle.lines.is_empty());
        assert_eq!(output.envelope.state, MappingStateV1::Indeterminate);
    }

    #[test]
    fn preauthorization_is_not_mapped_as_rendered_service_lines() {
        let mut value: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        value["use"] = serde_json::json!("preauthorization");
        let output = match map_fhir_r4_json(
            value.to_string().as_bytes(),
            &context(),
            &policy(),
        )
        .expect("valid JSON should produce a fail-closed result")
        {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert!(output
            .envelope
            .global_issues
            .contains(&MappingIssueV1::UnsupportedClaimUse));
        assert!(output.bundle.lines.is_empty());
        assert_eq!(output.envelope.state, MappingStateV1::Indeterminate);
    }

    #[test]
    fn missing_required_fhir_claim_context_fails_closed() {
        let mut value: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        value.as_object_mut().expect("Claim object").remove("patient");
        let output = match map_fhir_r4_json(
            value.to_string().as_bytes(),
            &context(),
            &policy(),
        )
        .expect("valid JSON should produce a fail-closed result")
        {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert!(output
            .envelope
            .global_issues
            .contains(&MappingIssueV1::MissingRequiredClaimFields));
        assert!(output.bundle.lines.is_empty());
        assert_eq!(output.envelope.state, MappingStateV1::Indeterminate);
        assert_eq!(output.evaluation.status, AnalysisStatus::Indeterminate);
    }

    #[test]
    fn invalid_dates_and_missing_net_are_explicitly_unmapped() {
        let mut value: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
        value["item"][0]["servicedDate"] = serde_json::json!("2026-02-30");
        value["item"][1].as_object_mut().expect("item object").remove("net");
        let output = match map_fhir_r4_json(
            value.to_string().as_bytes(),
            &context(),
            &policy()
        ).expect("valid JSON") {
            AdapterResultV1::Claim(output) => output,
            _ => panic!("expected Claim"),
        };
        assert!(output.envelope.unmapped_items.iter().any(|item| {
            item.item_sequence == Some(1) && item.reason == MappingIssueV1::InvalidServiceDate
        }));
        assert!(output.envelope.unmapped_items.iter().any(|item| {
            item.item_sequence == Some(2) && item.reason == MappingIssueV1::MissingNetMoney
        }));
        assert_eq!(output.envelope.state, MappingStateV1::Indeterminate);
        assert!(output.bundle.lines.is_empty());
    }

    #[test]
    fn malformed_json_and_unsupported_resource_types_return_errors() {
        assert_eq!(
            map_fhir_r4_json(b"{oops", &context(), &policy()).unwrap_err(),
            AdapterError::InvalidJson
        );
        assert_eq!(
            map_fhir_r4_json(
                br#"{"resourceType":"Patient"}"#,
                &context(),
                &policy()
            )
            .unwrap_err(),
            AdapterError::UnsupportedResourceType
        );
    }
}
