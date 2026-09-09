#![deny(unsafe_code)]
//! Evidence-bearing practitioner identity resolution for Mycelix-Health.
//!
//! FHIR references and identifiers are external identity claims. They are never
//! treated as equivalent to an authenticated Mycelix principal merely because a
//! string looks plausible. Resolution succeeds only through previously verified
//! provider/principal binding evidence, and ambiguity fails closed.

use mycelix_clinical_authority::PrincipalBinding;
use mycelix_clinical_integrity::{DigestDomain, StoredDigest, VerifiedDigest};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

const MAX_TOKEN_LEN: usize = 512;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct ExternalIdentifier {
    pub system: String,
    pub value: String,
}

impl ExternalIdentifier {
    pub fn validate(&self) -> Result<(), PrincipalResolutionError> {
        validate_token("identifier.system", &self.system)?;
        validate_token("identifier.value", &self.value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct ExternalResourceKey {
    pub resource_type: String,
    pub id: String,
}

impl ExternalResourceKey {
    pub fn validate(&self) -> Result<(), PrincipalResolutionError> {
        if !matches!(self.resource_type.as_str(), "Practitioner" | "PractitionerRole") {
            return Err(PrincipalResolutionError::UnsupportedResourceType(
                self.resource_type.clone(),
            ));
        }
        validate_token("resource.id", &self.id)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct ExternalReferenceBinding {
    /// Exact FHIR server/base namespace in which this reference was verified.
    pub source_system: String,
    pub resource: ExternalResourceKey,
}

impl ExternalReferenceBinding {
    fn validate(&self) -> Result<(), PrincipalResolutionError> {
        validate_token("reference.source_system", &self.source_system)?;
        self.resource.validate()
    }
}

/// Strict external practitioner reference input. Display names are deliberately
/// absent because names/display text are not identity evidence.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PractitionerReferenceInput {
    pub source_system: String,
    pub reference: Option<String>,
    pub type_name: Option<String>,
    pub identifier: Option<ExternalIdentifier>,
}

impl PractitionerReferenceInput {
    pub fn validate(&self) -> Result<(), PrincipalResolutionError> {
        validate_token("source_system", &self.source_system)?;
        if self.reference.is_none() && self.identifier.is_none() {
            return Err(PrincipalResolutionError::MissingResolvableIdentity);
        }
        if let Some(identifier) = &self.identifier {
            identifier.validate()?;
        }
        if let Some(type_name) = &self.type_name {
            if !matches!(type_name.as_str(), "Practitioner" | "PractitionerRole") {
                return Err(PrincipalResolutionError::UnsupportedResourceType(
                    type_name.clone(),
                ));
            }
        }
        if let Some(reference) = &self.reference {
            parse_reference(reference, &self.source_system, self.type_name.as_deref())?;
        }
        Ok(())
    }
}

/// Evidence already verified by a trusted provider-binding adapter.
///
/// This type intentionally has no serde implementation. The adapter must verify
/// provider-record authorship/principal binding and status/revocation evidence
/// before constructing it.
pub struct VerifiedProviderPrincipalBinding {
    principal: PrincipalBinding,
    reference_bindings: Vec<ExternalReferenceBinding>,
    identifiers: Vec<ExternalIdentifier>,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    revoked_at_micros: Option<i64>,
    provider_record_digest: VerifiedDigest,
    author_binding_evidence_digest: VerifiedDigest,
    status_evidence_digest: VerifiedDigest,
}

impl VerifiedProviderPrincipalBinding {
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_provider_record(
        principal: PrincipalBinding,
        reference_bindings: Vec<ExternalReferenceBinding>,
        identifiers: Vec<ExternalIdentifier>,
        valid_from_micros: i64,
        valid_until_micros: Option<i64>,
        revoked_at_micros: Option<i64>,
        provider_record_digest: VerifiedDigest,
        author_binding_evidence_digest: VerifiedDigest,
        status_evidence_digest: VerifiedDigest,
    ) -> Result<Self, PrincipalResolutionError> {
        if principal.0 == [0u8; 32] {
            return Err(PrincipalResolutionError::ZeroPrincipalBinding);
        }
        if reference_bindings.is_empty() && identifiers.is_empty() {
            return Err(PrincipalResolutionError::BindingHasNoExternalIdentity);
        }

        let mut seen_refs = HashSet::new();
        for binding in &reference_bindings {
            binding.validate()?;
            if !seen_refs.insert((
                binding.source_system.clone(),
                binding.resource.resource_type.clone(),
                binding.resource.id.clone(),
            )) {
                return Err(PrincipalResolutionError::DuplicateReferenceBinding);
            }
        }

        let mut seen_ids = HashSet::new();
        for identifier in &identifiers {
            identifier.validate()?;
            if !seen_ids.insert((identifier.system.clone(), identifier.value.clone())) {
                return Err(PrincipalResolutionError::DuplicateIdentifierBinding);
            }
        }

        provider_record_digest.require_domain(DigestDomain::ClinicalArtifact)?;
        author_binding_evidence_digest.require_domain(DigestDomain::ClinicalArtifact)?;
        status_evidence_digest.require_domain(DigestDomain::ClinicalArtifact)?;

        if let Some(valid_until) = valid_until_micros {
            if valid_until <= valid_from_micros {
                return Err(PrincipalResolutionError::InvalidValidityWindow);
            }
        }
        if let Some(revoked_at) = revoked_at_micros {
            if revoked_at < valid_from_micros {
                return Err(PrincipalResolutionError::RevocationPredatesValidity);
            }
        }

        Ok(Self {
            principal,
            reference_bindings,
            identifiers,
            valid_from_micros,
            valid_until_micros,
            revoked_at_micros,
            provider_record_digest,
            author_binding_evidence_digest,
            status_evidence_digest,
        })
    }

    pub fn principal(&self) -> PrincipalBinding {
        self.principal
    }

    fn is_active_at(&self, at_micros: i64) -> bool {
        if at_micros < self.valid_from_micros {
            return false;
        }
        if let Some(revoked_at) = self.revoked_at_micros {
            if at_micros >= revoked_at {
                return false;
            }
        }
        if let Some(valid_until) = self.valid_until_micros {
            if at_micros >= valid_until {
                return false;
            }
        }
        true
    }

    fn matches_reference(&self, source_system: &str, key: &ExternalResourceKey) -> bool {
        self.reference_bindings.iter().any(|binding| {
            binding.source_system == source_system && binding.resource == *key
        })
    }

    fn matches_identifier(&self, identifier: &ExternalIdentifier) -> bool {
        self.identifiers.iter().any(|candidate| candidate == identifier)
    }
}

/// Opaque resolution result. It is evidence that one external practitioner claim
/// resolved unambiguously to one Mycelix principal at one point in time.
pub struct ResolvedPractitionerPrincipal {
    principal: PrincipalBinding,
    provider_record_digest: StoredDigest,
    author_binding_evidence_digest: StoredDigest,
    status_evidence_digest: StoredDigest,
    matched_reference: Option<ExternalResourceKey>,
    matched_identifier: Option<ExternalIdentifier>,
    resolved_at_micros: i64,
}

impl ResolvedPractitionerPrincipal {
    pub fn principal(&self) -> PrincipalBinding {
        self.principal
    }

    pub fn provider_record_digest(&self) -> StoredDigest {
        self.provider_record_digest
    }

    pub fn author_binding_evidence_digest(&self) -> StoredDigest {
        self.author_binding_evidence_digest
    }

    pub fn status_evidence_digest(&self) -> StoredDigest {
        self.status_evidence_digest
    }

    pub fn matched_reference(&self) -> Option<&ExternalResourceKey> {
        self.matched_reference.as_ref()
    }

    pub fn matched_identifier(&self) -> Option<&ExternalIdentifier> {
        self.matched_identifier.as_ref()
    }

    pub fn resolved_at_micros(&self) -> i64 {
        self.resolved_at_micros
    }
}

pub fn resolve_practitioner_principal(
    input: &PractitionerReferenceInput,
    candidates: &[VerifiedProviderPrincipalBinding],
    at_micros: i64,
) -> Result<ResolvedPractitionerPrincipal, PrincipalResolutionError> {
    input.validate()?;

    let parsed_reference = match input.reference.as_deref() {
        Some(reference) => Some(parse_reference(
            reference,
            &input.source_system,
            input.type_name.as_deref(),
        )?),
        None => None,
    };

    let mut matches = Vec::new();
    for candidate in candidates {
        if !candidate.is_active_at(at_micros) {
            continue;
        }

        let reference_matches = parsed_reference
            .as_ref()
            .is_none_or(|key| candidate.matches_reference(&input.source_system, key));
        let identifier_matches = input
            .identifier
            .as_ref()
            .is_none_or(|identifier| candidate.matches_identifier(identifier));

        if reference_matches && identifier_matches {
            matches.push(candidate);
        }
    }

    match matches.as_slice() {
        [] => Err(PrincipalResolutionError::UnresolvedPractitioner),
        [candidate] => Ok(ResolvedPractitionerPrincipal {
            principal: candidate.principal,
            provider_record_digest: candidate.provider_record_digest.stored(),
            author_binding_evidence_digest: candidate.author_binding_evidence_digest.stored(),
            status_evidence_digest: candidate.status_evidence_digest.stored(),
            matched_reference: parsed_reference,
            matched_identifier: input.identifier.clone(),
            resolved_at_micros: at_micros,
        }),
        _ => Err(PrincipalResolutionError::AmbiguousPractitioner),
    }
}

fn parse_reference(
    reference: &str,
    source_system: &str,
    declared_type: Option<&str>,
) -> Result<ExternalResourceKey, PrincipalResolutionError> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err(PrincipalResolutionError::EmptyReference);
    }
    if reference.starts_with("urn:") || reference.starts_with('#') {
        return Err(PrincipalResolutionError::UnsupportedReferenceForm);
    }
    if reference.contains("/_history/") {
        return Err(PrincipalResolutionError::VersionSpecificReference);
    }

    let without_query = reference.split('?').next().unwrap_or(reference);
    let without_fragment = without_query.split('#').next().unwrap_or(without_query);

    if without_fragment.contains("://") {
        let source = source_system.trim_end_matches('/');
        if !without_fragment.starts_with(source)
            || !without_fragment
                .get(source.len()..)
                .is_some_and(|suffix| suffix.starts_with('/'))
        {
            return Err(PrincipalResolutionError::AbsoluteReferenceSourceMismatch);
        }
    }

    let segments: Vec<&str> = without_fragment
        .trim_end_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    let key = segments
        .windows(2)
        .rev()
        .find_map(|pair| {
            if matches!(pair[0], "Practitioner" | "PractitionerRole") {
                Some(ExternalResourceKey {
                    resource_type: pair[0].to_string(),
                    id: pair[1].to_string(),
                })
            } else {
                None
            }
        })
        .ok_or(PrincipalResolutionError::UnsupportedReferenceForm)?;

    key.validate()?;
    if let Some(declared_type) = declared_type {
        if declared_type != key.resource_type {
            return Err(PrincipalResolutionError::ReferenceTypeConflict {
                declared: declared_type.to_string(),
                parsed: key.resource_type.clone(),
            });
        }
    }
    Ok(key)
}

fn validate_token(
    field: &'static str,
    value: &str,
) -> Result<(), PrincipalResolutionError> {
    if value.trim().is_empty() {
        return Err(PrincipalResolutionError::MissingField(field));
    }
    if value.len() > MAX_TOKEN_LEN {
        return Err(PrincipalResolutionError::FieldTooLong(field));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum PrincipalResolutionError {
    #[error("required identity field is empty: {0}")]
    MissingField(&'static str),
    #[error("identity field exceeds maximum length: {0}")]
    FieldTooLong(&'static str),
    #[error("practitioner reference contains neither reference nor identifier")]
    MissingResolvableIdentity,
    #[error("provider binding contains no external reference or identifier")]
    BindingHasNoExternalIdentity,
    #[error("unsupported practitioner resource type: {0}")]
    UnsupportedResourceType(String),
    #[error("practitioner reference is empty")]
    EmptyReference,
    #[error("unsupported FHIR practitioner reference form")]
    UnsupportedReferenceForm,
    #[error("version-specific practitioner references are not accepted by v1")]
    VersionSpecificReference,
    #[error("absolute practitioner reference does not belong to the declared source system")]
    AbsoluteReferenceSourceMismatch,
    #[error("declared FHIR reference type {declared} conflicts with parsed type {parsed}")]
    ReferenceTypeConflict { declared: String, parsed: String },
    #[error("provider principal binding cannot use an all-zero principal")]
    ZeroPrincipalBinding,
    #[error("duplicate external resource binding")]
    DuplicateReferenceBinding,
    #[error("duplicate external identifier binding")]
    DuplicateIdentifierBinding,
    #[error("provider principal binding validity window is invalid")]
    InvalidValidityWindow,
    #[error("provider principal revocation cannot predate validity")]
    RevocationPredatesValidity,
    #[error("no verified active provider binding matches the practitioner reference")]
    UnresolvedPractitioner,
    #[error("multiple verified active provider bindings match the practitioner reference")]
    AmbiguousPractitioner,
    #[error("clinical integrity failure: {0}")]
    Integrity(String),
}

impl From<mycelix_clinical_integrity::IntegrityError> for PrincipalResolutionError {
    fn from(error: mycelix_clinical_integrity::IntegrityError) -> Self {
        Self::Integrity(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelix_clinical_integrity::hash_canonical_bytes;

    fn principal(byte: u8) -> PrincipalBinding {
        PrincipalBinding([byte; 32])
    }

    fn digest(seed: u8) -> VerifiedDigest {
        hash_canonical_bytes(DigestDomain::ClinicalArtifact, &[seed]).unwrap()
    }

    fn npi(value: &str) -> ExternalIdentifier {
        ExternalIdentifier {
            system: "http://hl7.org/fhir/sid/us-npi".into(),
            value: value.into(),
        }
    }

    fn binding(
        p: PrincipalBinding,
        refs: Vec<(&str, &str, &str)>,
        identifiers: Vec<ExternalIdentifier>,
        seed: u8,
    ) -> VerifiedProviderPrincipalBinding {
        VerifiedProviderPrincipalBinding::from_verified_provider_record(
            p,
            refs.into_iter()
                .map(|(source, resource_type, id)| ExternalReferenceBinding {
                    source_system: source.into(),
                    resource: ExternalResourceKey {
                        resource_type: resource_type.into(),
                        id: id.into(),
                    },
                })
                .collect(),
            identifiers,
            0,
            Some(1_000_000),
            None,
            digest(seed),
            digest(seed.wrapping_add(1)),
            digest(seed.wrapping_add(2)),
        )
        .unwrap()
    }

    fn expect_error<T>(result: Result<T, PrincipalResolutionError>) -> PrincipalResolutionError {
        match result {
            Ok(_) => panic!("expected resolution failure"),
            Err(error) => error,
        }
    }

    #[test]
    fn relative_practitioner_reference_resolves_exact_binding() {
        let candidate = binding(
            principal(1),
            vec![("https://ehr.example/fhir", "Practitioner", "123")],
            vec![npi("1234567893")],
            10,
        );
        let input = PractitionerReferenceInput {
            source_system: "https://ehr.example/fhir".into(),
            reference: Some("Practitioner/123".into()),
            type_name: Some("Practitioner".into()),
            identifier: None,
        };
        let resolved = resolve_practitioner_principal(&input, &[candidate], 100).unwrap();
        assert_eq!(resolved.principal(), principal(1));
    }

    #[test]
    fn absolute_reference_from_wrong_fhir_server_is_rejected() {
        let input = PractitionerReferenceInput {
            source_system: "https://ehr-a.example/fhir".into(),
            reference: Some("https://ehr-b.example/fhir/Practitioner/123".into()),
            type_name: Some("Practitioner".into()),
            identifier: None,
        };
        let error = expect_error(resolve_practitioner_principal(&input, &[], 100));
        assert!(matches!(
            error,
            PrincipalResolutionError::AbsoluteReferenceSourceMismatch
        ));
    }

    #[test]
    fn npi_identifier_can_resolve_without_name_or_display_text() {
        let candidate = binding(principal(2), vec![], vec![npi("1234567893")], 20);
        let input = PractitionerReferenceInput {
            source_system: "https://ehr.example/fhir".into(),
            reference: None,
            type_name: None,
            identifier: Some(npi("1234567893")),
        };
        let resolved = resolve_practitioner_principal(&input, &[candidate], 100).unwrap();
        assert_eq!(resolved.principal(), principal(2));
    }

    #[test]
    fn conflicting_reference_and_identifier_fail_closed() {
        let by_reference = binding(
            principal(3),
            vec![("https://ehr.example/fhir", "Practitioner", "123")],
            vec![npi("1111111111")],
            30,
        );
        let by_identifier = binding(
            principal(4),
            vec![("https://ehr.example/fhir", "Practitioner", "456")],
            vec![npi("1234567893")],
            40,
        );
        let input = PractitionerReferenceInput {
            source_system: "https://ehr.example/fhir".into(),
            reference: Some("Practitioner/123".into()),
            type_name: Some("Practitioner".into()),
            identifier: Some(npi("1234567893")),
        };
        let error = expect_error(resolve_practitioner_principal(
            &input,
            &[by_reference, by_identifier],
            100,
        ));
        assert!(matches!(
            error,
            PrincipalResolutionError::UnresolvedPractitioner
        ));
    }

    #[test]
    fn duplicate_active_identifier_bindings_are_ambiguous() {
        let a = binding(principal(5), vec![], vec![npi("1234567893")], 50);
        let b = binding(principal(6), vec![], vec![npi("1234567893")], 60);
        let input = PractitionerReferenceInput {
            source_system: "https://ehr.example/fhir".into(),
            reference: None,
            type_name: None,
            identifier: Some(npi("1234567893")),
        };
        let error = expect_error(resolve_practitioner_principal(&input, &[a, b], 100));
        assert!(matches!(
            error,
            PrincipalResolutionError::AmbiguousPractitioner
        ));
    }

    #[test]
    fn history_reference_is_not_silently_collapsed_to_current_identity() {
        let input = PractitionerReferenceInput {
            source_system: "https://ehr.example/fhir".into(),
            reference: Some("Practitioner/123/_history/7".into()),
            type_name: Some("Practitioner".into()),
            identifier: None,
        };
        let error = expect_error(resolve_practitioner_principal(&input, &[], 100));
        assert!(matches!(
            error,
            PrincipalResolutionError::VersionSpecificReference
        ));
    }
}
