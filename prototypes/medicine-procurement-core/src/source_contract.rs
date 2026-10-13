// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Typed, source-agnostic conformance boundary for price observations.
//!
//! This module does not download, parse, hash, or legally interpret source files.
//! An upstream adapter supplies typed fields and the evidence manifest; this
//! code checks internal consistency before an observation can enter comparison.
//! A listed price is never evidence of stock or a supplier's willingness to sell.

use super::{
    is_currency_code, is_identifier, is_sha256_hex, is_valid_date, PriceKindV1,
    SourceReuseState, SourceUseScopeV1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRightsReviewStateV1 {
    Pending,
    Granted,
    Denied,
    Expired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceOperationV1 {
    ReadPage,
    BulkExtract,
    StoreRawBytes,
    StoreDerivedRecords,
    TransformRecords,
    PublishDerivedAggregates,
    RedistributeRawRecords,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceListingStatusV1 {
    Listed,
    Discontinued,
    Withdrawn,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriceTemporalStateV1 {
    CurrentAsOfDate,
    FutureEffective,
    Expired,
    Undated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceManifestV1 {
    pub source_id: String,
    pub publisher_id: String,
    pub source_revision_id: String,
    pub source_published_yyyymmdd: u32,
    pub source_snapshot_sha256: String,
    pub terms_document_sha256: String,
    pub permission_evidence_sha256: String,
    pub rights_reviewer_id: String,
    pub rights_review_state: SourceRightsReviewStateV1,
    pub rights_reviewed_yyyymmdd: Option<u32>,
    pub permission_valid_through_yyyymmdd: Option<u32>,
    pub permitted_use_scopes: Vec<SourceUseScopeV1>,
    pub permitted_operations: Vec<SourceOperationV1>,
    pub source_reuse_state: SourceReuseState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceIngestionPolicyV1 {
    pub as_of_yyyymmdd: u32,
    pub required_use_scope: SourceUseScopeV1,
    pub required_operations: Vec<SourceOperationV1>,
    pub required_medicine_presentation_id: String,
    pub required_jurisdiction: String,
    pub required_price_kind: PriceKindV1,
    pub required_price_class_id: String,
    pub required_currency_code: String,
    pub required_normalized_unit_basis: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePriceRecordV1 {
    pub source_record_id: String,
    pub source_record_sha256: String,
    pub medicine_presentation_id: String,
    pub jurisdiction: String,
    pub price_kind: PriceKindV1,
    /// Exact source-defined basis, e.g. za-ndoh.sep; never inferred from amount.
    pub price_class_id: String,
    pub amount_minor: u64,
    pub currency_code: String,
    pub source_unit_basis: String,
    /// Supplied only when an independently reviewed unit conversion was applied.
    pub normalized_amount_minor: Option<u64>,
    pub normalized_unit_basis: Option<String>,
    pub normalization_method_id: Option<String>,
    pub normalization_evidence_sha256: Option<String>,
    pub observed_yyyymmdd: u32,
    pub effective_from_yyyymmdd: Option<u32>,
    pub effective_to_yyyymmdd: Option<u32>,
    pub listing_status: SourceListingStatusV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceContractRejectionV1 {
    InvalidPolicy,
    InvalidManifestMetadata,
    RightsNotGranted,
    RightsReviewMissing,
    RightsPermissionExpired,
    SourceReuseNotPermitted,
    UseScopeNotPermitted,
    OperationNotPermitted,
    InvalidSourceDigest,
    InvalidRecordIdentity,
    InvalidPriceClass,
    PriceKindMismatch,
    PriceClassMismatch,
    CurrencyMismatch,
    JurisdictionMismatch,
    MedicinePresentationMismatch,
    InvalidAmountOrDate,
    ObservationDatedInFuture,
    InvalidEffectiveDateRange,
    MissingNormalizationEvidence,
    InvalidNormalizationEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedPriceObservationV1 {
    pub schema_version: u16,
    pub source_id: String,
    pub publisher_id: String,
    pub source_revision_id: String,
    pub source_snapshot_sha256: String,
    pub source_record_id: String,
    pub source_record_sha256: String,
    pub rights_permission_evidence_sha256: String,
    pub medicine_presentation_id: String,
    pub jurisdiction: String,
    pub price_kind: PriceKindV1,
    pub price_class_id: String,
    pub original_amount_minor: u64,
    pub original_currency_code: String,
    pub original_unit_basis: String,
    pub normalized_amount_minor: u64,
    pub normalized_currency_code: String,
    pub normalized_unit_basis: String,
    pub normalization_method_id: String,
    pub normalization_evidence_sha256: Option<String>,
    pub observed_yyyymmdd: u32,
    pub effective_from_yyyymmdd: Option<u32>,
    pub effective_to_yyyymmdd: Option<u32>,
    pub temporal_state: PriceTemporalStateV1,
    pub listing_status: SourceListingStatusV1,
    /// Only a candidate for current price comparison; never proof of stock.
    pub eligible_for_current_price_comparison: bool,
}

pub fn normalize_price_observation_v1(
    manifest: &SourceManifestV1,
    policy: &SourceIngestionPolicyV1,
    record: &SourcePriceRecordV1,
) -> Result<NormalizedPriceObservationV1, SourceContractRejectionV1> {
    validate_ingestion_policy(policy)?;
    validate_source_manifest(manifest, policy)?;

    if !is_identifier(&record.source_record_id)
        || !is_identifier(&record.medicine_presentation_id)
        || !is_identifier(&record.jurisdiction)
        || !is_identifier(&record.price_class_id)
        || !is_identifier(&record.source_unit_basis)
    {
        return Err(SourceContractRejectionV1::InvalidRecordIdentity);
    }
    if !is_sha256_hex(&record.source_record_sha256) {
        return Err(SourceContractRejectionV1::InvalidSourceDigest);
    }
    if record.price_kind == PriceKindV1::Unknown
        || policy.required_price_kind == PriceKindV1::Unknown
    {
        return Err(SourceContractRejectionV1::InvalidPriceClass);
    }
    if record.price_kind != policy.required_price_kind {
        return Err(SourceContractRejectionV1::PriceKindMismatch);
    }
    if record.price_class_id != policy.required_price_class_id {
        return Err(SourceContractRejectionV1::PriceClassMismatch);
    }
    if record.currency_code != policy.required_currency_code
        || !is_currency_code(&record.currency_code)
    {
        return Err(SourceContractRejectionV1::CurrencyMismatch);
    }
    if record.jurisdiction != policy.required_jurisdiction {
        return Err(SourceContractRejectionV1::JurisdictionMismatch);
    }
    if record.medicine_presentation_id != policy.required_medicine_presentation_id {
        return Err(SourceContractRejectionV1::MedicinePresentationMismatch);
    }
    if !is_valid_date(record.observed_yyyymmdd) {
        return Err(SourceContractRejectionV1::InvalidAmountOrDate);
    }
    if record.observed_yyyymmdd > policy.as_of_yyyymmdd {
        return Err(SourceContractRejectionV1::ObservationDatedInFuture);
    }

    let (effective_amount, effective_unit, method_id, conversion_digest) =
        normalize_unit_basis(policy, record)?;
    let temporal_state = temporal_state(policy.as_of_yyyymmdd, record)?;

    let eligible_for_current_price_comparison = record.listing_status
        == SourceListingStatusV1::Listed
        && temporal_state == PriceTemporalStateV1::CurrentAsOfDate;

    Ok(NormalizedPriceObservationV1 {
        schema_version: 1,
        source_id: manifest.source_id.clone(),
        publisher_id: manifest.publisher_id.clone(),
        source_revision_id: manifest.source_revision_id.clone(),
        source_snapshot_sha256: manifest.source_snapshot_sha256.clone(),
        source_record_id: record.source_record_id.clone(),
        source_record_sha256: record.source_record_sha256.clone(),
        rights_permission_evidence_sha256: manifest.permission_evidence_sha256.clone(),
        medicine_presentation_id: record.medicine_presentation_id.clone(),
        jurisdiction: record.jurisdiction.clone(),
        price_kind: record.price_kind,
        price_class_id: record.price_class_id.clone(),
        original_amount_minor: record.amount_minor,
        original_currency_code: record.currency_code.clone(),
        original_unit_basis: record.source_unit_basis.clone(),
        normalized_amount_minor: effective_amount,
        normalized_currency_code: record.currency_code.clone(),
        normalized_unit_basis: effective_unit,
        normalization_method_id: method_id,
        normalization_evidence_sha256: conversion_digest,
        observed_yyyymmdd: record.observed_yyyymmdd,
        effective_from_yyyymmdd: record.effective_from_yyyymmdd,
        effective_to_yyyymmdd: record.effective_to_yyyymmdd,
        temporal_state,
        listing_status: record.listing_status,
        eligible_for_current_price_comparison,
    })
}

fn validate_ingestion_policy(
    policy: &SourceIngestionPolicyV1,
) -> Result<(), SourceContractRejectionV1> {
    if !is_valid_date(policy.as_of_yyyymmdd)
        || !is_identifier(&policy.required_medicine_presentation_id)
        || !is_identifier(&policy.required_jurisdiction)
        || !is_identifier(&policy.required_price_class_id)
        || !is_currency_code(&policy.required_currency_code)
        || !is_identifier(&policy.required_normalized_unit_basis)
        || policy.required_price_kind == PriceKindV1::Unknown
    {
        return Err(SourceContractRejectionV1::InvalidPolicy);
    }
    Ok(())
}

fn validate_source_manifest(
    manifest: &SourceManifestV1,
    policy: &SourceIngestionPolicyV1,
) -> Result<(), SourceContractRejectionV1> {
    if !is_identifier(&manifest.source_id)
        || !is_identifier(&manifest.publisher_id)
        || !is_identifier(&manifest.source_revision_id)
        || !is_valid_date(manifest.source_published_yyyymmdd)
        || manifest.source_published_yyyymmdd > policy.as_of_yyyymmdd
        || !is_identifier(&manifest.rights_reviewer_id)
    {
        return Err(SourceContractRejectionV1::InvalidManifestMetadata);
    }
    if ![
        manifest.source_snapshot_sha256.as_str(),
        manifest.terms_document_sha256.as_str(),
        manifest.permission_evidence_sha256.as_str(),
    ]
    .into_iter()
    .all(is_sha256_hex)
    {
        return Err(SourceContractRejectionV1::InvalidSourceDigest);
    }
    if manifest.rights_review_state != SourceRightsReviewStateV1::Granted {
        return Err(SourceContractRejectionV1::RightsNotGranted);
    }
    let reviewed_on = manifest
        .rights_reviewed_yyyymmdd
        .filter(|date| is_valid_date(*date) && *date <= policy.as_of_yyyymmdd)
        .ok_or(SourceContractRejectionV1::RightsReviewMissing)?;
    let _ = reviewed_on;
    if manifest
        .permission_valid_through_yyyymmdd
        .is_some_and(|date| !is_valid_date(date) || date < policy.as_of_yyyymmdd)
    {
        return Err(SourceContractRejectionV1::RightsPermissionExpired);
    }
    if manifest.source_reuse_state != SourceReuseState::Permitted {
        return Err(SourceContractRejectionV1::SourceReuseNotPermitted);
    }
    if !manifest.permitted_use_scopes.contains(&policy.required_use_scope) {
        return Err(SourceContractRejectionV1::UseScopeNotPermitted);
    }
    if policy
        .required_operations
        .iter()
        .any(|operation| !manifest.permitted_operations.contains(operation))
    {
        return Err(SourceContractRejectionV1::OperationNotPermitted);
    }
    Ok(())
}

fn normalize_unit_basis(
    policy: &SourceIngestionPolicyV1,
    record: &SourcePriceRecordV1,
) -> Result<(u64, String, String, Option<String>), SourceContractRejectionV1> {
    if record.source_unit_basis == policy.required_normalized_unit_basis {
        if record.normalized_amount_minor.is_some()
            || record.normalized_unit_basis.is_some()
            || record.normalization_method_id.is_some()
            || record.normalization_evidence_sha256.is_some()
        {
            return Err(SourceContractRejectionV1::InvalidNormalizationEvidence);
        }
        return Ok((
            record.amount_minor,
            record.source_unit_basis.clone(),
            "identity-no-conversion-v1".to_string(),
            None,
        ));
    }

    let amount = record
        .normalized_amount_minor
        .ok_or(SourceContractRejectionV1::MissingNormalizationEvidence)?;
    let unit = record
        .normalized_unit_basis
        .as_ref()
        .ok_or(SourceContractRejectionV1::MissingNormalizationEvidence)?;
    let method = record
        .normalization_method_id
        .as_ref()
        .ok_or(SourceContractRejectionV1::MissingNormalizationEvidence)?;
    let digest = record
        .normalization_evidence_sha256
        .as_ref()
        .ok_or(SourceContractRejectionV1::MissingNormalizationEvidence)?;

    if unit != &policy.required_normalized_unit_basis
        || !is_identifier(method)
        || !is_sha256_hex(digest)
    {
        return Err(SourceContractRejectionV1::InvalidNormalizationEvidence);
    }
    Ok((amount, unit.clone(), method.clone(), Some(digest.clone())))
}

fn temporal_state(
    as_of: u32,
    record: &SourcePriceRecordV1,
) -> Result<PriceTemporalStateV1, SourceContractRejectionV1> {
    for date in [
        record.effective_from_yyyymmdd,
        record.effective_to_yyyymmdd,
    ]
    .into_iter()
    .flatten()
    {
        if !is_valid_date(date) {
            return Err(SourceContractRejectionV1::InvalidEffectiveDateRange);
        }
    }
    if let (Some(start), Some(end)) =
        (record.effective_from_yyyymmdd, record.effective_to_yyyymmdd)
    {
        if end < start {
            return Err(SourceContractRejectionV1::InvalidEffectiveDateRange);
        }
    }
    if record
        .effective_from_yyyymmdd
        .is_some_and(|start| start > as_of)
    {
        return Ok(PriceTemporalStateV1::FutureEffective);
    }
    if record
        .effective_to_yyyymmdd
        .is_some_and(|end| end < as_of)
    {
        return Ok(PriceTemporalStateV1::Expired);
    }
    if record.effective_from_yyyymmdd.is_none() {
        return Ok(PriceTemporalStateV1::Undated);
    }
    Ok(PriceTemporalStateV1::CurrentAsOfDate)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn policy() -> SourceIngestionPolicyV1 {
        SourceIngestionPolicyV1 {
            as_of_yyyymmdd: 20261014,
            required_use_scope: SourceUseScopeV1::CommercialAnalysis,
            required_operations: vec![
                SourceOperationV1::BulkExtract,
                SourceOperationV1::TransformRecords,
                SourceOperationV1::StoreDerivedRecords,
            ],
            required_medicine_presentation_id: "med-a-500mg-tablet-10".to_string(),
            required_jurisdiction: "ZA".to_string(),
            required_price_kind: PriceKindV1::PublishedReferencePrice,
            required_price_class_id: "za-ndoh.sep".to_string(),
            required_currency_code: "ZAR".to_string(),
            required_normalized_unit_basis: "tablet".to_string(),
        }
    }

    fn manifest() -> SourceManifestV1 {
        SourceManifestV1 {
            source_id: "za-ndoh-medicine-prices".to_string(),
            publisher_id: "za-ndoh".to_string(),
            source_revision_id: "snapshot-20261001".to_string(),
            source_published_yyyymmdd: 20261001,
            source_snapshot_sha256: DIGEST_A.to_string(),
            terms_document_sha256: DIGEST_B.to_string(),
            permission_evidence_sha256: DIGEST_C.to_string(),
            rights_reviewer_id: "reviewer-legal-01".to_string(),
            rights_review_state: SourceRightsReviewStateV1::Granted,
            rights_reviewed_yyyymmdd: Some(20261010),
            permission_valid_through_yyyymmdd: Some(20261231),
            permitted_use_scopes: vec![SourceUseScopeV1::CommercialAnalysis],
            permitted_operations: vec![
                SourceOperationV1::BulkExtract,
                SourceOperationV1::TransformRecords,
                SourceOperationV1::StoreDerivedRecords,
            ],
            source_reuse_state: SourceReuseState::Permitted,
        }
    }

    fn record() -> SourcePriceRecordV1 {
        SourcePriceRecordV1 {
            source_record_id: "record-001".to_string(),
            source_record_sha256: DIGEST_A.to_string(),
            medicine_presentation_id: "med-a-500mg-tablet-10".to_string(),
            jurisdiction: "ZA".to_string(),
            price_kind: PriceKindV1::PublishedReferencePrice,
            price_class_id: "za-ndoh.sep".to_string(),
            amount_minor: 1250,
            currency_code: "ZAR".to_string(),
            source_unit_basis: "tablet".to_string(),
            normalized_amount_minor: None,
            normalized_unit_basis: None,
            normalization_method_id: None,
            normalization_evidence_sha256: None,
            observed_yyyymmdd: 20261001,
            effective_from_yyyymmdd: Some(20261001),
            effective_to_yyyymmdd: Some(20261231),
            listing_status: SourceListingStatusV1::Listed,
        }
    }

    #[test]
    fn preserves_price_class_and_provenance_in_deterministic_normalization() {
        let first = normalize_price_observation_v1(&manifest(), &policy(), &record())
            .expect("valid synthetic record");
        let second = normalize_price_observation_v1(&manifest(), &policy(), &record())
            .expect("same source inputs");
        assert_eq!(first, second);
        assert_eq!(first.price_class_id, "za-ndoh.sep");
        assert_eq!(first.original_amount_minor, 1250);
        assert_eq!(first.normalized_amount_minor, 1250);
        assert_eq!(first.normalization_method_id, "identity-no-conversion-v1");
        assert!(first.eligible_for_current_price_comparison);
    }

    #[test]
    fn pending_or_denied_rights_fail_closed() {
        let mut source = manifest();
        source.rights_review_state = SourceRightsReviewStateV1::Pending;
        assert_eq!(
            normalize_price_observation_v1(&source, &policy(), &record()).unwrap_err(),
            SourceContractRejectionV1::RightsNotGranted
        );
        source.rights_review_state = SourceRightsReviewStateV1::Denied;
        assert_eq!(
            normalize_price_observation_v1(&source, &policy(), &record()).unwrap_err(),
            SourceContractRejectionV1::RightsNotGranted
        );
    }

    #[test]
    fn noncommercial_permission_does_not_allow_commercial_analysis() {
        let mut source = manifest();
        source.permitted_use_scopes = vec![SourceUseScopeV1::NonCommercialReference];
        assert_eq!(
            normalize_price_observation_v1(&source, &policy(), &record()).unwrap_err(),
            SourceContractRejectionV1::UseScopeNotPermitted
        );
    }

    #[test]
    fn each_required_operation_must_be_permitted() {
        let mut source = manifest();
        source
            .permitted_operations
            .retain(|operation| *operation != SourceOperationV1::TransformRecords);
        assert_eq!(
            normalize_price_observation_v1(&source, &policy(), &record()).unwrap_err(),
            SourceContractRejectionV1::OperationNotPermitted
        );
    }

    #[test]
    fn expired_source_permission_is_rejected() {
        let mut source = manifest();
        source.permission_valid_through_yyyymmdd = Some(20261013);
        assert_eq!(
            normalize_price_observation_v1(&source, &policy(), &record()).unwrap_err(),
            SourceContractRejectionV1::RightsPermissionExpired
        );
    }

    #[test]
    fn different_price_class_is_rejected_without_remapping() {
        let mut price = record();
        price.price_class_id = "za-ndoh.ex-manufacturer".to_string();
        assert_eq!(
            normalize_price_observation_v1(&manifest(), &policy(), &price).unwrap_err(),
            SourceContractRejectionV1::PriceClassMismatch
        );
    }

    #[test]
    fn unit_conversion_requires_method_target_and_evidence_digest() {
        let mut price = record();
        price.source_unit_basis = "pack-of-10".to_string();
        assert_eq!(
            normalize_price_observation_v1(&manifest(), &policy(), &price).unwrap_err(),
            SourceContractRejectionV1::MissingNormalizationEvidence
        );

        price.normalized_amount_minor = Some(125);
        price.normalized_unit_basis = Some("tablet".to_string());
        price.normalization_method_id = Some("pack-divide-v1".to_string());
        price.normalization_evidence_sha256 = Some(DIGEST_C.to_string());
        let normalized = normalize_price_observation_v1(&manifest(), &policy(), &price)
            .expect("explicit synthetic conversion receipt");
        assert_eq!(normalized.original_amount_minor, 1250);
        assert_eq!(normalized.normalized_amount_minor, 125);
        assert_eq!(normalized.original_unit_basis, "pack-of-10");
        assert_eq!(normalized.normalized_unit_basis, "tablet");
    }

    #[test]
    fn discontinued_observation_is_preserved_but_not_current_comparison_eligible() {
        let mut price = record();
        price.listing_status = SourceListingStatusV1::Discontinued;
        let normalized = normalize_price_observation_v1(&manifest(), &policy(), &price)
            .expect("discontinued data can be preserved as reference history");
        assert_eq!(normalized.listing_status, SourceListingStatusV1::Discontinued);
        assert!(!normalized.eligible_for_current_price_comparison);
    }

    #[test]
    fn expired_observation_is_preserved_as_historical_not_current() {
        let mut price = record();
        price.effective_to_yyyymmdd = Some(20261012);
        let normalized = normalize_price_observation_v1(&manifest(), &policy(), &price)
            .expect("historical observation remains dated");
        assert_eq!(normalized.temporal_state, PriceTemporalStateV1::Expired);
        assert!(!normalized.eligible_for_current_price_comparison);
    }

    #[test]
    fn future_or_invalid_dates_fail_closed() {
        let mut price = record();
        price.observed_yyyymmdd = 20261015;
        assert_eq!(
            normalize_price_observation_v1(&manifest(), &policy(), &price).unwrap_err(),
            SourceContractRejectionV1::ObservationDatedInFuture
        );
        price.observed_yyyymmdd = 20261001;
        price.effective_from_yyyymmdd = Some(20261231);
        price.effective_to_yyyymmdd = Some(20261001);
        assert_eq!(
            normalize_price_observation_v1(&manifest(), &policy(), &price).unwrap_err(),
            SourceContractRejectionV1::InvalidEffectiveDateRange
        );
    }

    #[test]
    fn unknown_listing_status_never_becomes_current_comparison_candidate() {
        let mut price = record();
        price.listing_status = SourceListingStatusV1::Unknown;
        let normalized = normalize_price_observation_v1(&manifest(), &policy(), &price)
            .expect("preserve uncertain listing state");
        assert_eq!(normalized.listing_status, SourceListingStatusV1::Unknown);
        assert!(!normalized.eligible_for_current_price_comparison);
    }
}
