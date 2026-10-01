#![deny(unsafe_code)]
//! Bounded issuer trust-chain evaluation for Mycelix-Health.
//!
//! Credential author-binding proves which agent issued a record. This crate answers
//! the separate question: was that issuer authorized, under an explicit trust
//! policy, to issue this credential kind in this jurisdiction at this time?
//!
//! Trust is rooted in configured/verified anchors and may be delegated only through
//! bounded, verified grants. A credential issuer cannot bootstrap authority by
//! issuing a grant to itself.

use mycelix_clinical_authority::{
    CredentialKind, EvidenceDigest, JurisdictionCode, PrincipalBinding,
};
use std::collections::{HashSet, VecDeque};
use thiserror::Error;

pub const MAX_DELEGATION_DEPTH: u8 = 8;
pub const MAX_TRUST_ANCHORS: usize = 64;
pub const MAX_DELEGATION_GRANTS: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustAnchor {
    principal: PrincipalBinding,
    credential_kinds: Vec<CredentialKind>,
    jurisdictions: Vec<JurisdictionCode>,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    max_delegation_depth: u8,
    policy_digest: EvidenceDigest,
    anchor_evidence_digest: EvidenceDigest,
}

impl TrustAnchor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        principal: PrincipalBinding,
        credential_kinds: Vec<CredentialKind>,
        jurisdictions: Vec<JurisdictionCode>,
        valid_from_micros: i64,
        valid_until_micros: Option<i64>,
        max_delegation_depth: u8,
        policy_digest: EvidenceDigest,
        anchor_evidence_digest: EvidenceDigest,
    ) -> Result<Self, IssuerTrustError> {
        validate_principal(principal)?;
        validate_kind_set(&credential_kinds)?;
        validate_jurisdiction_set(&jurisdictions)?;
        validate_digest(policy_digest)?;
        validate_digest(anchor_evidence_digest)?;
        validate_validity_window(valid_from_micros, valid_until_micros)?;
        if max_delegation_depth > MAX_DELEGATION_DEPTH {
            return Err(IssuerTrustError::DelegationDepthTooLarge);
        }
        Ok(Self {
            principal,
            credential_kinds,
            jurisdictions,
            valid_from_micros,
            valid_until_micros,
            max_delegation_depth,
            policy_digest,
            anchor_evidence_digest,
        })
    }

    fn active_at(&self, at_micros: i64) -> bool {
        at_micros >= self.valid_from_micros
            && self
                .valid_until_micros
                .map(|until| at_micros < until)
                .unwrap_or(true)
    }

    fn allows(&self, kind: CredentialKind, jurisdiction: &JurisdictionCode) -> bool {
        self.credential_kinds.contains(&kind) && self.jurisdictions.contains(jurisdiction)
    }
}

/// Delegation grant that has already crossed its source-integrity boundary.
///
/// The adapter constructing this type is responsible for verifying that the grant
/// record was authored/signed by `grantor`, that its status/revocation query is
/// trustworthy, and that the digests bind those exact artifacts. This type has no
/// serde implementation so raw network input cannot deserialize directly into
/// trusted delegation evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedDelegationGrant {
    grantor: PrincipalBinding,
    grantee: PrincipalBinding,
    credential_kinds: Vec<CredentialKind>,
    jurisdictions: Vec<JurisdictionCode>,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    revoked_at_micros: Option<i64>,
    /// Maximum delegation depth the grantee may itself pass onward.
    child_delegation_depth: u8,
    grant_record_digest: EvidenceDigest,
    status_check_evidence_digest: EvidenceDigest,
}

impl VerifiedDelegationGrant {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_record(
        grantor: PrincipalBinding,
        grantee: PrincipalBinding,
        credential_kinds: Vec<CredentialKind>,
        jurisdictions: Vec<JurisdictionCode>,
        valid_from_micros: i64,
        valid_until_micros: Option<i64>,
        revoked_at_micros: Option<i64>,
        child_delegation_depth: u8,
        grant_record_digest: EvidenceDigest,
        status_check_evidence_digest: EvidenceDigest,
    ) -> Result<Self, IssuerTrustError> {
        validate_principal(grantor)?;
        validate_principal(grantee)?;
        if grantor == grantee {
            return Err(IssuerTrustError::SelfDelegationForbidden);
        }
        validate_kind_set(&credential_kinds)?;
        validate_jurisdiction_set(&jurisdictions)?;
        validate_validity_window(valid_from_micros, valid_until_micros)?;
        if let Some(revoked_at) = revoked_at_micros {
            if revoked_at < valid_from_micros {
                return Err(IssuerTrustError::RevocationPredatesValidity);
            }
        }
        if child_delegation_depth > MAX_DELEGATION_DEPTH {
            return Err(IssuerTrustError::DelegationDepthTooLarge);
        }
        validate_digest(grant_record_digest)?;
        validate_digest(status_check_evidence_digest)?;
        Ok(Self {
            grantor,
            grantee,
            credential_kinds,
            jurisdictions,
            valid_from_micros,
            valid_until_micros,
            revoked_at_micros,
            child_delegation_depth,
            grant_record_digest,
            status_check_evidence_digest,
        })
    }

