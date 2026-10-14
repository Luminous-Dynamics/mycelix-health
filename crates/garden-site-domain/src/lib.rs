// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Privacy-first site profiles and reviewed non-clinical growing catalogues.
//!
//! This crate does not identify plants, certify soil safety, establish land
//! rights, decide whether a permit is required, or provide medical advice.
//! It deliberately has no fields for a person's name, exact street address,
//! precise coordinates, patient data, or traditional/sacred knowledge.
//!
//! Site planning can remain available while information is incomplete. The
//! catalogue-selection function is stricter: it returns no species-specific
//! entries until profile, jurisdiction, site review, catalogue validity, and
//! per-entry review gates all pass.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const GARDEN_SITE_SCHEMA_VERSION: u16 = 1;

/// Small site descriptors sufficient to start non-clinical garden planning.
/// No exact household location is represented by this type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GardenSiteProfileV1 {
    /// Random/local stable identifier; must not encode a person's identity.
    pub profile_id: String,
    pub schema_version: u16,
    /// ISO-style jurisdiction token, for example ZA. Not a street address.
    pub jurisdiction_code: String,
    /// Coarse, reviewed catalogue region; never a GPS coordinate or address.
    pub catalogue_region_id: Option<String>,
    pub growing_space: Option<GrowingSpace>,
    pub sun_exposure: Option<SunExposure>,
    pub water_access: Option<WaterAccess>,
    pub land_permission: LandPermission,
    pub hazard_screening: HazardScreening,
    /// BCP-47-like language tag, e.g. en-ZA or zu-ZA.
    pub preferred_language: Option<String>,
    pub interface_preferences: Vec<InterfacePreference>,
    /// Local-only is the default; network sync is never implicit.
    pub sync_state: SyncState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrowingSpace {
    Containers,
    HouseholdGround,
    SchoolGarden,
    CommunityGarden,
    SharedUrbanPlot,
    Rooftop,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SunExposure {
    MostlySunny,
    PartSunPartShade,
    MostlyShaded,
    Variable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaterAccess {
    Reliable,
    Seasonal,
    Restricted,
    RainwaterOnly,
    NoReliableAccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfacePreference {
    PlainLanguage,
    LargeText,
    ScreenReaderFriendly,
    LowBandwidth,
    PrintFriendly,
}

/// Evidence that a site may be used; this is separate from species or product
/// permissions and does not make a legal determination for the user.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LandPermission {
    Unknown,
    Pending,
    Denied,
    Granted { evidence_ref: String },
    NotRequired {
        review_ref: String,
        rationale: String,
    },
}

/// Record a review state without asserting that this crate can assess a site's
/// safety. Species-specific guidance requires a reviewer to explicitly scope
/// the site as suitable for that limited, non-clinical purpose.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum HazardScreening {
    NotAssessed,
    ConcernReported,
    ProfessionalReviewRequired,
    ReviewedForPlanning {
        review_ref: String,
        disposition: HazardGuidanceDisposition,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardGuidanceDisposition {
    GeneralPlanningOnly,
    SpeciesSpecificCultivationGuidanceAllowed,
}

/// Sync is off unless an explicit, valid opt-in is present. Withdrawal records
/// intent; a sync adapter must separately verify deletion/export of remote data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SyncState {
    LocalOnly,
    OptedIn {
        consent_ref: String,
        consented_at_utc: String,
    },
    Withdrawn {
        revocation_ref: String,
        withdrawn_at_utc: String,
    },
}

impl Default for SyncState {
    fn default() -> Self {
        Self::LocalOnly
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestStatus {
    Draft,
    Approved,
    Suspended,
}

/// Approval metadata for a specific, versioned catalogue manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestApproval {
    pub reviewer_id: String,
    pub evidence_ref: String,
    pub reviewed_at_utc: String,
}

/// Catalogue of publicly shareable plant-growing records for one region.
/// A valid, approved manifest may deliberately have zero entries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalGrowingCatalogueManifestV1 {
    pub schema_version: u16,
    pub manifest_id: String,
    pub catalogue_version: String,
    pub jurisdiction_code: String,
    pub catalogue_region_id: String,
    pub status: ManifestStatus,
    pub approval: Option<ManifestApproval>,
    pub expires_at_utc: String,
    pub entries: Vec<LocalGrowingCatalogueEntry>,
}

/// An entry is displayable only if all independent review dimensions pass.
/// No traditional-use narrative or private knowledge is stored in the entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalGrowingCatalogueEntry {
    pub entry_id: String,
    /// Public display name only, disclosed under the rights review below.
    pub display_label: String,
    /// Reference to an authoritative taxonomy record, not an AI guess.
    pub taxon_ref: String,
    pub taxon_review: ReviewState,
    pub conservation_review: ConservationReview,
    pub knowledge_disclosure: KnowledgeDisclosure,
    pub cultivation_guide_review: ReviewState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ReviewState {
    Unreviewed,
    Approved { evidence_ref: String },
    Rejected { evidence_ref: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ConservationReview {
    Unknown,
    ApprovedForCultivation { evidence_ref: String },
    RestrictionsApply { evidence_ref: String },
    Prohibited { evidence_ref: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum KnowledgeDisclosure {
    Unknown,
    PubliclyShareable { evidence_ref: String },
    Restricted,
    Withheld,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileBlockerCode {
    EmptyProfileId,
    InvalidProfileIdFormat,
    UnsupportedProfileSchema,
    EmptyJurisdiction,
    InvalidJurisdictionCode,
    MissingCatalogueRegion,
    InvalidCatalogueRegionId,
    MissingGrowingSpace,
    MissingSunExposure,
    MissingWaterAccess,
    LandPermissionUnresolved,
    LandPermissionDenied,
    InvalidLandPermissionEvidence,
    HazardNotAssessed,
    HazardConcernReported,
    HazardReviewRequired,
    InvalidHazardReview,
    SpeciesGuidanceNotAllowed,
    InvalidPreferredLanguage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogueBlockerCode {
    InvalidEvaluationTimestamp,
    UnsupportedManifestSchema,
    EmptyManifestId,
    EmptyCatalogueVersion,
    EmptyManifestJurisdiction,
    InvalidManifestJurisdictionCode,
    EmptyManifestRegion,
    InvalidManifestRegionFormat,
    JurisdictionMismatch,
    RegionMismatch,
    ManifestNotApproved,
    MissingManifestApproval,
    InvalidManifestApproval,
    ManifestReviewedInFuture,
    InvalidManifestWindow,
    ManifestExpired,
    DuplicateEntryId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryExclusionCode {
    EmptyEntryId,
    EmptyDisplayLabel,
    EmptyTaxonReference,
    TaxonUnreviewed,
    TaxonRejected,
    InvalidTaxonEvidence,
    ConservationUnknown,
    ConservationRestricted,
    ConservationProhibited,
    InvalidConservationEvidence,
    KnowledgeRightsUnknown,
    KnowledgeRestricted,
    KnowledgeWithheld,
    InvalidKnowledgeEvidence,
    GuideUnreviewed,
    GuideRejected,
    InvalidGuideEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileBlocker {
    pub code: ProfileBlockerCode,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogueBlocker {
    pub code: CatalogueBlockerCode,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExcludedEntry {
    pub entry_id: String,
    pub reasons: Vec<EntryExclusionCode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogueDecision {
    /// True only when the profile and manifest are valid for this exact region.
    pub catalogue_available: bool,
    /// IDs only; callers should resolve public names from the same reviewed
    /// manifest and must not fabricate or infer missing entries.
    pub visible_entry_ids: Vec<String>,
    /// Intended for review/admin views, not as a public disclosure of private
    /// knowledge. Unknown or restricted entries remain excluded.
    pub excluded_entries: Vec<ExcludedEntry>,
    pub profile_blockers: Vec<ProfileBlocker>,
    pub catalogue_blockers: Vec<CatalogueBlocker>,
    /// Empty is a valid state and must not be auto-filled by a model.
    pub no_eligible_entries: bool,
}

impl GardenSiteProfileV1 {
    /// Check only the structure of consent metadata for a future sync adapter.
    /// This does not authenticate consent or authorize network access.
    pub fn has_structurally_valid_sync_opt_in(&self) -> bool {
        match &self.sync_state {
            SyncState::OptedIn { consent_ref, consented_at_utc } => {
                !consent_ref.trim().is_empty() && is_valid_utc_timestamp(consented_at_utc)
            }
            SyncState::LocalOnly | SyncState::Withdrawn { .. } => false,
        }
    }

    /// Validate the profile only for a non-clinical, species-specific catalogue.
    /// A failure here does not prevent generic education or manual planning.
    pub fn catalogue_blockers(&self) -> Vec<ProfileBlocker> {
        let mut blockers = Vec::new();

        if self.profile_id.trim().is_empty() {
            push_profile(
                &mut blockers,
                ProfileBlockerCode::EmptyProfileId,
                "profile_id must be non-empty and non-identifying",
            );
        } else if !is_opaque_identifier(&self.profile_id) {
            push_profile(
                &mut blockers,
                ProfileBlockerCode::InvalidProfileIdFormat,
                "profile_id must be a locally generated opaque UUID-shaped value",
            );
        }
        if self.schema_version != GARDEN_SITE_SCHEMA_VERSION {
            push_profile(
                &mut blockers,
                ProfileBlockerCode::UnsupportedProfileSchema,
                "profile schema version is unsupported",
            );
        }
        if self.jurisdiction_code.trim().is_empty() {
            push_profile(
                &mut blockers,
                ProfileBlockerCode::EmptyJurisdiction,
                "jurisdiction_code must be non-empty",
            );
        } else if !is_country_code(&self.jurisdiction_code) {
            push_profile(
                &mut blockers,
                ProfileBlockerCode::InvalidJurisdictionCode,
                "jurisdiction_code must be a two-letter uppercase country code",
            );
        }
        if self
            .catalogue_region_id
            .as_deref()
            .map(str::trim)
            .map(str::is_empty)
            .unwrap_or(true)
        {
            push_profile(
                &mut blockers,
                ProfileBlockerCode::MissingCatalogueRegion,
                "a coarse catalogue region is required; do not use an exact address",
            );
        }
        if let Some(region_id) = self.catalogue_region_id.as_deref() {
            if !is_region_id(region_id) {
                push_profile(
                    &mut blockers,
                    ProfileBlockerCode::InvalidCatalogueRegionId,
                    "catalogue_region_id must be a reviewed region token, not free-form locality text",
                );
            }
        }
        if self.growing_space.is_none() {
            push_profile(
                &mut blockers,
                ProfileBlockerCode::MissingGrowingSpace,
                "growing space is unknown",
            );
        }
        if self.sun_exposure.is_none() {
            push_profile(
                &mut blockers,
                ProfileBlockerCode::MissingSunExposure,
                "sun exposure is unknown",
            );
        }
        if self.water_access.is_none() {
            push_profile(
                &mut blockers,
                ProfileBlockerCode::MissingWaterAccess,
                "water access is unknown",
            );
        }

        match &self.land_permission {
            LandPermission::Unknown | LandPermission::Pending => push_profile(
                &mut blockers,
                ProfileBlockerCode::LandPermissionUnresolved,
                "land-use permission is unresolved",
            ),
            LandPermission::Denied => push_profile(
                &mut blockers,
                ProfileBlockerCode::LandPermissionDenied,
                "land-use permission was denied",
            ),
            LandPermission::Granted { evidence_ref } if evidence_ref.trim().is_empty() => {
                push_profile(
                    &mut blockers,
                    ProfileBlockerCode::InvalidLandPermissionEvidence,
                    "granted permission requires an evidence reference",
                );
            }
            LandPermission::NotRequired { review_ref, rationale }
                if review_ref.trim().is_empty() || rationale.trim().is_empty() =>
            {
                push_profile(
                    &mut blockers,
                    ProfileBlockerCode::InvalidLandPermissionEvidence,
                    "not-required status needs a review reference and rationale",
                );
            }
            _ => {}
        }

        match &self.hazard_screening {
            HazardScreening::NotAssessed => push_profile(
                &mut blockers,
                ProfileBlockerCode::HazardNotAssessed,
                "site hazards have not been reviewed",
            ),
            HazardScreening::ConcernReported => push_profile(
                &mut blockers,
                ProfileBlockerCode::HazardConcernReported,
                "a site hazard concern is recorded",
            ),
            HazardScreening::ProfessionalReviewRequired => push_profile(
                &mut blockers,
                ProfileBlockerCode::HazardReviewRequired,
                "professional site review is required",
            ),
            HazardScreening::ReviewedForPlanning { review_ref, disposition } => {
                if review_ref.trim().is_empty() {
                    push_profile(
                        &mut blockers,
                        ProfileBlockerCode::InvalidHazardReview,
                        "site review requires an evidence reference",
                    );
                }
                if *disposition
                    != HazardGuidanceDisposition::SpeciesSpecificCultivationGuidanceAllowed
                {
                    push_profile(
                        &mut blockers,
                        ProfileBlockerCode::SpeciesGuidanceNotAllowed,
                        "site review allows general planning only",
                    );
                }
            }
        }

        if let Some(language) = &self.preferred_language {
            if !is_language_tag(language) {
                push_profile(
                    &mut blockers,
                    ProfileBlockerCode::InvalidPreferredLanguage,
                    "preferred_language must be a simple language tag",
                );
            }
        }

        blockers
    }
}

pub fn evaluate_local_catalogue(
    profile: &GardenSiteProfileV1,
    manifest: &LocalGrowingCatalogueManifestV1,
    now_utc: &str,
) -> CatalogueDecision {
    let profile_blockers = profile.catalogue_blockers();
    let mut catalogue_blockers = Vec::new();

    if !is_valid_utc_timestamp(now_utc) {
        push_catalogue(
            &mut catalogue_blockers,
            CatalogueBlockerCode::InvalidEvaluationTimestamp,
            "evaluation time must be a valid UTC timestamp",
        );
    }
    if manifest.schema_version != GARDEN_SITE_SCHEMA_VERSION {
        push_catalogue(
            &mut catalogue_blockers,
            CatalogueBlockerCode::UnsupportedManifestSchema,
            "manifest schema version is unsupported",
        );
    }
    if manifest.manifest_id.trim().is_empty() {
        push_catalogue(
            &mut catalogue_blockers,
            CatalogueBlockerCode::EmptyManifestId,
            "manifest_id must be non-empty",
        );
    }
    if manifest.catalogue_version.trim().is_empty() {
        push_catalogue(
            &mut catalogue_blockers,
            CatalogueBlockerCode::EmptyCatalogueVersion,
            "catalogue_version must be non-empty",
        );
    }
    if manifest.jurisdiction_code.trim().is_empty() {
        push_catalogue(
            &mut catalogue_blockers,
            CatalogueBlockerCode::EmptyManifestJurisdiction,
            "manifest jurisdiction must be non-empty",
        );
    } else if !is_country_code(&manifest.jurisdiction_code) {
        push_catalogue(
            &mut catalogue_blockers,
            CatalogueBlockerCode::InvalidManifestJurisdictionCode,
            "manifest jurisdiction must be a two-letter uppercase country code",
        );
    }
    if manifest.catalogue_region_id.trim().is_empty() {
        push_catalogue(
            &mut catalogue_blockers,
            CatalogueBlockerCode::EmptyManifestRegion,
            "manifest region must be non-empty",
        );
    } else if !is_region_id(&manifest.catalogue_region_id) {
        push_catalogue(
            &mut catalogue_blockers,
            CatalogueBlockerCode::InvalidManifestRegionFormat,
            "manifest region must be a reviewed region token, not an address",
        );
    }
    if !profile.jurisdiction_code.trim().is_empty()
        && profile.jurisdiction_code != manifest.jurisdiction_code
    {
        push_catalogue(
            &mut catalogue_blockers,
            CatalogueBlockerCode::JurisdictionMismatch,
            "profile and catalogue jurisdictions do not match",
        );
    }
    if profile.catalogue_region_id.as_deref() != Some(manifest.catalogue_region_id.as_str()) {
        push_catalogue(
            &mut catalogue_blockers,
            CatalogueBlockerCode::RegionMismatch,
            "profile and catalogue regions do not match",
        );
    }

    match manifest.status {
        ManifestStatus::Draft | ManifestStatus::Suspended => push_catalogue(
            &mut catalogue_blockers,
            CatalogueBlockerCode::ManifestNotApproved,
            "catalogue manifest is not active and approved",
        ),
        ManifestStatus::Approved => {
            let Some(approval) = &manifest.approval else {
                push_catalogue(
                    &mut catalogue_blockers,
                    CatalogueBlockerCode::MissingManifestApproval,
                    "approved manifest needs approval metadata",
                );
                return finish_decision(
                    profile_blockers,
                    catalogue_blockers,
                    Vec::new(),
                    Vec::new(),
                );
            };
            if approval.reviewer_id.trim().is_empty()
                || approval.evidence_ref.trim().is_empty()
                || !is_valid_utc_timestamp(&approval.reviewed_at_utc)
            {
                push_catalogue(
                    &mut catalogue_blockers,
                    CatalogueBlockerCode::InvalidManifestApproval,
                    "manifest approval metadata is incomplete or malformed",
                );
            } else if is_valid_utc_timestamp(now_utc) && approval.reviewed_at_utc > now_utc {
                push_catalogue(
                    &mut catalogue_blockers,
                    CatalogueBlockerCode::ManifestReviewedInFuture,
                    "manifest review timestamp is in the future",
                );
            }
            if !is_valid_utc_timestamp(&manifest.expires_at_utc)
                || !is_valid_utc_timestamp(&approval.reviewed_at_utc)
                || manifest.expires_at_utc <= approval.reviewed_at_utc
            {
                push_catalogue(
                    &mut catalogue_blockers,
                    CatalogueBlockerCode::InvalidManifestWindow,
                    "manifest expiry must be valid and later than its review time",
                );
            } else if is_valid_utc_timestamp(now_utc) && manifest.expires_at_utc <= now_utc {
                push_catalogue(
                    &mut catalogue_blockers,
                    CatalogueBlockerCode::ManifestExpired,
                    "catalogue manifest has expired",
                );
            }
        }
    }

    let mut seen = BTreeSet::new();
    for entry in &manifest.entries {
        if !seen.insert(entry.entry_id.clone()) {
            push_catalogue(
                &mut catalogue_blockers,
                CatalogueBlockerCode::DuplicateEntryId,
                "catalogue contains duplicate entry identifiers",
            );
            break;
        }
    }

    if !profile_blockers.is_empty() || !catalogue_blockers.is_empty() {
        return finish_decision(
            profile_blockers,
            catalogue_blockers,
            Vec::new(),
            Vec::new(),
        );
    }

    let mut visible_entry_ids = Vec::new();
    let mut excluded_entries = Vec::new();
    for entry in &manifest.entries {
        let reasons = entry.exclusion_reasons();
        if reasons.is_empty() {
            visible_entry_ids.push(entry.entry_id.clone());
        } else {
            excluded_entries.push(ExcludedEntry {
                entry_id: entry.entry_id.clone(),
                reasons,
            });
        }
    }
    visible_entry_ids.sort();
    excluded_entries.sort_by(|a, b| a.entry_id.cmp(&b.entry_id));
    finish_decision(Vec::new(), Vec::new(), visible_entry_ids, excluded_entries)
}

impl LocalGrowingCatalogueEntry {
    pub fn exclusion_reasons(&self) -> Vec<EntryExclusionCode> {
        let mut reasons = Vec::new();
        if self.entry_id.trim().is_empty() {
            reasons.push(EntryExclusionCode::EmptyEntryId);
        }
        if self.display_label.trim().is_empty() {
            reasons.push(EntryExclusionCode::EmptyDisplayLabel);
        }
        if self.taxon_ref.trim().is_empty() {
            reasons.push(EntryExclusionCode::EmptyTaxonReference);
        }
        check_review(
            &self.taxon_review,
            EntryExclusionCode::TaxonUnreviewed,
            EntryExclusionCode::TaxonRejected,
            &mut reasons,
        );
        match &self.conservation_review {
            ConservationReview::Unknown => reasons.push(EntryExclusionCode::ConservationUnknown),
            ConservationReview::RestrictionsApply { evidence_ref } => {
                reasons.push(EntryExclusionCode::ConservationRestricted);
                if evidence_ref.trim().is_empty() {
                    reasons.push(EntryExclusionCode::InvalidConservationEvidence);
                }
            }
            ConservationReview::Prohibited { evidence_ref } => {
                reasons.push(EntryExclusionCode::ConservationProhibited);
                if evidence_ref.trim().is_empty() {
                    reasons.push(EntryExclusionCode::InvalidConservationEvidence);
                }
            }
            ConservationReview::ApprovedForCultivation { evidence_ref } => {
                if evidence_ref.trim().is_empty() {
                    reasons.push(EntryExclusionCode::InvalidConservationEvidence);
                }
            }
        }
        match &self.knowledge_disclosure {
            KnowledgeDisclosure::Unknown => {
                reasons.push(EntryExclusionCode::KnowledgeRightsUnknown)
            }
            KnowledgeDisclosure::Restricted => {
                reasons.push(EntryExclusionCode::KnowledgeRestricted)
            }
            KnowledgeDisclosure::Withheld => reasons.push(EntryExclusionCode::KnowledgeWithheld),
            KnowledgeDisclosure::PubliclyShareable { evidence_ref } => {
                if evidence_ref.trim().is_empty() {
                    reasons.push(EntryExclusionCode::InvalidKnowledgeEvidence);
                }
            }
        }
        check_review(
            &self.cultivation_guide_review,
            EntryExclusionCode::GuideUnreviewed,
            EntryExclusionCode::GuideRejected,
            &mut reasons,
        );
        reasons
    }
}

fn check_review(
    review: &ReviewState,
    unreviewed: EntryExclusionCode,
    rejected: EntryExclusionCode,
    reasons: &mut Vec<EntryExclusionCode>,
) {
    match review {
        ReviewState::Unreviewed => reasons.push(unreviewed),
        ReviewState::Rejected { evidence_ref } => {
            reasons.push(rejected);
            if evidence_ref.trim().is_empty() {
                reasons.push(match rejected {
                    EntryExclusionCode::TaxonRejected => EntryExclusionCode::InvalidTaxonEvidence,
                    _ => EntryExclusionCode::InvalidGuideEvidence,
                });
            }
        }
        ReviewState::Approved { evidence_ref } => {
            if evidence_ref.trim().is_empty() {
                reasons.push(match unreviewed {
                    EntryExclusionCode::TaxonUnreviewed => EntryExclusionCode::InvalidTaxonEvidence,
                    _ => EntryExclusionCode::InvalidGuideEvidence,
                });
            }
        }
    }
}

fn finish_decision(
    profile_blockers: Vec<ProfileBlocker>,
    catalogue_blockers: Vec<CatalogueBlocker>,
    visible_entry_ids: Vec<String>,
    excluded_entries: Vec<ExcludedEntry>,
) -> CatalogueDecision {
    let catalogue_available = profile_blockers.is_empty() && catalogue_blockers.is_empty();
    CatalogueDecision {
        catalogue_available,
        no_eligible_entries: visible_entry_ids.is_empty(),
        visible_entry_ids,
        excluded_entries,
        profile_blockers,
        catalogue_blockers,
    }
}

fn push_profile(
    blockers: &mut Vec<ProfileBlocker>,
    code: ProfileBlockerCode,
    detail: &str,
) {
    blockers.push(ProfileBlocker {
        code,
        detail: detail.to_owned(),
    });
}

fn push_catalogue(
    blockers: &mut Vec<CatalogueBlocker>,
    code: CatalogueBlockerCode,
    detail: &str,
) {
    blockers.push(CatalogueBlocker {
        code,
        detail: detail.to_owned(),
    });
}

/// Format check for an ISO-3166 alpha-2-shaped jurisdiction token. This does
/// not validate the code against a maintained country registry.
fn is_country_code(value: &str) -> bool {
    value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn is_opaque_identifier(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && [8, 13, 18, 23].into_iter().all(|index| bytes[index] == b'-')
        && bytes.iter().enumerate().all(|(index, byte)| {
            [8, 13, 18, 23].contains(&index)
                || (byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
}

fn is_region_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 48
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value.bytes().any(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_language_tag(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn is_valid_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    for index in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !bytes[index].is_ascii_digit() {
            return false;
        }
    }
    let number = |start: usize, end: usize| -> u32 {
        value[start..end].parse::<u32>().unwrap_or(u32::MAX)
    };
    let year = number(0, 4);
    let month = number(5, 7);
    let day = number(8, 10);
    let hour = number(11, 13);
    let minute = number(14, 16);
    let second = number(17, 19);
    if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap_year = year.is_multiple_of(4)
        && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-10-14T15:00:00Z";

    fn good_profile() -> GardenSiteProfileV1 {
        GardenSiteProfileV1 {
            profile_id: "8c2f53a4-9921-4b78-8ab5-a0b1234c9001".to_owned(),
            schema_version: GARDEN_SITE_SCHEMA_VERSION,
            jurisdiction_code: "ZA".to_owned(),
            catalogue_region_id: Some("gauteng-urban".to_owned()),
            growing_space: Some(GrowingSpace::CommunityGarden),
            sun_exposure: Some(SunExposure::PartSunPartShade),
            water_access: Some(WaterAccess::Seasonal),
            land_permission: LandPermission::Granted {
                evidence_ref: "synthetic-permission-review-001".to_owned(),
            },
            hazard_screening: HazardScreening::ReviewedForPlanning {
                review_ref: "synthetic-site-review-001".to_owned(),
                disposition: HazardGuidanceDisposition::SpeciesSpecificCultivationGuidanceAllowed,
            },
            preferred_language: Some("en-ZA".to_owned()),
            interface_preferences: vec![InterfacePreference::LowBandwidth],
            sync_state: SyncState::LocalOnly,
        }
    }

    fn approved_entry() -> LocalGrowingCatalogueEntry {
        LocalGrowingCatalogueEntry {
            entry_id: "SYNTHETIC-ENTRY-001".to_owned(),
            display_label: "Synthetic non-clinical growing record".to_owned(),
            taxon_ref: "SYNTHETIC-TAXON-001".to_owned(),
            taxon_review: ReviewState::Approved {
                evidence_ref: "synthetic-taxon-review".to_owned(),
            },
            conservation_review: ConservationReview::ApprovedForCultivation {
                evidence_ref: "synthetic-conservation-review".to_owned(),
            },
            knowledge_disclosure: KnowledgeDisclosure::PubliclyShareable {
                evidence_ref: "synthetic-disclosure-review".to_owned(),
            },
            cultivation_guide_review: ReviewState::Approved {
                evidence_ref: "synthetic-guide-review".to_owned(),
            },
        }
    }

    fn approved_manifest(
        entries: Vec<LocalGrowingCatalogueEntry>,
    ) -> LocalGrowingCatalogueManifestV1 {
        LocalGrowingCatalogueManifestV1 {
            schema_version: GARDEN_SITE_SCHEMA_VERSION,
            manifest_id: "SYNTHETIC-MANIFEST-001".to_owned(),
            catalogue_version: "2026.10-synthetic-1".to_owned(),
            jurisdiction_code: "ZA".to_owned(),
            catalogue_region_id: "gauteng-urban".to_owned(),
            status: ManifestStatus::Approved,
            approval: Some(ManifestApproval {
                reviewer_id: "synthetic-reviewer".to_owned(),
                evidence_ref: "synthetic-manifest-review".to_owned(),
                reviewed_at_utc: "2026-10-01T00:00:00Z".to_owned(),
            }),
            expires_at_utc: "2027-10-01T00:00:00Z".to_owned(),
            entries,
        }
    }

    fn has_profile_code(decision: &CatalogueDecision, code: ProfileBlockerCode) -> bool {
        decision.profile_blockers.iter().any(|blocker| blocker.code == code)
    }

    fn has_catalogue_code(decision: &CatalogueDecision, code: CatalogueBlockerCode) -> bool {
        decision.catalogue_blockers.iter().any(|blocker| blocker.code == code)
    }

    #[test]
    fn local_only_is_the_default_and_sync_is_never_implicit() {
        assert_eq!(SyncState::default(), SyncState::LocalOnly);
        assert!(!good_profile().has_structurally_valid_sync_opt_in());
        assert!(good_profile().catalogue_blockers().is_empty());
    }

    #[test]
    fn human_readable_profile_ids_are_rejected() {
        let mut profile = good_profile();
        profile.profile_id = "my-home-garden-johannesburg".to_owned();
        assert!(has_profile_code(
            &evaluate_local_catalogue(&profile, &approved_manifest(vec![approved_entry()]), NOW),
            ProfileBlockerCode::InvalidProfileIdFormat
        ));
    }

    #[test]
    fn jurisdiction_must_be_a_country_code_not_free_text() {
        let mut profile = good_profile();
        profile.jurisdiction_code = "Johannesburg".to_owned();
        assert!(has_profile_code(
            &evaluate_local_catalogue(
                &profile,
                &approved_manifest(vec![approved_entry()]),
                NOW
            ),
            ProfileBlockerCode::InvalidJurisdictionCode
        ));
        profile.jurisdiction_code = "za".to_owned();
        assert!(has_profile_code(
            &evaluate_local_catalogue(
                &profile,
                &approved_manifest(vec![approved_entry()]),
                NOW
            ),
            ProfileBlockerCode::InvalidJurisdictionCode
        ));
    }

    #[test]
    fn free_form_addresses_are_not_valid_region_tokens() {
        let mut profile = good_profile();
        profile.catalogue_region_id = Some("123-main-street".to_owned());
        assert!(has_profile_code(
            &evaluate_local_catalogue(&profile, &approved_manifest(vec![approved_entry()]), NOW),
            ProfileBlockerCode::InvalidCatalogueRegionId
        ));
    }

    #[test]
    fn region_tokens_cannot_look_like_numbered_street_addresses() {
        assert!(!is_region_id("123-main-street"));
        assert!(is_region_id("gauteng-urban"));
    }

    #[test]
    fn missing_site_facts_remain_unresolved() {
        let mut profile = good_profile();
        profile.sun_exposure = None;
        profile.water_access = None;
        let blockers = profile.catalogue_blockers();
        assert!(blockers.iter().any(|b| b.code == ProfileBlockerCode::MissingSunExposure));
        assert!(blockers.iter().any(|b| b.code == ProfileBlockerCode::MissingWaterAccess));
    }

    #[test]
    fn blank_profile_id_and_unsupported_schema_block_catalogue() {
        let mut profile = good_profile();
        profile.profile_id.clear();
        profile.schema_version = 99;
        let blockers = profile.catalogue_blockers();
        assert!(blockers.iter().any(|b| b.code == ProfileBlockerCode::EmptyProfileId));
        assert!(blockers.iter().any(|b| b.code == ProfileBlockerCode::UnsupportedProfileSchema));
    }

    #[test]
    fn denied_or_unknown_land_permission_blocks_catalogue() {
        let mut profile = good_profile();
        profile.land_permission = LandPermission::Denied;
        assert!(has_profile_code(
            &evaluate_local_catalogue(&profile, &approved_manifest(vec![approved_entry()]), NOW),
            ProfileBlockerCode::LandPermissionDenied
        ));
        profile.land_permission = LandPermission::Unknown;
        assert!(has_profile_code(
            &evaluate_local_catalogue(&profile, &approved_manifest(vec![approved_entry()]), NOW),
            ProfileBlockerCode::LandPermissionUnresolved
        ));
    }

    #[test]
    fn hazard_not_assessed_does_not_allow_species_specific_guidance() {
        let mut profile = good_profile();
        profile.hazard_screening = HazardScreening::NotAssessed;
        let decision = evaluate_local_catalogue(
            &profile,
            &approved_manifest(vec![approved_entry()]),
            NOW,
        );
        assert!(!decision.catalogue_available);
        assert!(has_profile_code(&decision, ProfileBlockerCode::HazardNotAssessed));
        assert!(decision.visible_entry_ids.is_empty());
    }

    #[test]
    fn general_planning_only_hazard_review_does_not_unlock_species_catalogue() {
        let mut profile = good_profile();
        profile.hazard_screening = HazardScreening::ReviewedForPlanning {
            review_ref: "synthetic-review".to_owned(),
            disposition: HazardGuidanceDisposition::GeneralPlanningOnly,
        };
        assert!(has_profile_code(
            &evaluate_local_catalogue(&profile, &approved_manifest(vec![approved_entry()]), NOW),
            ProfileBlockerCode::SpeciesGuidanceNotAllowed
        ));
    }

    #[test]
    fn invalid_sync_consent_never_enables_sync_or_blocks_local_use() {
        let mut profile = good_profile();
        profile.sync_state = SyncState::OptedIn {
            consent_ref: " ".to_owned(),
            consented_at_utc: "not-a-time".to_owned(),
        };
        assert!(!profile.has_structurally_valid_sync_opt_in());
        let decision = evaluate_local_catalogue(
            &profile,
            &approved_manifest(vec![approved_entry()]),
            NOW,
        );
        assert!(decision.catalogue_available);
        assert_eq!(decision.visible_entry_ids.len(), 1);
    }

    #[test]
    fn an_opt_in_record_only_passes_structural_validation() {
        let mut profile = good_profile();
        profile.sync_state = SyncState::OptedIn {
            consent_ref: "synthetic-consent".to_owned(),
            consented_at_utc: NOW.to_owned(),
        };
        assert!(profile.has_structurally_valid_sync_opt_in());
    }

    #[test]
    fn sync_withdrawal_is_explicit_and_disables_sync() {
        let mut profile = good_profile();
        profile.sync_state = SyncState::Withdrawn {
            revocation_ref: "synthetic-revocation".to_owned(),
            withdrawn_at_utc: NOW.to_owned(),
        };
        assert!(!profile.has_structurally_valid_sync_opt_in());
        assert!(profile.catalogue_blockers().is_empty());
    }

    #[test]
    fn malformed_language_tag_is_rejected() {
        let mut profile = good_profile();
        profile.preferred_language = Some("--en--".to_owned());
        assert!(has_profile_code(
            &evaluate_local_catalogue(&profile, &approved_manifest(vec![approved_entry()]), NOW),
            ProfileBlockerCode::InvalidPreferredLanguage
        ));
    }

    #[test]
    fn exact_region_and_jurisdiction_must_match() {
        let profile = good_profile();
        let mut manifest = approved_manifest(vec![approved_entry()]);
        manifest.catalogue_region_id = "wrong-region".to_owned();
        let decision = evaluate_local_catalogue(&profile, &manifest, NOW);
        assert!(has_catalogue_code(&decision, CatalogueBlockerCode::RegionMismatch));
        assert!(decision.visible_entry_ids.is_empty());

        manifest = approved_manifest(vec![approved_entry()]);
        manifest.jurisdiction_code = "XX".to_owned();
        let decision = evaluate_local_catalogue(&profile, &manifest, NOW);
        assert!(has_catalogue_code(&decision, CatalogueBlockerCode::JurisdictionMismatch));
    }

    #[test]
    fn manifest_jurisdiction_must_be_a_country_code() {
        let profile = good_profile();
        let mut manifest = approved_manifest(vec![approved_entry()]);
        manifest.jurisdiction_code = "Johannesburg".to_owned();
        assert!(has_catalogue_code(
            &evaluate_local_catalogue(&profile, &manifest, NOW),
            CatalogueBlockerCode::InvalidManifestJurisdictionCode
        ));
    }

    #[test]
    fn free_form_manifest_region_is_rejected() {
        let profile = good_profile();
        let mut manifest = approved_manifest(vec![approved_entry()]);
        manifest.catalogue_region_id = "123-main-street".to_owned();
        assert!(has_catalogue_code(
            &evaluate_local_catalogue(&profile, &manifest, NOW),
            CatalogueBlockerCode::InvalidManifestRegionFormat
        ));
    }

    #[test]
    fn draft_suspended_and_expired_manifests_are_not_usable() {
        let profile = good_profile();
        let mut manifest = approved_manifest(vec![approved_entry()]);
        manifest.status = ManifestStatus::Draft;
        assert!(has_catalogue_code(
            &evaluate_local_catalogue(&profile, &manifest, NOW),
            CatalogueBlockerCode::ManifestNotApproved
        ));
        manifest.status = ManifestStatus::Suspended;
        assert!(has_catalogue_code(
            &evaluate_local_catalogue(&profile, &manifest, NOW),
            CatalogueBlockerCode::ManifestNotApproved
        ));
        manifest = approved_manifest(vec![approved_entry()]);
        manifest.expires_at_utc = "2026-10-14T14:59:59Z".to_owned();
        assert!(has_catalogue_code(
            &evaluate_local_catalogue(&profile, &manifest, NOW),
            CatalogueBlockerCode::ManifestExpired
        ));
    }

    #[test]
    fn future_review_and_invalid_manifest_expiry_are_rejected() {
        let profile = good_profile();
        let mut manifest = approved_manifest(vec![approved_entry()]);
        manifest.approval.as_mut().unwrap().reviewed_at_utc = "2026-10-15T00:00:00Z".to_owned();
        assert!(has_catalogue_code(
            &evaluate_local_catalogue(&profile, &manifest, NOW),
            CatalogueBlockerCode::ManifestReviewedInFuture
        ));
        manifest.approval.as_mut().unwrap().reviewed_at_utc = "2026-10-01T00:00:00Z".to_owned();
        manifest.expires_at_utc = "2026-09-01T00:00:00Z".to_owned();
        assert!(has_catalogue_code(
            &evaluate_local_catalogue(&profile, &manifest, NOW),
            CatalogueBlockerCode::InvalidManifestWindow
        ));
    }

    #[test]
    fn invalid_evaluation_time_blocks_all_catalogue_entries() {
        let decision = evaluate_local_catalogue(
            &good_profile(),
            &approved_manifest(vec![approved_entry()]),
            "yesterday",
        );
        assert!(has_catalogue_code(
            &decision,
            CatalogueBlockerCode::InvalidEvaluationTimestamp
        ));
        assert!(decision.visible_entry_ids.is_empty());
    }

    #[test]
    fn one_eligible_entry_is_visible_only_when_all_gates_pass() {
        let decision = evaluate_local_catalogue(
            &good_profile(),
            &approved_manifest(vec![approved_entry()]),
            NOW,
        );
        assert!(decision.catalogue_available);
        assert_eq!(decision.visible_entry_ids, vec!["SYNTHETIC-ENTRY-001"]);
        assert!(decision.excluded_entries.is_empty());
    }

    #[test]
    fn unknown_conservation_status_excludes_entry_without_poisoning_other_entries() {
        let mut excluded = approved_entry();
        excluded.entry_id = "SYNTHETIC-ENTRY-002".to_owned();
        excluded.conservation_review = ConservationReview::Unknown;
        let decision = evaluate_local_catalogue(
            &good_profile(),
            &approved_manifest(vec![approved_entry(), excluded]),
            NOW,
        );
        assert!(decision.catalogue_available);
        assert_eq!(decision.visible_entry_ids, vec!["SYNTHETIC-ENTRY-001"]);
        assert_eq!(decision.excluded_entries.len(), 1);
        assert_eq!(decision.excluded_entries[0].entry_id, "SYNTHETIC-ENTRY-002");
        assert!(decision.excluded_entries[0].reasons.contains(
            &EntryExclusionCode::ConservationUnknown
        ));
    }

    #[test]
    fn restricted_knowledge_and_unreviewed_guidance_are_never_shown() {
        let mut entry = approved_entry();
        entry.knowledge_disclosure = KnowledgeDisclosure::Restricted;
        entry.cultivation_guide_review = ReviewState::Unreviewed;
        let decision = evaluate_local_catalogue(
            &good_profile(),
            &approved_manifest(vec![entry]),
            NOW,
        );
        assert!(decision.visible_entry_ids.is_empty());
        assert!(decision.excluded_entries[0].reasons.contains(
            &EntryExclusionCode::KnowledgeRestricted
        ));
        assert!(decision.excluded_entries[0].reasons.contains(
            &EntryExclusionCode::GuideUnreviewed
        ));
    }

    #[test]
    fn an_approved_empty_manifest_is_allowed_and_stays_empty() {
        let decision = evaluate_local_catalogue(
            &good_profile(),
            &approved_manifest(Vec::new()),
            NOW,
        );
        assert!(decision.catalogue_available);
        assert!(decision.no_eligible_entries);
        assert!(decision.visible_entry_ids.is_empty());
    }

    #[test]
    fn duplicate_entry_ids_block_the_whole_manifest() {
        let entry = approved_entry();
        let decision = evaluate_local_catalogue(
            &good_profile(),
            &approved_manifest(vec![entry.clone(), entry]),
            NOW,
        );
        assert!(has_catalogue_code(&decision, CatalogueBlockerCode::DuplicateEntryId));
        assert!(!decision.catalogue_available);
        assert!(decision.visible_entry_ids.is_empty());
    }

    #[test]
    fn serialization_does_not_include_exact_location_or_health_fields() {
        let value = serde_json::to_value(good_profile()).unwrap();
        let object = value.as_object().unwrap();
        assert!(!object.contains_key("address"));
        assert!(!object.contains_key("latitude"));
        assert!(!object.contains_key("longitude"));
        assert!(!object.contains_key("gps"));
        assert!(!object.contains_key("medical_history"));
        assert!(!object.contains_key("patient_id"));
    }

    #[test]
    fn timestamps_validate_calendar_dates() {
        assert!(is_valid_utc_timestamp("2024-02-29T03:04:05Z"));
        assert!(!is_valid_utc_timestamp("2026-02-29T03:04:05Z"));
        assert!(!is_valid_utc_timestamp("2026-04-31T03:04:05Z"));
        assert!(!is_valid_utc_timestamp("2026-01-02T03:04:05+02:00"));
    }

    #[test]
    fn catalogue_blocker_order_is_deterministic() {
        let mut manifest = approved_manifest(Vec::new());
        manifest.schema_version = 99;
        manifest.catalogue_version.clear();
        manifest.catalogue_region_id.clear();
        let first = evaluate_local_catalogue(&good_profile(), &manifest, NOW);
        let second = evaluate_local_catalogue(&good_profile(), &manifest, NOW);
        assert_eq!(first, second);
    }
}
