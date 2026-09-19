#![deny(unsafe_code)]
//! Typed practitioner authority policy for Mycelix-Health.
//!
//! This crate deliberately separates three questions:
//! 1. What does a credential claim?
//! 2. Has an adapter actually resolved/verified that credential, its issuer trust,
//!    and revocation status?
//! 3. Does that verified evidence satisfy the exact policy for one target action?
//!
//! The output permit is intentionally non-serializable and non-cloneable. It is
//! bound to one principal, one target artifact, one policy, one jurisdiction, and
//! one authority purpose.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use thiserror::Error;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CodeRef {
    pub system: String,
    pub code: String,
}

impl CodeRef {
    pub fn validate(&self) -> Result<(), AuthorityError> {
        validate_token("code.system", &self.system, 256)?;
        validate_token("code.code", &self.code, 128)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct JurisdictionCode {
    /// Coding system or namespace, e.g. ISO-3166-2 or an organization policy URI.
    pub system: String,
    pub code: String,
}

impl JurisdictionCode {
    pub fn validate(&self) -> Result<(), AuthorityError> {
        validate_token("jurisdiction.system", &self.system, 256)?;
        validate_token("jurisdiction.code", &self.code, 128)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ScopeCode {
    /// Scope namespace. This may later map to a standards-based code system or a
    /// versioned Mycelix policy vocabulary.
    pub system: String,
    pub code: String,
}

impl ScopeCode {
    pub fn validate(&self) -> Result<(), AuthorityError> {
        validate_token("scope.system", &self.system, 256)?;
        validate_token("scope.code", &self.code, 128)
    }
}

fn validate_token(field: &'static str, value: &str, max_len: usize) -> Result<(), AuthorityError> {
    if value.trim().is_empty() {
        return Err(AuthorityError::MissingField(field));
    }
    if value.len() > max_len {
        return Err(AuthorityError::FieldTooLong(field));
    }
    Ok(())
}

/// Strict typed claims schema for a practitioner-license credential.
///
/// Deliberately omits names, license numbers, addresses, and other identifiers not
/// needed for authority policy evaluation. Those may remain in an appropriately
/// protected source credential but are not needed to decide scope/jurisdiction.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PractitionerLicenseClaimsV1 {
    pub schema_version: u16,
    pub license_class: CodeRef,
    pub jurisdictions: Vec<JurisdictionCode>,
    pub scopes: Vec<ScopeCode>,
}

impl PractitionerLicenseClaimsV1 {
    pub fn validate(&self) -> Result<(), AuthorityError> {
        if self.schema_version != 1 {
            return Err(AuthorityError::UnsupportedClaimsSchemaVersion(
                self.schema_version,
            ));
        }
        self.license_class.validate()?;
        validate_nonempty_unique_jurisdictions(&self.jurisdictions)?;
        validate_nonempty_unique_scopes(&self.scopes)?;
        Ok(())
    }
}

pub fn parse_practitioner_license_claims(
    json: &str,
) -> Result<PractitionerLicenseClaimsV1, AuthorityError> {
    if json.trim().is_empty() {
        return Err(AuthorityError::EmptyClaims);
    }
    let claims: PractitionerLicenseClaimsV1 =
        serde_json::from_str(json).map_err(|_| AuthorityError::InvalidClaimsJson)?;
    claims.validate()?;
    Ok(claims)
}

fn validate_nonempty_unique_jurisdictions(
    values: &[JurisdictionCode],
) -> Result<(), AuthorityError> {
    if values.is_empty() {
        return Err(AuthorityError::MissingJurisdictionClaims);
    }
    let mut seen = HashSet::new();
    for value in values {
        value.validate()?;
        if !seen.insert((value.system.clone(), value.code.clone())) {
            return Err(AuthorityError::DuplicateJurisdictionClaim);
        }
    }
    Ok(())
}

fn validate_nonempty_unique_scopes(values: &[ScopeCode]) -> Result<(), AuthorityError> {
    if values.is_empty() {
        return Err(AuthorityError::MissingScopeClaims);
    }
    let mut seen = HashSet::new();
    for value in values {
        value.validate()?;
        if !seen.insert((value.system.clone(), value.code.clone())) {
            return Err(AuthorityError::DuplicateScopeClaim);
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrincipalBinding(pub [u8; 32]);

impl PrincipalBinding {
    fn validate(self) -> Result<(), AuthorityError> {
        if self.0 == [0u8; 32] {
            return Err(AuthorityError::ZeroPrincipalBinding);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EvidenceDigest(pub [u8; 32]);

impl EvidenceDigest {
    fn validate(self) -> Result<(), AuthorityError> {
        if self.0 == [0u8; 32] {
            return Err(AuthorityError::ZeroEvidenceDigest);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CredentialKind {
    PractitionerLicense,
    ControlledSubstanceRegistration,
    OrganizationDelegation,
    SpecialtyCertification,
    ResearchEthicsAppointment,
}

/// Evidence that has already crossed the credential-resolution trust boundary.
///
/// This type intentionally has no serde implementation. It should be constructed
/// only by a trusted adapter after checking the source credential record, holder
/// binding, issuer authority, expiration, and revocation evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedCredentialEvidence {
    credential_kind: CredentialKind,
    principal: PrincipalBinding,
    jurisdictions: Vec<JurisdictionCode>,
    scopes: Vec<ScopeCode>,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    revoked_at_micros: Option<i64>,
    credential_record_digest: EvidenceDigest,
    issuer_authority_evidence_digest: EvidenceDigest,
    status_check_evidence_digest: EvidenceDigest,
}

impl ResolvedCredentialEvidence {
    /// Construct from a credential adapter that has already verified the source
    /// record and issuer/revocation evidence. This constructor validates shape and
    /// temporal consistency but cannot itself prove external issuer legitimacy;
    /// that proof is represented by `issuer_authority_evidence_digest` and must be
    /// produced by the adapter's trusted registry/policy path.
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_record(
        credential_kind: CredentialKind,
        principal: PrincipalBinding,
        jurisdictions: Vec<JurisdictionCode>,
        scopes: Vec<ScopeCode>,
        valid_from_micros: i64,
        valid_until_micros: Option<i64>,
        revoked_at_micros: Option<i64>,
        credential_record_digest: EvidenceDigest,
        issuer_authority_evidence_digest: EvidenceDigest,
        status_check_evidence_digest: EvidenceDigest,
    ) -> Result<Self, AuthorityError> {
        principal.validate()?;
        validate_nonempty_unique_jurisdictions(&jurisdictions)?;
        validate_nonempty_unique_scopes(&scopes)?;
        credential_record_digest.validate()?;
        issuer_authority_evidence_digest.validate()?;
        status_check_evidence_digest.validate()?;
        if let Some(valid_until) = valid_until_micros {
            if valid_until <= valid_from_micros {
                return Err(AuthorityError::InvalidCredentialValidityWindow);
            }
        }
        if let Some(revoked_at) = revoked_at_micros {
            if revoked_at < valid_from_micros {
                return Err(AuthorityError::RevocationPredatesValidity);
            }
        }

        Ok(Self {
            credential_kind,
            principal,
            jurisdictions,
            scopes,
            valid_from_micros,
            valid_until_micros,
            revoked_at_micros,
            credential_record_digest,
            issuer_authority_evidence_digest,
            status_check_evidence_digest,
        })
    }

    pub fn credential_kind(&self) -> CredentialKind {
        self.credential_kind
    }

    pub fn principal(&self) -> PrincipalBinding {
        self.principal
    }

    fn supports_jurisdiction(&self, required: &JurisdictionCode) -> bool {
        self.jurisdictions.iter().any(|value| value == required)
    }

    fn supports_scope(&self, required: &ScopeCode) -> bool {
        self.scopes.iter().any(|value| value == required)
    }

    fn is_active_at(&self, at_micros: i64) -> CredentialTemporalState {
        if at_micros < self.valid_from_micros {
            return CredentialTemporalState::NotYetValid;
        }
        if let Some(revoked_at) = self.revoked_at_micros {
            if at_micros >= revoked_at {
                return CredentialTemporalState::Revoked;
            }
        }
        if let Some(valid_until) = self.valid_until_micros {
            if at_micros >= valid_until {
                return CredentialTemporalState::Expired;
            }
        }
        CredentialTemporalState::Active
    }

    fn supporting_digests(&self) -> [EvidenceDigest; 3] {
        [
            self.credential_record_digest,
            self.issuer_authority_evidence_digest,
            self.status_check_evidence_digest,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CredentialTemporalState {
    Active,
    NotYetValid,
    Expired,
    Revoked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityPurpose {
    ClinicalReview,
    Prescribe,
    Dispense,
    Administer,
    TrialEligibilityReview,
    ResearchEthicsReview,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityRequest {
    pub principal: PrincipalBinding,
    pub target_artifact_digest: EvidenceDigest,
    pub policy_digest: EvidenceDigest,
    pub purpose: AuthorityPurpose,
    pub jurisdiction: JurisdictionCode,
    pub required_credential_kinds: Vec<CredentialKind>,
    pub required_scopes: Vec<ScopeCode>,
    pub evaluated_at_micros: i64,
}

impl AuthorityRequest {
    pub fn validate(&self) -> Result<(), AuthorityError> {
        self.principal.validate()?;
        self.target_artifact_digest.validate()?;
        self.policy_digest.validate()?;
        self.jurisdiction.validate()?;
        if self.required_credential_kinds.is_empty() {
            return Err(AuthorityError::MissingCredentialRequirements);
        }
        if self.required_scopes.is_empty() {
            return Err(AuthorityError::MissingScopeRequirements);
        }

        let mut kinds = HashSet::new();
        if self
            .required_credential_kinds
            .iter()
            .any(|kind| !kinds.insert(*kind))
        {
            return Err(AuthorityError::DuplicateCredentialRequirement);
        }

        let mut scopes = HashSet::new();
        for scope in &self.required_scopes {
            scope.validate()?;
            if !scopes.insert((scope.system.clone(), scope.code.clone())) {
                return Err(AuthorityError::DuplicateScopeRequirement);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthorityDenial {
    PrincipalMismatch,
    CredentialNotYetValid(CredentialKind),
    CredentialExpired(CredentialKind),
    CredentialRevoked(CredentialKind),
    JurisdictionNotCovered(CredentialKind),
    MissingRequiredCredential(CredentialKind),
    MissingRequiredScope(ScopeCode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityDecision {
    Granted,
    Denied,
}

pub struct AuthorityEvaluation {
    pub decision: AuthorityDecision,
    pub denials: Vec<AuthorityDenial>,
    permit: Option<AuthorityPermit>,
}

impl AuthorityEvaluation {
    /// Consume the evaluation and obtain the non-cloneable permit if granted.
    pub fn into_permit(self) -> Option<AuthorityPermit> {
        self.permit
    }
}

/// Opaque, single-owner authority capability for one exact target/policy decision.
///
/// No `Clone`, `Copy`, `Serialize`, or `Deserialize` implementation is provided.
/// Downstream APIs can consume this value by ownership when executing the one
/// authorized workflow step.
pub struct AuthorityPermit {
    principal: PrincipalBinding,
    target_artifact_digest: EvidenceDigest,
    policy_digest: EvidenceDigest,
    purpose: AuthorityPurpose,
    jurisdiction: JurisdictionCode,
    evaluated_at_micros: i64,
    supporting_evidence_digests: Vec<EvidenceDigest>,
}

impl AuthorityPermit {
    pub fn principal(&self) -> PrincipalBinding {
        self.principal
    }

    pub fn target_artifact_digest(&self) -> EvidenceDigest {
        self.target_artifact_digest
    }

    pub fn policy_digest(&self) -> EvidenceDigest {
        self.policy_digest
    }

    pub fn purpose(&self) -> AuthorityPurpose {
        self.purpose
    }

    pub fn jurisdiction(&self) -> &JurisdictionCode {
        &self.jurisdiction
    }

    pub fn evaluated_at_micros(&self) -> i64 {
        self.evaluated_at_micros
    }

    pub fn supporting_evidence_digests(&self) -> &[EvidenceDigest] {
        &self.supporting_evidence_digests
    }
}

pub fn evaluate_authority(
    request: &AuthorityRequest,
    credentials: &[ResolvedCredentialEvidence],
) -> Result<AuthorityEvaluation, AuthorityError> {
    request.validate()?;

    let principal_credentials: Vec<&ResolvedCredentialEvidence> = credentials
        .iter()
        .filter(|credential| credential.principal == request.principal)
        .collect();

    if principal_credentials.is_empty() && !credentials.is_empty() {
        return Ok(denied(vec![AuthorityDenial::PrincipalMismatch]));
    }

    let mut denials = Vec::new();
    let mut qualifying = Vec::new();

    for required_kind in &request.required_credential_kinds {
        let matching_kind: Vec<&ResolvedCredentialEvidence> = principal_credentials
            .iter()
            .copied()
            .filter(|credential| credential.credential_kind == *required_kind)
            .collect();

        if matching_kind.is_empty() {
            denials.push(AuthorityDenial::MissingRequiredCredential(*required_kind));
            continue;
        }

        let mut kind_has_active = false;
        let mut kind_has_jurisdiction = false;

        for credential in matching_kind {
            match credential.is_active_at(request.evaluated_at_micros) {
                CredentialTemporalState::Active => {
                    kind_has_active = true;
                    if credential.supports_jurisdiction(&request.jurisdiction) {
                        kind_has_jurisdiction = true;
                        qualifying.push(credential);
                    }
                }
                CredentialTemporalState::NotYetValid => {
                    denials.push(AuthorityDenial::CredentialNotYetValid(*required_kind));
                }
                CredentialTemporalState::Expired => {
                    denials.push(AuthorityDenial::CredentialExpired(*required_kind));
                }
                CredentialTemporalState::Revoked => {
                    denials.push(AuthorityDenial::CredentialRevoked(*required_kind));
                }
            }
        }

        if !kind_has_active {
            if !denials.iter().any(|denial| {
                matches!(
                    denial,
                    AuthorityDenial::CredentialNotYetValid(kind)
                        | AuthorityDenial::CredentialExpired(kind)
                        | AuthorityDenial::CredentialRevoked(kind)
                        if kind == required_kind
                )
            }) {
                denials.push(AuthorityDenial::MissingRequiredCredential(*required_kind));
            }
        } else if !kind_has_jurisdiction {
            denials.push(AuthorityDenial::JurisdictionNotCovered(*required_kind));
        }
    }

    if denials.is_empty() {
        for required_scope in &request.required_scopes {
            if !qualifying
                .iter()
                .any(|credential| credential.supports_scope(required_scope))
            {
                denials.push(AuthorityDenial::MissingRequiredScope(required_scope.clone()));
            }
        }
    }

    dedupe_denials(&mut denials);
    if !denials.is_empty() {
        return Ok(denied(denials));
    }

    let mut evidence = BTreeSet::new();
    for credential in qualifying {
        for digest in credential.supporting_digests() {
            evidence.insert(digest);
        }
    }

    Ok(AuthorityEvaluation {
        decision: AuthorityDecision::Granted,
        denials: Vec::new(),
        permit: Some(AuthorityPermit {
            principal: request.principal,
            target_artifact_digest: request.target_artifact_digest,
            policy_digest: request.policy_digest,
            purpose: request.purpose,
            jurisdiction: request.jurisdiction.clone(),
            evaluated_at_micros: request.evaluated_at_micros,
            supporting_evidence_digests: evidence.into_iter().collect(),
        }),
    })
}

fn denied(denials: Vec<AuthorityDenial>) -> AuthorityEvaluation {
    AuthorityEvaluation {
        decision: AuthorityDecision::Denied,
        denials,
        permit: None,
    }
}

fn dedupe_denials(denials: &mut Vec<AuthorityDenial>) {
    let mut seen = HashSet::new();
    denials.retain(|denial| {
        let key = format!("{denial:?}");
        seen.insert(key)
    });
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthorityError {
    #[error("credential claims are empty")]
    EmptyClaims,
    #[error("credential claims are not valid strict v1 JSON")]
    InvalidClaimsJson,
    #[error("unsupported practitioner claims schema version {0}")]
    UnsupportedClaimsSchemaVersion(u16),
    #[error("required field is empty: {0}")]
    MissingField(&'static str),
    #[error("field exceeds maximum length: {0}")]
    FieldTooLong(&'static str),
    #[error("practitioner claims require at least one jurisdiction")]
    MissingJurisdictionClaims,
    #[error("practitioner claims contain duplicate jurisdiction")]
    DuplicateJurisdictionClaim,
    #[error("practitioner claims require at least one scope")]
    MissingScopeClaims,
    #[error("practitioner claims contain duplicate scope")]
    DuplicateScopeClaim,
    #[error("principal binding must be non-zero")]
    ZeroPrincipalBinding,
    #[error("evidence digest must be non-zero")]
    ZeroEvidenceDigest,
    #[error("credential validity end must be after validity start")]
    InvalidCredentialValidityWindow,
    #[error("credential revocation cannot predate its validity start")]
    RevocationPredatesValidity,
    #[error("authority request requires at least one credential kind")]
    MissingCredentialRequirements,
    #[error("authority request contains duplicate credential kind")]
    DuplicateCredentialRequirement,
    #[error("authority request requires at least one scope")]
    MissingScopeRequirements,
    #[error("authority request contains duplicate scope")]
    DuplicateScopeRequirement,
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

    fn scope(code: &str) -> ScopeCode {
        ScopeCode {
            system: "https://mycelix.health/authority-scope/v1".into(),
            code: code.into(),
        }
    }

    fn credential(
        kind: CredentialKind,
        principal_binding: PrincipalBinding,
        jurisdictions: Vec<JurisdictionCode>,
        scopes: Vec<ScopeCode>,
        valid_from: i64,
        valid_until: Option<i64>,
        revoked_at: Option<i64>,
        seed: u8,
    ) -> ResolvedCredentialEvidence {
        ResolvedCredentialEvidence::from_verified_record(
            kind,
            principal_binding,
            jurisdictions,
            scopes,
            valid_from,
            valid_until,
            revoked_at,
            digest(seed),
            digest(seed.wrapping_add(1)),
            digest(seed.wrapping_add(2)),
        )
        .unwrap()
    }

    fn request(kinds: Vec<CredentialKind>, scopes: Vec<ScopeCode>) -> AuthorityRequest {
        AuthorityRequest {
            principal: principal(1),
            target_artifact_digest: digest(40),
            policy_digest: digest(41),
            purpose: AuthorityPurpose::ClinicalReview,
            jurisdiction: jurisdiction("US-TX"),
            required_credential_kinds: kinds,
            required_scopes: scopes,
            evaluated_at_micros: 100,
        }
    }

    #[test]
    fn strict_practitioner_claims_parse() {
        let claims = parse_practitioner_license_claims(
            r#"{
                "schema_version": 1,
                "license_class": {"system":"urn:example:license","code":"physician"},
                "jurisdictions": [{"system":"urn:iso:std:iso:3166-2","code":"US-TX"}],
                "scopes": [{"system":"https://mycelix.health/authority-scope/v1","code":"clinical-review"}]
            }"#,
        )
        .unwrap();
        assert_eq!(claims.schema_version, 1);
    }

    #[test]
    fn unknown_claim_fields_fail_closed() {
        let error = parse_practitioner_license_claims(
            r#"{
                "schema_version": 1,
                "license_class": {"system":"urn:example:license","code":"physician"},
                "jurisdictions": [{"system":"urn:iso:std:iso:3166-2","code":"US-TX"}],
                "scopes": [{"system":"https://mycelix.health/authority-scope/v1","code":"clinical-review"}],
                "magic_admin": true
            }"#,
        )
        .unwrap_err();
        assert_eq!(error, AuthorityError::InvalidClaimsJson);
    }

    #[test]
    fn active_license_with_exact_scope_and_jurisdiction_grants() {
        let license = credential(
            CredentialKind::PractitionerLicense,
            principal(1),
            vec![jurisdiction("US-TX")],
            vec![scope("clinical-review")],
            0,
            Some(1_000),
            None,
            10,
        );
        let result = evaluate_authority(
            &request(
                vec![CredentialKind::PractitionerLicense],
                vec![scope("clinical-review")],
            ),
            &[license],
        )
        .unwrap();
        assert_eq!(result.decision, AuthorityDecision::Granted);
        let permit = result.into_permit().expect("granted request needs permit");
        assert_eq!(permit.target_artifact_digest(), digest(40));
        assert_eq!(permit.policy_digest(), digest(41));
    }

    #[test]
    fn wrong_principal_cannot_borrow_someone_elses_license() {
        let license = credential(
            CredentialKind::PractitionerLicense,
            principal(2),
            vec![jurisdiction("US-TX")],
            vec![scope("clinical-review")],
            0,
            Some(1_000),
            None,
            10,
        );
        let result = evaluate_authority(
            &request(
                vec![CredentialKind::PractitionerLicense],
                vec![scope("clinical-review")],
            ),
            &[license],
        )
        .unwrap();
        assert_eq!(result.decision, AuthorityDecision::Denied);
        assert_eq!(result.denials, vec![AuthorityDenial::PrincipalMismatch]);
    }

    #[test]
    fn revoked_license_is_denied() {
        let license = credential(
            CredentialKind::PractitionerLicense,
            principal(1),
            vec![jurisdiction("US-TX")],
            vec![scope("clinical-review")],
            0,
            Some(1_000),
            Some(50),
            10,
        );
        let result = evaluate_authority(
            &request(
                vec![CredentialKind::PractitionerLicense],
                vec![scope("clinical-review")],
            ),
            &[license],
        )
        .unwrap();
        assert!(result
            .denials
            .contains(&AuthorityDenial::CredentialRevoked(
                CredentialKind::PractitionerLicense
            )));
    }

    #[test]
    fn expired_license_is_denied() {
        let license = credential(
            CredentialKind::PractitionerLicense,
            principal(1),
            vec![jurisdiction("US-TX")],
            vec![scope("clinical-review")],
            0,
            Some(100),
            None,
            10,
        );
        let result = evaluate_authority(
            &request(
                vec![CredentialKind::PractitionerLicense],
                vec![scope("clinical-review")],
            ),
            &[license],
        )
        .unwrap();
        assert!(result
            .denials
            .contains(&AuthorityDenial::CredentialExpired(
                CredentialKind::PractitionerLicense
            )));
    }

    #[test]
    fn wrong_jurisdiction_is_denied() {
        let license = credential(
            CredentialKind::PractitionerLicense,
            principal(1),
            vec![jurisdiction("US-CA")],
            vec![scope("clinical-review")],
            0,
            Some(1_000),
            None,
            10,
        );
        let result = evaluate_authority(
            &request(
                vec![CredentialKind::PractitionerLicense],
                vec![scope("clinical-review")],
            ),
            &[license],
        )
        .unwrap();
        assert!(result
            .denials
            .contains(&AuthorityDenial::JurisdictionNotCovered(
                CredentialKind::PractitionerLicense
            )));
    }

    #[test]
    fn missing_scope_is_denied() {
        let license = credential(
            CredentialKind::PractitionerLicense,
            principal(1),
            vec![jurisdiction("US-TX")],
            vec![scope("clinical-review")],
            0,
            Some(1_000),
            None,
            10,
        );
        let result = evaluate_authority(
            &request(
                vec![CredentialKind::PractitionerLicense],
                vec![scope("prescribe-non-controlled")],
            ),
            &[license],
        )
        .unwrap();
        assert!(result.denials.contains(&AuthorityDenial::MissingRequiredScope(
            scope("prescribe-non-controlled")
        )));
    }

    #[test]
    fn controlled_prescribing_can_require_two_independent_credentials() {
        let license = credential(
            CredentialKind::PractitionerLicense,
            principal(1),
            vec![jurisdiction("US-TX")],
            vec![scope("prescribe")],
            0,
            Some(1_000),
            None,
            10,
        );
        let controlled = credential(
            CredentialKind::ControlledSubstanceRegistration,
            principal(1),
            vec![jurisdiction("US-TX")],
            vec![scope("controlled-schedule-ii")],
            0,
            Some(500),
            None,
            20,
        );
        let req = request(
            vec![
                CredentialKind::PractitionerLicense,
                CredentialKind::ControlledSubstanceRegistration,
            ],
            vec![scope("prescribe"), scope("controlled-schedule-ii")],
        );
        let result = evaluate_authority(&req, &[license, controlled]).unwrap();
        assert_eq!(result.decision, AuthorityDecision::Granted);
    }

    #[test]
    fn missing_controlled_registration_fails_closed() {
        let license = credential(
            CredentialKind::PractitionerLicense,
            principal(1),
            vec![jurisdiction("US-TX")],
            vec![scope("prescribe")],
            0,
            Some(1_000),
            None,
            10,
        );
        let req = request(
            vec![
                CredentialKind::PractitionerLicense,
                CredentialKind::ControlledSubstanceRegistration,
            ],
            vec![scope("prescribe"), scope("controlled-schedule-ii")],
        );
        let result = evaluate_authority(&req, &[license]).unwrap();
        assert!(result.denials.contains(&AuthorityDenial::MissingRequiredCredential(
            CredentialKind::ControlledSubstanceRegistration
        )));
    }

    #[test]
    fn resolved_credential_requires_issuer_and_status_evidence() {
        let error = ResolvedCredentialEvidence::from_verified_record(
            CredentialKind::PractitionerLicense,
            principal(1),
            vec![jurisdiction("US-TX")],
            vec![scope("clinical-review")],
            0,
            Some(1_000),
            None,
            digest(10),
            EvidenceDigest([0u8; 32]),
            digest(12),
        )
        .unwrap_err();
        assert_eq!(error, AuthorityError::ZeroEvidenceDigest);
    }
}