    fn active_at(&self, at_micros: i64) -> bool {
        if at_micros < self.valid_from_micros {
            return false;
        }
        if self
            .valid_until_micros
            .map(|until| at_micros >= until)
            .unwrap_or(false)
        {
            return false;
        }
        if self
            .revoked_at_micros
            .map(|revoked| at_micros >= revoked)
            .unwrap_or(false)
        {
            return false;
        }
        true
    }

    fn allows(&self, kind: CredentialKind, jurisdiction: &JurisdictionCode) -> bool {
        self.credential_kinds.contains(&kind) && self.jurisdictions.contains(jurisdiction)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssuerAuthorityRequest {
    pub issuer: PrincipalBinding,
    pub credential_kind: CredentialKind,
    pub jurisdiction: JurisdictionCode,
    pub evaluated_at_micros: i64,
    pub policy_digest: EvidenceDigest,
}

impl IssuerAuthorityRequest {
    pub fn validate(&self) -> Result<(), IssuerTrustError> {
        validate_principal(self.issuer)?;
        self.jurisdiction
            .validate()
            .map_err(|_| IssuerTrustError::InvalidJurisdiction)?;
        validate_digest(self.policy_digest)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssuerTrustDecision {
    Authorized,
    Denied,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IssuerTrustDenial {
    NoApplicableTrustAnchor,
    NoValidTrustPath,
    DelegationDepthExceeded,
}

pub struct IssuerTrustEvaluation {
    pub decision: IssuerTrustDecision,
    pub denials: Vec<IssuerTrustDenial>,
    receipt: Option<IssuerAuthorityReceipt>,
}

impl IssuerTrustEvaluation {
    pub fn into_receipt(self) -> Option<IssuerAuthorityReceipt> {
        self.receipt
    }
}

/// Non-serializable, non-cloneable proof object produced only by successful trust
/// path evaluation. The ordered evidence digests bind the chosen anchor and every
/// delegation hop used to authorize the issuer.
pub struct IssuerAuthorityReceipt {
    issuer: PrincipalBinding,
    credential_kind: CredentialKind,
    jurisdiction: JurisdictionCode,
    evaluated_at_micros: i64,
    policy_digest: EvidenceDigest,
    path_length: u8,
    ordered_evidence_digests: Vec<EvidenceDigest>,
}

impl IssuerAuthorityReceipt {
    pub fn issuer(&self) -> PrincipalBinding {
        self.issuer
    }

    pub fn credential_kind(&self) -> CredentialKind {
        self.credential_kind
    }

    pub fn jurisdiction(&self) -> &JurisdictionCode {
        &self.jurisdiction
    }

    pub fn evaluated_at_micros(&self) -> i64 {
        self.evaluated_at_micros
    }

    pub fn policy_digest(&self) -> EvidenceDigest {
        self.policy_digest
    }

    pub fn path_length(&self) -> u8 {
        self.path_length
    }

    pub fn ordered_evidence_digests(&self) -> &[EvidenceDigest] {
        &self.ordered_evidence_digests
    }
}

#[derive(Clone)]
struct PathState {
    principal: PrincipalBinding,
    remaining_depth: u8,
    path_length: u8,
    evidence: Vec<EvidenceDigest>,
}

pub fn evaluate_issuer_authority(
    request: &IssuerAuthorityRequest,
    anchors: &[TrustAnchor],
    grants: &[VerifiedDelegationGrant],
) -> Result<IssuerTrustEvaluation, IssuerTrustError> {
    request.validate()?;
    if anchors.len() > MAX_TRUST_ANCHORS {
        return Err(IssuerTrustError::TooManyTrustAnchors);
    }
    if grants.len() > MAX_DELEGATION_GRANTS {
        return Err(IssuerTrustError::TooManyDelegationGrants);
    }

    let mut applicable_anchors: Vec<&TrustAnchor> = anchors
        .iter()
        .filter(|anchor| anchor.policy_digest == request.policy_digest)
        .filter(|anchor| anchor.active_at(request.evaluated_at_micros))
        .filter(|anchor| anchor.allows(request.credential_kind, &request.jurisdiction))
        .collect();

    applicable_anchors.sort_by_key(|anchor| anchor.anchor_evidence_digest);

    if applicable_anchors.is_empty() {
        return Ok(denied(IssuerTrustDenial::NoApplicableTrustAnchor));
    }

    let mut queue = VecDeque::new();
    let mut visited: HashSet<(PrincipalBinding, u8)> = HashSet::new();

    for anchor in applicable_anchors {
        let state = PathState {
            principal: anchor.principal,
            remaining_depth: anchor.max_delegation_depth,
            path_length: 0,
            evidence: vec![anchor.anchor_evidence_digest],
        };
        if state.principal == request.issuer {
            return Ok(authorized(request, state));
        }
        if visited.insert((state.principal, state.remaining_depth)) {
            queue.push_back(state);
        }
    }

    let mut saw_depth_violation = false;

    while let Some(state) = queue.pop_front() {
        let mut candidates: Vec<&VerifiedDelegationGrant> = grants
            .iter()
            .filter(|grant| grant.grantor == state.principal)
            .filter(|grant| grant.active_at(request.evaluated_at_micros))
            .filter(|grant| grant.allows(request.credential_kind, &request.jurisdiction))
            .collect();
        candidates.sort_by_key(|grant| grant.grant_record_digest);

        for grant in candidates {
            if state.remaining_depth == 0 {
                saw_depth_violation = true;
                continue;
            }
            let maximum_child_depth = state.remaining_depth - 1;
            if grant.child_delegation_depth > maximum_child_depth {
                saw_depth_violation = true;
                continue;
            }

            let path_length = state
                .path_length
                .checked_add(1)
                .ok_or(IssuerTrustError::PathLengthOverflow)?;
            let mut evidence = state.evidence.clone();
            evidence.push(grant.grant_record_digest);
            evidence.push(grant.status_check_evidence_digest);

            let next = PathState {
                principal: grant.grantee,
                remaining_depth: grant.child_delegation_depth,
                path_length,
                evidence,
            };

            if next.principal == request.issuer {
                return Ok(authorized(request, next));
            }

            if visited.insert((next.principal, next.remaining_depth)) {
                queue.push_back(next);
            }
        }
    }

    if saw_depth_violation {
        Ok(denied(IssuerTrustDenial::DelegationDepthExceeded))
    } else {
        Ok(denied(IssuerTrustDenial::NoValidTrustPath))
    }
}

fn authorized(request: &IssuerAuthorityRequest, state: PathState) -> IssuerTrustEvaluation {
    IssuerTrustEvaluation {
        decision: IssuerTrustDecision::Authorized,
        denials: Vec::new(),
        receipt: Some(IssuerAuthorityReceipt {
            issuer: request.issuer,
            credential_kind: request.credential_kind,
            jurisdiction: request.jurisdiction.clone(),
            evaluated_at_micros: request.evaluated_at_micros,
            policy_digest: request.policy_digest,
            path_length: state.path_length,
            ordered_evidence_digests: state.evidence,
        }),
    }
}

fn denied(reason: IssuerTrustDenial) -> IssuerTrustEvaluation {
    IssuerTrustEvaluation {
        decision: IssuerTrustDecision::Denied,
        denials: vec![reason],
        receipt: None,
    }
}

fn validate_principal(principal: PrincipalBinding) -> Result<(), IssuerTrustError> {
    if principal.0 == [0u8; 32] {
        return Err(IssuerTrustError::ZeroPrincipalBinding);
    }
    Ok(())
}

fn validate_digest(digest: EvidenceDigest) -> Result<(), IssuerTrustError> {
    if digest.0 == [0u8; 32] {
        return Err(IssuerTrustError::ZeroEvidenceDigest);
    }
    Ok(())
}

fn validate_kind_set(kinds: &[CredentialKind]) -> Result<(), IssuerTrustError> {
    if kinds.is_empty() {
        return Err(IssuerTrustError::MissingCredentialKinds);
    }
    let mut seen = HashSet::new();
    if kinds.iter().any(|kind| !seen.insert(*kind)) {
        return Err(IssuerTrustError::DuplicateCredentialKind);
    }
    Ok(())
}

fn validate_jurisdiction_set(
    jurisdictions: &[JurisdictionCode],
) -> Result<(), IssuerTrustError> {
    if jurisdictions.is_empty() {
        return Err(IssuerTrustError::MissingJurisdictions);
    }
    let mut seen = HashSet::new();
    for jurisdiction in jurisdictions {
        jurisdiction
            .validate()
            .map_err(|_| IssuerTrustError::InvalidJurisdiction)?;
        if !seen.insert((jurisdiction.system.clone(), jurisdiction.code.clone())) {
            return Err(IssuerTrustError::DuplicateJurisdiction);
        }
    }
    Ok(())
}

fn validate_validity_window(
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
) -> Result<(), IssuerTrustError> {
    if let Some(until) = valid_until_micros {
        if until <= valid_from_micros {
            return Err(IssuerTrustError::InvalidValidityWindow);
        }
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IssuerTrustError {
    #[error("principal binding must be non-zero")]
    ZeroPrincipalBinding,
    #[error("evidence digest must be non-zero")]
    ZeroEvidenceDigest,
    #[error("issuer authority requires at least one credential kind")]
    MissingCredentialKinds,
    #[error("issuer authority contains duplicate credential kind")]
    DuplicateCredentialKind,
    #[error("issuer authority requires at least one jurisdiction")]
    MissingJurisdictions,
    #[error("issuer authority contains invalid jurisdiction")]
    InvalidJurisdiction,
    #[error("issuer authority contains duplicate jurisdiction")]
    DuplicateJurisdiction,
    #[error("validity end must be after validity start")]
    InvalidValidityWindow,
    #[error("revocation cannot predate delegation validity")]
    RevocationPredatesValidity,
    #[error("issuer cannot delegate authority to itself")]
    SelfDelegationForbidden,
    #[error("delegation depth exceeds policy maximum")]
    DelegationDepthTooLarge,
    #[error("trust policy exceeds maximum anchor count")]
    TooManyTrustAnchors,
    #[error("trust policy exceeds maximum delegation grant count")]
    TooManyDelegationGrants,
    #[error("trust path length overflow")]
    PathLengthOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(byte: u8) -> PrincipalBinding {
        PrincipalBinding([byte; 32])
    }

    fn digest(byte: u8) -> EvidenceDigest {
        EvidenceDigest([byte; 32])
    }

    fn jurisdiction(code: &str) -> JurisdictionCode {
        JurisdictionCode {
            system: "urn:iso:std:iso:3166-2".into(),
            code: code.into(),
        }
    }

    fn anchor(root: u8, depth: u8) -> TrustAnchor {
        TrustAnchor::new(
            principal(root),
            vec![CredentialKind::PractitionerLicense],
            vec![jurisdiction("US-TX")],
            0,
            Some(1_000),
            depth,
            digest(90),
            digest(root.wrapping_add(100)),
        )
        .unwrap()
    }

    fn grant(grantor: u8, grantee: u8, child_depth: u8, seed: u8) -> VerifiedDelegationGrant {
        VerifiedDelegationGrant::from_verified_record(
            principal(grantor),
            principal(grantee),
            vec![CredentialKind::PractitionerLicense],
            vec![jurisdiction("US-TX")],
            0,
            Some(1_000),
            None,
            child_depth,
            digest(seed),
            digest(seed.wrapping_add(1)),
        )
        .unwrap()
    }

    fn request(issuer: u8) -> IssuerAuthorityRequest {
        IssuerAuthorityRequest {
            issuer: principal(issuer),
            credential_kind: CredentialKind::PractitionerLicense,
            jurisdiction: jurisdiction("US-TX"),
            evaluated_at_micros: 100,
            policy_digest: digest(90),
        }
    }

    #[test]
    fn trust_anchor_is_authorized_without_delegation() {
        let result = evaluate_issuer_authority(&request(1), &[anchor(1, 2)], &[]).unwrap();
        assert_eq!(result.decision, IssuerTrustDecision::Authorized);
        let receipt = result.into_receipt().unwrap();
        assert_eq!(receipt.path_length(), 0);
    }

    #[test]
    fn bounded_delegation_chain_authorizes_issuer() {
        let grants = [grant(1, 2, 1, 10), grant(2, 3, 0, 20)];
        let result = evaluate_issuer_authority(&request(3), &[anchor(1, 2)], &grants).unwrap();
        assert_eq!(result.decision, IssuerTrustDecision::Authorized);
        let receipt = result.into_receipt().unwrap();
        assert_eq!(receipt.path_length(), 2);
        assert_eq!(receipt.ordered_evidence_digests().len(), 5);
    }

    #[test]
    fn self_declared_issuer_without_anchor_is_denied() {
        let result = evaluate_issuer_authority(&request(7), &[], &[]).unwrap();
        assert_eq!(result.decision, IssuerTrustDecision::Denied);
        assert_eq!(
            result.denials,
            vec![IssuerTrustDenial::NoApplicableTrustAnchor]
        );
    }

    #[test]
    fn self_delegation_grant_is_rejected_at_construction() {
        let error = VerifiedDelegationGrant::from_verified_record(
            principal(1),
            principal(1),
            vec![CredentialKind::PractitionerLicense],
            vec![jurisdiction("US-TX")],
            0,
            Some(1_000),
            None,
            0,
            digest(10),
            digest(11),
        )
        .unwrap_err();
        assert_eq!(error, IssuerTrustError::SelfDelegationForbidden);
    }

    #[test]
    fn over_delegation_is_denied() {
        // Root allows only one hop, but grant attempts to give the child one more
        // delegation hop instead of zero.
        let grants = [grant(1, 2, 1, 10)];
        let result = evaluate_issuer_authority(&request(2), &[anchor(1, 1)], &grants).unwrap();
        assert_eq!(result.decision, IssuerTrustDecision::Denied);
        assert_eq!(
            result.denials,
            vec![IssuerTrustDenial::DelegationDepthExceeded]
        );
    }

    #[test]
    fn cycle_does_not_bootstrap_untrusted_issuer() {
        let grants = [grant(7, 8, 1, 10), grant(8, 7, 0, 20)];
        let result = evaluate_issuer_authority(&request(7), &[anchor(1, 2)], &grants).unwrap();
        assert_eq!(result.decision, IssuerTrustDecision::Denied);
        assert_eq!(result.denials, vec![IssuerTrustDenial::NoValidTrustPath]);
    }

    #[test]
    fn revoked_delegation_is_not_a_trust_path() {
        let revoked = VerifiedDelegationGrant::from_verified_record(
            principal(1),
            principal(2),
            vec![CredentialKind::PractitionerLicense],
            vec![jurisdiction("US-TX")],
            0,
            Some(1_000),
            Some(50),
            0,
            digest(10),
            digest(11),
        )
        .unwrap();
        let result = evaluate_issuer_authority(&request(2), &[anchor(1, 1)], &[revoked]).unwrap();
        assert_eq!(result.decision, IssuerTrustDecision::Denied);
    }

    #[test]
    fn wrong_jurisdiction_does_not_transfer_authority() {
        let ca_grant = VerifiedDelegationGrant::from_verified_record(
            principal(1),
            principal(2),
            vec![CredentialKind::PractitionerLicense],
            vec![jurisdiction("US-CA")],
            0,
            Some(1_000),
            None,
            0,
            digest(10),
            digest(11),
        )
        .unwrap();
        let result = evaluate_issuer_authority(&request(2), &[anchor(1, 1)], &[ca_grant]).unwrap();
        assert_eq!(result.decision, IssuerTrustDecision::Denied);
    }

    #[test]
    fn wrong_credential_kind_does_not_transfer_authority() {
        let grant = VerifiedDelegationGrant::from_verified_record(
            principal(1),
            principal(2),
            vec![CredentialKind::ResearchEthicsAppointment],
            vec![jurisdiction("US-TX")],
            0,
            Some(1_000),
            None,
            0,
            digest(10),
            digest(11),
        )
        .unwrap();
        let result = evaluate_issuer_authority(&request(2), &[anchor(1, 1)], &[grant]).unwrap();
        assert_eq!(result.decision, IssuerTrustDecision::Denied);
    }

    #[test]
    fn expired_anchor_is_not_a_root() {
        let expired = TrustAnchor::new(
            principal(1),
            vec![CredentialKind::PractitionerLicense],
            vec![jurisdiction("US-TX")],
            0,
            Some(50),
            1,
            digest(90),
            digest(91),
        )
        .unwrap();
        let result = evaluate_issuer_authority(&request(1), &[expired], &[]).unwrap();
        assert_eq!(result.decision, IssuerTrustDecision::Denied);
    }

    #[test]
    fn policy_digest_is_an_exact_trust_root_boundary() {
        let mut req = request(1);
        req.policy_digest = digest(99);
        let result = evaluate_issuer_authority(&req, &[anchor(1, 1)], &[]).unwrap();
        assert_eq!(result.decision, IssuerTrustDecision::Denied);
    }
}
