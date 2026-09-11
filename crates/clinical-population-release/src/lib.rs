#![deny(unsafe_code)]
//! Privacy-qualified release boundary for population evidence.
//!
//! V1 deliberately exposes only two broad-release modes:
//!
//! - static pre-specified reports with primary + complementary suppression evidence;
//! - interactive releases backed by an explicit differential-privacy mechanism and
//!   a chained privacy-accountant receipt.
//!
//! Threshold-only interactive querying is intentionally absent.

use mycelix_clinical_integrity::{DigestDomain, IntegrityError, StoredDigest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

const RELEASE_HASH_CONTEXT: &str = "mycelix.health.population-release.v1";
const FRAME_VERSION: u8 = 1;
const MAX_ID_LEN: usize = 160;
const MAX_ALLOWED_DP_MECHANISMS: usize = 16;
/// Conservative mechanical floor for static-report cells. This is not a claim that
/// 11 is universally sufficient for privacy; deployment policy may only raise it.
pub const ABSOLUTE_STATIC_CELL_FLOOR: u64 = 11;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PopulationReleaseArtifactKindV1 {
    ReleasePolicy,
    StaticSuppressionEvidence,
    DpMechanismDescriptor,
    PrivacyAccountantReceipt,
    ReleaseRequest,
    ReleaseReceipt,
}

impl PopulationReleaseArtifactKindV1 {
    fn label(self) -> &'static [u8] {
        match self {
            Self::ReleasePolicy => b"release-policy",
            Self::StaticSuppressionEvidence => b"static-suppression-evidence",
            Self::DpMechanismDescriptor => b"dp-mechanism-descriptor",
            Self::PrivacyAccountantReceipt => b"privacy-accountant-receipt",
            Self::ReleaseRequest => b"release-request",
            Self::ReleaseReceipt => b"release-receipt",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PopulationReleaseDigestV1 {
    pub kind: PopulationReleaseArtifactKindV1,
    pub value: [u8; 32],
}

impl PopulationReleaseDigestV1 {
    pub fn validate(&self) -> Result<(), PopulationReleaseError> {
        if self.value == [0u8; 32] {
            return Err(PopulationReleaseError::ZeroReleaseDigest);
        }
        Ok(())
    }

    fn require_kind(
        self,
        expected: PopulationReleaseArtifactKindV1,
    ) -> Result<Self, PopulationReleaseError> {
        self.validate()?;
        if self.kind != expected {
            return Err(PopulationReleaseError::WrongReleaseDigestKind {
                expected,
                actual: self.kind,
            });
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PopulationFindingKindV1 {
    SafetySignalCandidate,
    DenominatorBasedAssociation,
    CausalEffectEstimate,
    PopulationReplication,
}

impl PopulationFindingKindV1 {
    fn expected_domain(self) -> DigestDomain {
        match self {
            Self::SafetySignalCandidate => DigestDomain::ClinicalSafetySignalCandidate,
            Self::DenominatorBasedAssociation => DigestDomain::ClinicalPopulationAssociation,
            Self::CausalEffectEstimate => DigestDomain::ClinicalCausalEffectEstimate,
            Self::PopulationReplication => DigestDomain::ClinicalPopulationReplication,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReducedFractionV1 {
    pub numerator: u64,
    pub denominator: u64,
}

impl ReducedFractionV1 {
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    pub fn validate(&self) -> Result<(), PopulationReleaseError> {
        if self.denominator == 0 {
            return Err(PopulationReleaseError::ZeroFractionDenominator);
        }
        if self.numerator == 0 {
            if self.denominator != 1 {
                return Err(PopulationReleaseError::NonCanonicalFraction);
            }
            return Ok(());
        }
        if gcd(self.numerator, self.denominator) != 1 {
            return Err(PopulationReleaseError::NonCanonicalFraction);
        }
        Ok(())
    }

    pub fn less_than_or_equal(self, other: Self) -> bool {
        (self.numerator as u128) * (other.denominator as u128)
            <= (other.numerator as u128) * (self.denominator as u128)
    }

    pub fn is_zero(self) -> bool {
        self.numerator == 0
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrivacyLossV1 {
    pub epsilon: ReducedFractionV1,
    pub delta: ReducedFractionV1,
}

impl PrivacyLossV1 {
    pub const ZERO: Self = Self {
        epsilon: ReducedFractionV1::ZERO,
        delta: ReducedFractionV1::ZERO,
    };

    pub fn validate(&self) -> Result<(), PopulationReleaseError> {
        self.epsilon.validate()?;
        self.delta.validate()?;
        if !self.delta.less_than_or_equal(ReducedFractionV1 {
            numerator: 1,
            denominator: 1,
        }) {
            return Err(PopulationReleaseError::DeltaOutsideUnitInterval);
        }
        Ok(())
    }

    fn less_than_or_equal(self, other: Self) -> bool {
        self.epsilon.less_than_or_equal(other.epsilon)
            && self.delta.less_than_or_equal(other.delta)
    }

    fn is_componentwise_non_decreasing_from(self, previous: Self) -> bool {
        previous.epsilon.less_than_or_equal(self.epsilon)
            && previous.delta.less_than_or_equal(self.delta)
    }

    fn is_zero(self) -> bool {
        self.epsilon.is_zero() && self.delta.is_zero()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DpMechanismKindV1 {
    Laplace,
    Gaussian,
    DiscreteGaussian,
    Exponential,
    OtherQualified,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PopulationReleasePolicyV1 {
    pub schema_version: u16,
    pub policy_id: String,
    /// Static/pre-specified reports only. Must be >= the software floor.
    pub static_minimum_cell_count: u64,
    pub max_static_report_cells: u64,
    pub allowed_dp_mechanisms: Vec<DpMechanismKindV1>,
    pub max_per_release_privacy_loss: PrivacyLossV1,
    pub max_cumulative_privacy_loss: PrivacyLossV1,
    pub max_interactive_queries: u64,
    /// Exact deployment-selected accountant instance and method. These are bound by
    /// policy so callers cannot pair a strong policy ID with a different accountant.
    pub accountant_instance_digest: StoredDigest,
    pub accountant_method_digest: StoredDigest,
}

impl PopulationReleasePolicyV1 {
    pub fn validate(&self) -> Result<(), PopulationReleaseError> {
        require_version(self.schema_version, "release policy")?;
        validate_id("release policy ID", &self.policy_id)?;
        if self.static_minimum_cell_count < ABSOLUTE_STATIC_CELL_FLOOR {
            return Err(PopulationReleaseError::StaticCellThresholdBelowSoftwareFloor);
        }
        if self.max_static_report_cells == 0 {
            return Err(PopulationReleaseError::ZeroStaticReportCellLimit);
        }
        if self.allowed_dp_mechanisms.is_empty()
            || self.allowed_dp_mechanisms.len() > MAX_ALLOWED_DP_MECHANISMS
        {
            return Err(PopulationReleaseError::InvalidAllowedDpMechanismSet);
        }
        let unique: BTreeSet<_> = self.allowed_dp_mechanisms.iter().copied().collect();
        if unique.len() != self.allowed_dp_mechanisms.len() {
            return Err(PopulationReleaseError::DuplicateAllowedDpMechanism);
        }
        self.max_per_release_privacy_loss.validate()?;
        self.max_cumulative_privacy_loss.validate()?;
        if self.max_per_release_privacy_loss.is_zero()
            || self.max_cumulative_privacy_loss.is_zero()
        {
            return Err(PopulationReleaseError::ZeroInteractivePrivacyBudget);
        }
        if !self
            .max_per_release_privacy_loss
            .less_than_or_equal(self.max_cumulative_privacy_loss)
        {
            return Err(PopulationReleaseError::PerReleaseBudgetExceedsCumulativeBudget);
        }
        if self.max_interactive_queries == 0 {
            return Err(PopulationReleaseError::ZeroInteractiveQueryLimit);
        }
        require_shape(self.accountant_instance_digest)?;
        require_shape(self.accountant_method_digest)?;
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<PopulationReleaseDigestV1, PopulationReleaseError> {
        self.validate()?;
        hash_json(PopulationReleaseArtifactKindV1::ReleasePolicy, self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StaticSuppressionEvidenceV1 {
    pub schema_version: u16,
    pub source_finding_kind: PopulationFindingKindV1,
    pub source_finding_digest: StoredDigest,
    pub report_specification_digest: StoredDigest,
    pub raw_aggregate_digest: StoredDigest,
    pub public_output_digest: StoredDigest,
    pub total_report_cells: u64,
    pub minimum_cell_threshold: u64,
    pub primary_suppression_count: u64,
    pub complementary_suppression_count: u64,
    pub primary_suppression_evidence_digest: StoredDigest,
    pub complementary_suppression_evidence_digest: StoredDigest,
    pub composition_review_evidence_digest: StoredDigest,
}

impl StaticSuppressionEvidenceV1 {
    pub fn validate(
        &self,
        policy: &PopulationReleasePolicyV1,
    ) -> Result<(), PopulationReleaseError> {
        require_version(self.schema_version, "static suppression evidence")?;
        validate_source_finding(self.source_finding_kind, self.source_finding_digest)?;
        for digest in [
            self.report_specification_digest,
            self.raw_aggregate_digest,
            self.public_output_digest,
            self.primary_suppression_evidence_digest,
            self.complementary_suppression_evidence_digest,
            self.composition_review_evidence_digest,
        ] {
            require_shape(digest)?;
        }
        if self.total_report_cells == 0
            || self.total_report_cells > policy.max_static_report_cells
        {
            return Err(PopulationReleaseError::InvalidStaticReportCellCount);
        }
        if self.minimum_cell_threshold < policy.static_minimum_cell_count
            || self.minimum_cell_threshold < ABSOLUTE_STATIC_CELL_FLOOR
        {
            return Err(PopulationReleaseError::StaticCellThresholdBelowPolicy);
        }
        let total_suppressed = self
            .primary_suppression_count
            .checked_add(self.complementary_suppression_count)
            .ok_or(PopulationReleaseError::SuppressionCountOverflow)?;
        if total_suppressed > self.total_report_cells {
            return Err(PopulationReleaseError::SuppressionCountExceedsReportCells);
        }
        if self.primary_suppression_evidence_digest
            == self.complementary_suppression_evidence_digest
            || self.primary_suppression_evidence_digest
                == self.composition_review_evidence_digest
            || self.complementary_suppression_evidence_digest
                == self.composition_review_evidence_digest
        {
            return Err(PopulationReleaseError::SuppressionEvidenceNotIndependentArtifacts);
        }
        Ok(())
    }

    pub fn verified_digest(
        &self,
        policy: &PopulationReleasePolicyV1,
    ) -> Result<PopulationReleaseDigestV1, PopulationReleaseError> {
        self.validate(policy)?;
        hash_json(
            PopulationReleaseArtifactKindV1::StaticSuppressionEvidence,
            self,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DifferentialPrivacyMechanismV1 {
    pub schema_version: u16,
    pub mechanism_id: String,
    pub mechanism_kind: DpMechanismKindV1,
    pub implementation_version: String,
    pub implementation_digest: StoredDigest,
    pub privacy_unit_definition_digest: StoredDigest,
    pub adjacency_definition_digest: StoredDigest,
    pub contribution_bounds_digest: StoredDigest,
    pub sensitivity_analysis_digest: StoredDigest,
    pub randomness_source_evidence_digest: StoredDigest,
    pub release_privacy_loss: PrivacyLossV1,
}

impl DifferentialPrivacyMechanismV1 {
    pub fn validate(
        &self,
        policy: &PopulationReleasePolicyV1,
    ) -> Result<(), PopulationReleaseError> {
        require_version(self.schema_version, "DP mechanism")?;
        validate_id("DP mechanism ID", &self.mechanism_id)?;
        validate_id("DP implementation version", &self.implementation_version)?;
        if !policy.allowed_dp_mechanisms.contains(&self.mechanism_kind) {
            return Err(PopulationReleaseError::DpMechanismNotAllowedByPolicy);
        }
        for digest in [
            self.implementation_digest,
            self.privacy_unit_definition_digest,
            self.adjacency_definition_digest,
            self.contribution_bounds_digest,
            self.sensitivity_analysis_digest,
            self.randomness_source_evidence_digest,
        ] {
            require_shape(digest)?;
        }
        self.release_privacy_loss.validate()?;
        if self.release_privacy_loss.epsilon.is_zero() {
            return Err(PopulationReleaseError::ZeroDpEpsilon);
        }
        if !self
            .release_privacy_loss
            .less_than_or_equal(policy.max_per_release_privacy_loss)
        {
            return Err(PopulationReleaseError::PerReleasePrivacyBudgetExceeded);
        }
        Ok(())
    }

    pub fn verified_digest(
        &self,
        policy: &PopulationReleasePolicyV1,
    ) -> Result<PopulationReleaseDigestV1, PopulationReleaseError> {
        self.validate(policy)?;
        hash_json(PopulationReleaseArtifactKindV1::DpMechanismDescriptor, self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrivacyAccountantReceiptV1 {
    pub schema_version: u16,
    pub accountant_instance_digest: StoredDigest,
    pub accountant_method_digest: StoredDigest,
    pub sequence: u64,
    pub previous_receipt_digest: Option<PopulationReleaseDigestV1>,
    pub cumulative_privacy_loss: PrivacyLossV1,
    pub query_count: u64,
    pub last_query_spec_digest: Option<StoredDigest>,
    pub last_mechanism_digest: Option<PopulationReleaseDigestV1>,
    pub last_output_digest: Option<StoredDigest>,
    pub accountant_evidence_digest: StoredDigest,
    pub observed_at_micros: i64,
}

impl PrivacyAccountantReceiptV1 {
    pub fn validate_shape(
        &self,
        policy: &PopulationReleasePolicyV1,
    ) -> Result<(), PopulationReleaseError> {
        require_version(self.schema_version, "privacy accountant receipt")?;
        if self.accountant_instance_digest != policy.accountant_instance_digest
            || self.accountant_method_digest != policy.accountant_method_digest
        {
            return Err(PopulationReleaseError::WrongPrivacyAccountant);
        }
        require_shape(self.accountant_instance_digest)?;
        require_shape(self.accountant_method_digest)?;
        require_shape(self.accountant_evidence_digest)?;
        self.cumulative_privacy_loss.validate()?;
        if !self
            .cumulative_privacy_loss
            .less_than_or_equal(policy.max_cumulative_privacy_loss)
        {
            return Err(PopulationReleaseError::CumulativePrivacyBudgetExceeded);
        }
        if self.query_count > policy.max_interactive_queries {
            return Err(PopulationReleaseError::InteractiveQueryLimitExceeded);
        }
        match (
            self.sequence,
            self.previous_receipt_digest,
            self.last_query_spec_digest,
            self.last_mechanism_digest,
            self.last_output_digest,
        ) {
            (0, None, None, None, None) => {
                if self.query_count != 0 || !self.cumulative_privacy_loss.is_zero() {
                    return Err(PopulationReleaseError::InvalidAccountantGenesis);
                }
            }
            (0, ..) => return Err(PopulationReleaseError::InvalidAccountantGenesis),
            (_, Some(previous), Some(query), Some(mechanism), Some(output)) => {
                previous.require_kind(PopulationReleaseArtifactKindV1::PrivacyAccountantReceipt)?;
                require_shape(query)?;
                mechanism.require_kind(PopulationReleaseArtifactKindV1::DpMechanismDescriptor)?;
                require_shape(output)?;
                if self.query_count != self.sequence {
                    return Err(PopulationReleaseError::AccountantSequenceQueryCountMismatch);
                }
            }
            _ => return Err(PopulationReleaseError::IncompleteAccountantLineage),
        }
        Ok(())
    }

    pub fn verified_digest(
        &self,
        policy: &PopulationReleasePolicyV1,
    ) -> Result<PopulationReleaseDigestV1, PopulationReleaseError> {
        self.validate_shape(policy)?;
        hash_json(
            PopulationReleaseArtifactKindV1::PrivacyAccountantReceipt,
            self,
        )
    }
}

/// Verify exact accountant-chain continuity. This does not re-implement the
/// accountant's mathematics; the exact accountant method/evidence remains explicit.
pub fn verify_accountant_transition(
    previous: &PrivacyAccountantReceiptV1,
    next: &PrivacyAccountantReceiptV1,
    policy: &PopulationReleasePolicyV1,
) -> Result<(), PopulationReleaseError> {
    previous.validate_shape(policy)?;
    next.validate_shape(policy)?;
    let previous_digest = previous.verified_digest(policy)?;
    if next.previous_receipt_digest != Some(previous_digest) {
        return Err(PopulationReleaseError::AccountantPredecessorMismatch);
    }
    if next.sequence != previous.sequence.checked_add(1).ok_or(
        PopulationReleaseError::AccountantSequenceOverflow,
    )? {
        return Err(PopulationReleaseError::AccountantSequenceGap);
    }
    if next.query_count != previous.query_count.checked_add(1).ok_or(
        PopulationReleaseError::AccountantQueryCountOverflow,
    )? {
        return Err(PopulationReleaseError::AccountantQueryCountGap);
    }
    if !next
        .cumulative_privacy_loss
        .is_componentwise_non_decreasing_from(previous.cumulative_privacy_loss)
    {
        return Err(PopulationReleaseError::PrivacyLossDecreased);
    }
    if next.observed_at_micros < previous.observed_at_micros {
        return Err(PopulationReleaseError::AccountantTimeWentBackwards);
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StaticReleaseRequestV1 {
    pub suppression: StaticSuppressionEvidenceV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InteractiveDpReleaseRequestV1 {
    pub source_finding_kind: PopulationFindingKindV1,
    pub source_finding_digest: StoredDigest,
    pub query_specification_digest: StoredDigest,
    pub output_schema_digest: StoredDigest,
    pub public_output_digest: StoredDigest,
    pub mechanism: DifferentialPrivacyMechanismV1,
    pub accountant_before: PrivacyAccountantReceiptV1,
    pub accountant_after: PrivacyAccountantReceiptV1,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PopulationReleaseModeV1 {
    StaticPreSpecifiedReport(StaticReleaseRequestV1),
    InteractiveDifferentialPrivacy(InteractiveDpReleaseRequestV1),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PopulationReleaseRequestV1 {
    pub schema_version: u16,
    pub release_id: String,
    pub policy: PopulationReleasePolicyV1,
    pub mode: PopulationReleaseModeV1,
    pub requested_at_micros: i64,
}

impl PopulationReleaseRequestV1 {
    pub fn validate(&self) -> Result<(), PopulationReleaseError> {
        require_version(self.schema_version, "population release request")?;
        validate_id("release ID", &self.release_id)?;
        self.policy.validate()?;
        match &self.mode {
            PopulationReleaseModeV1::StaticPreSpecifiedReport(request) => {
                request.suppression.validate(&self.policy)?;
            }
            PopulationReleaseModeV1::InteractiveDifferentialPrivacy(request) => {
                validate_source_finding(
                    request.source_finding_kind,
                    request.source_finding_digest,
                )?;
                for digest in [
                    request.query_specification_digest,
                    request.output_schema_digest,
                    request.public_output_digest,
                ] {
                    require_shape(digest)?;
                }
                request.mechanism.validate(&self.policy)?;
                verify_accountant_transition(
                    &request.accountant_before,
                    &request.accountant_after,
                    &self.policy,
                )?;
                let mechanism_digest = request.mechanism.verified_digest(&self.policy)?;
                if request.accountant_after.last_query_spec_digest
                    != Some(request.query_specification_digest)
                    || request.accountant_after.last_mechanism_digest != Some(mechanism_digest)
                    || request.accountant_after.last_output_digest != Some(request.public_output_digest)
                {
                    return Err(PopulationReleaseError::AccountantTransitionDoesNotBindRelease);
                }
            }
        }
        Ok(())
    }

    pub fn verified_digest(&self) -> Result<PopulationReleaseDigestV1, PopulationReleaseError> {
        self.validate()?;
        hash_json(PopulationReleaseArtifactKindV1::ReleaseRequest, self)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PopulationReleaseModeClassV1 {
    StaticPreSpecifiedReport,
    InteractiveDifferentialPrivacy,
}

/// Non-serializable capability created only after all v1 release checks succeed.
pub struct PopulationReleaseCapabilityV1 {
    request_digest: PopulationReleaseDigestV1,
    policy_digest: PopulationReleaseDigestV1,
    source_finding_digest: StoredDigest,
    public_output_digest: StoredDigest,
    mode: PopulationReleaseModeClassV1,
    requested_at_micros: i64,
}

pub fn authorize_population_release(
    request: &PopulationReleaseRequestV1,
) -> Result<PopulationReleaseCapabilityV1, PopulationReleaseError> {
    request.validate()?;
    let request_digest = request.verified_digest()?;
    let policy_digest = request.policy.verified_digest()?;
    let (source_finding_digest, public_output_digest, mode) = match &request.mode {
        PopulationReleaseModeV1::StaticPreSpecifiedReport(static_request) => (
            static_request.suppression.source_finding_digest,
            static_request.suppression.public_output_digest,
            PopulationReleaseModeClassV1::StaticPreSpecifiedReport,
        ),
        PopulationReleaseModeV1::InteractiveDifferentialPrivacy(dp_request) => (
            dp_request.source_finding_digest,
            dp_request.public_output_digest,
            PopulationReleaseModeClassV1::InteractiveDifferentialPrivacy,
        ),
    };
    Ok(PopulationReleaseCapabilityV1 {
        request_digest,
        policy_digest,
        source_finding_digest,
        public_output_digest,
        mode,
        requested_at_micros: request.requested_at_micros,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PopulationReleaseReceiptV1 {
    pub schema_version: u16,
    pub request_digest: PopulationReleaseDigestV1,
    pub policy_digest: PopulationReleaseDigestV1,
    pub source_finding_digest: StoredDigest,
    pub public_output_digest: StoredDigest,
    pub mode: PopulationReleaseModeClassV1,
    pub requested_at_micros: i64,
    pub released_at_micros: i64,
}

impl PopulationReleaseCapabilityV1 {
    pub fn into_receipt(
        self,
        released_at_micros: i64,
    ) -> Result<PopulationReleaseReceiptV1, PopulationReleaseError> {
        if released_at_micros < self.requested_at_micros {
            return Err(PopulationReleaseError::ReleaseTimeBeforeRequest);
        }
        Ok(PopulationReleaseReceiptV1 {
            schema_version: 1,
            request_digest: self.request_digest,
            policy_digest: self.policy_digest,
            source_finding_digest: self.source_finding_digest,
            public_output_digest: self.public_output_digest,
            mode: self.mode,
            requested_at_micros: self.requested_at_micros,
            released_at_micros,
        })
    }
}

impl PopulationReleaseReceiptV1 {
    pub fn verified_digest(&self) -> Result<PopulationReleaseDigestV1, PopulationReleaseError> {
        require_version(self.schema_version, "population release receipt")?;
        self.request_digest
            .require_kind(PopulationReleaseArtifactKindV1::ReleaseRequest)?;
        self.policy_digest
            .require_kind(PopulationReleaseArtifactKindV1::ReleasePolicy)?;
        require_shape(self.source_finding_digest)?;
        require_shape(self.public_output_digest)?;
        if self.released_at_micros < self.requested_at_micros {
            return Err(PopulationReleaseError::ReleaseTimeBeforeRequest);
        }
        hash_json(PopulationReleaseArtifactKindV1::ReleaseReceipt, self)
    }
}

fn validate_source_finding(
    kind: PopulationFindingKindV1,
    digest: StoredDigest,
) -> Result<(), PopulationReleaseError> {
    require_shape(digest)?;
    let expected = kind.expected_domain();
    if digest.domain != expected {
        return Err(PopulationReleaseError::WrongSourceFindingDomain {
            expected,
            actual: digest.domain,
        });
    }
    Ok(())
}

fn require_shape(digest: StoredDigest) -> Result<(), PopulationReleaseError> {
    digest.validate_shape()?;
    Ok(())
}

fn require_version(version: u16, label: &'static str) -> Result<(), PopulationReleaseError> {
    if version != 1 {
        return Err(PopulationReleaseError::UnsupportedVersion { label, version });
    }
    Ok(())
}

fn validate_id(label: &'static str, value: &str) -> Result<(), PopulationReleaseError> {
    if value.trim().is_empty() {
        return Err(PopulationReleaseError::MissingId(label));
    }
    if value.len() > MAX_ID_LEN {
        return Err(PopulationReleaseError::OversizedId(label));
    }
    Ok(())
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

fn hash_json<T: Serialize>(
    kind: PopulationReleaseArtifactKindV1,
    value: &T,
) -> Result<PopulationReleaseDigestV1, PopulationReleaseError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| PopulationReleaseError::Serialization(error.to_string()))?;
    let label = kind.label();
    let mut hasher = blake3::Hasher::new_derive_key(RELEASE_HASH_CONTEXT);
    hasher.update(&[FRAME_VERSION]);
    hasher.update(&(label.len() as u16).to_be_bytes());
    hasher.update(label);
    hasher.update(&(encoded.len() as u64).to_be_bytes());
    hasher.update(&encoded);
    let value = *hasher.finalize().as_bytes();
    if value == [0u8; 32] {
        return Err(PopulationReleaseError::ZeroReleaseDigest);
    }
    Ok(PopulationReleaseDigestV1 { kind, value })
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PopulationReleaseError {
    #[error("unsupported {label} version {version}")]
    UnsupportedVersion { label: &'static str, version: u16 },
    #[error("{0} is required")]
    MissingId(&'static str),
    #[error("{0} exceeds the v1 identifier bound")]
    OversizedId(&'static str),
    #[error("release digest cannot be all zero")]
    ZeroReleaseDigest,
    #[error("release digest kind mismatch: expected {expected:?}, got {actual:?}")]
    WrongReleaseDigestKind {
        expected: PopulationReleaseArtifactKindV1,
        actual: PopulationReleaseArtifactKindV1,
    },
    #[error("fraction denominator cannot be zero")]
    ZeroFractionDenominator,
    #[error("fraction must use one reduced canonical representation")]
    NonCanonicalFraction,
    #[error("delta must be in the closed unit interval")]
    DeltaOutsideUnitInterval,
    #[error("static minimum cell count is below the software floor")]
    StaticCellThresholdBelowSoftwareFloor,
    #[error("static report cell limit cannot be zero")]
    ZeroStaticReportCellLimit,
    #[error("allowed DP mechanism set is empty or too large")]
    InvalidAllowedDpMechanismSet,
    #[error("allowed DP mechanism set contains duplicates")]
    DuplicateAllowedDpMechanism,
    #[error("interactive privacy budget cannot be zero")]
    ZeroInteractivePrivacyBudget,
    #[error("per-release privacy budget exceeds cumulative budget")]
    PerReleaseBudgetExceedsCumulativeBudget,
    #[error("interactive query limit cannot be zero")]
    ZeroInteractiveQueryLimit,
    #[error("source finding digest domain mismatch: expected {expected:?}, got {actual:?}")]
    WrongSourceFindingDomain {
        expected: DigestDomain,
        actual: DigestDomain,
    },
    #[error("static report cell count is zero or exceeds policy")]
    InvalidStaticReportCellCount,
    #[error("static cell threshold is below policy")]
    StaticCellThresholdBelowPolicy,
    #[error("suppression count overflow")]
    SuppressionCountOverflow,
    #[error("suppression counts exceed total report cells")]
    SuppressionCountExceedsReportCells,
    #[error("primary, complementary, and composition-review evidence must be distinct artifacts")]
    SuppressionEvidenceNotIndependentArtifacts,
    #[error("DP mechanism is not allowed by release policy")]
    DpMechanismNotAllowedByPolicy,
    #[error("DP epsilon must be greater than zero")]
    ZeroDpEpsilon,
    #[error("per-release privacy budget exceeded")]
    PerReleasePrivacyBudgetExceeded,
    #[error("privacy accountant does not match policy-bound instance/method")]
    WrongPrivacyAccountant,
    #[error("cumulative privacy budget exceeded")]
    CumulativePrivacyBudgetExceeded,
    #[error("interactive query limit exceeded")]
    InteractiveQueryLimitExceeded,
    #[error("privacy-accountant genesis receipt is invalid")]
    InvalidAccountantGenesis,
    #[error("privacy-accountant sequence and query count differ")]
    AccountantSequenceQueryCountMismatch,
    #[error("privacy-accountant lineage fields are incomplete")]
    IncompleteAccountantLineage,
    #[error("privacy-accountant predecessor digest mismatch")]
    AccountantPredecessorMismatch,
    #[error("privacy-accountant sequence overflow")]
    AccountantSequenceOverflow,
    #[error("privacy-accountant sequence has a gap")]
    AccountantSequenceGap,
    #[error("privacy-accountant query count overflow")]
    AccountantQueryCountOverflow,
    #[error("privacy-accountant query count has a gap")]
    AccountantQueryCountGap,
    #[error("cumulative privacy loss decreased")]
    PrivacyLossDecreased,
    #[error("privacy-accountant observation time went backwards")]
    AccountantTimeWentBackwards,
    #[error("privacy-accountant transition does not bind this exact query/mechanism/output")]
    AccountantTransitionDoesNotBindRelease,
    #[error("release time precedes request time")]
    ReleaseTimeBeforeRequest,
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}
