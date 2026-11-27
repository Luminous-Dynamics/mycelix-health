//! Typed privacy-scoped references for clinical transaction proof envelopes.
//!
//! This module validates reference *metadata and scope discipline*. It does not
//! generate random values, compute HMACs, validate SD-JWTs/BBS proofs, verify
//! issuer signatures, or establish unlinkability. Those cryptographic operations
//! must be delegated to reviewed implementations and independently trusted keys.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceScheme {
    /// 256-bit CSPRNG handle generated freshly for one operation.
    TransactionRandomV1,
    /// Domain-separated HMAC-SHA-256 under a tenant/purpose/epoch key.
    TenantPurposeEpochKeyedV1,
    /// Opaque pointer to a selectively disclosed SD-JWT presentation.
    SdJwtDisclosureV1,
    /// Opaque pointer to an experimental BBS derived proof.
    BbsDerivedProofV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceScope {
    Operation,
    TenantPurposeEpoch,
    Presentation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopedReference {
    pub scheme: ReferenceScheme,
    pub scope: ReferenceScope,
    /// Opaque scope handle. Never put a patient ID or plain tenant slug here.
    pub scope_id: String,
    /// Trusted issuer identity, resolved against an external trust registry.
    pub issuer_id: String,
    /// Key identifier is mandatory for keyed/presentation schemes.
    pub key_id: Option<String>,
    /// Key epoch is mandatory for keyed pseudonyms and BBS proof issuer keys.
    pub key_epoch: Option<u64>,
    /// Opaque 256-bit token/handle. Never place a raw JWT or clinical payload here.
    pub handle: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceError {
    EmptyScope,
    EmptyIssuer,
    EmptyKeyId,
    MissingKeyId,
    MissingKeyEpoch,
    UnexpectedKeyMetadata,
    WrongScopeForScheme,
    MalformedOpaqueHandle,
    DuplicateScopedHandle,
    UnsupportedSchemaVersion,
    IncompleteOperationContext,
}

impl ScopedReference {
    pub fn validate(&self) -> Result<(), ReferenceError> {
        if self.scope_id.trim().is_empty() {
            return Err(ReferenceError::EmptyScope);
        }
        if self.issuer_id.trim().is_empty() {
            return Err(ReferenceError::EmptyIssuer);
        }
        if self
            .key_id
            .as_ref()
            .is_some_and(|key_id| key_id.trim().is_empty())
        {
            return Err(ReferenceError::EmptyKeyId);
        }

        let (expected_scope, expected_prefix, key_id_required, epoch_required) = match self.scheme {
            ReferenceScheme::TransactionRandomV1 => {
                (ReferenceScope::Operation, "op1:", false, false)
            }
            ReferenceScheme::TenantPurposeEpochKeyedV1 => {
                (ReferenceScope::TenantPurposeEpoch, "tpk1:", true, true)
            }
            ReferenceScheme::SdJwtDisclosureV1 => {
                (ReferenceScope::Presentation, "sdjwt1:", true, false)
            }
            ReferenceScheme::BbsDerivedProofV1 => {
                (ReferenceScope::Presentation, "bbs1:", true, true)
            }
        };

        if self.scope != expected_scope {
            return Err(ReferenceError::WrongScopeForScheme);
        }
        if key_id_required && self.key_id.is_none() {
            return Err(ReferenceError::MissingKeyId);
        }
        if epoch_required && self.key_epoch.is_none() {
            return Err(ReferenceError::MissingKeyEpoch);
        }
        if !key_id_required && self.key_id.is_some() {
            return Err(ReferenceError::UnexpectedKeyMetadata);
        }
        if !epoch_required && self.key_epoch.is_some() {
            return Err(ReferenceError::UnexpectedKeyMetadata);
        }

        let token = self
            .handle
            .strip_prefix(expected_prefix)
            .ok_or(ReferenceError::MalformedOpaqueHandle)?;
        if token.len() != 64
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ReferenceError::MalformedOpaqueHandle);
        }
        Ok(())
    }

    pub fn validate_operation_scope(&self, expected_scope_id: &str) -> Result<(), ReferenceError> {
        self.validate()?;
        if self.scheme != ReferenceScheme::TransactionRandomV1 || self.scope_id != expected_scope_id
        {
            return Err(ReferenceError::WrongScopeForScheme);
        }
        Ok(())
    }

    pub fn validate_tenant_purpose_epoch(
        &self,
        expected_scope_id: &str,
        expected_key_id: &str,
        expected_key_epoch: u64,
    ) -> Result<(), ReferenceError> {
        self.validate()?;
        if self.scheme != ReferenceScheme::TenantPurposeEpochKeyedV1
            || self.scope_id != expected_scope_id
            || self.key_id.as_deref() != Some(expected_key_id)
            || self.key_epoch != Some(expected_key_epoch)
        {
            return Err(ReferenceError::WrongScopeForScheme);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationReferenceSet {
    pub patient: ScopedReference,
    pub actor: ScopedReference,
    pub recipient: ScopedReference,
    pub tenant: ScopedReference,
}

impl OperationReferenceSet {
    pub fn validate(
        &self,
        operation_scope_id: &str,
        tenant_scope_id: &str,
        tenant_key_id: &str,
        tenant_key_epoch: u64,
    ) -> Result<(), ReferenceError> {
        self.patient.validate_operation_scope(operation_scope_id)?;
        self.actor.validate_operation_scope(operation_scope_id)?;
        self.recipient
            .validate_operation_scope(operation_scope_id)?;
        self.tenant.validate_tenant_purpose_epoch(
            tenant_scope_id,
            tenant_key_id,
            tenant_key_epoch,
        )?;
        Ok(())
    }
}

/// A v2 operation's subject and resource references are unique handles scoped
/// to one operation. The tenant reference uses a distinct keyed scope so
/// cross-operation tenancy checks are possible without a global patient token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationReferenceSetV2 {
    pub patient: ScopedReference,
    pub actor: ScopedReference,
    pub recipient: ScopedReference,
    pub source_resource: ScopedReference,
    pub resource_set: ScopedReference,
    pub consent_revision: ScopedReference,
    pub policy_revision: ScopedReference,
    pub idempotency_key: ScopedReference,
    pub tenant: ScopedReference,
}

impl OperationReferenceSetV2 {
    pub fn validate(
        &self,
        operation_scope_id: &str,
        tenant_scope_id: &str,
        tenant_key_id: &str,
        tenant_key_epoch: u64,
    ) -> Result<(), ReferenceError> {
        let operation_refs = [
            &self.patient,
            &self.actor,
            &self.recipient,
            &self.source_resource,
            &self.resource_set,
            &self.consent_revision,
            &self.policy_revision,
            &self.idempotency_key,
        ];
        let mut handles = std::collections::HashSet::new();
        for reference in operation_refs {
            reference.validate_operation_scope(operation_scope_id)?;
            if !handles.insert(reference.handle.as_str()) {
                return Err(ReferenceError::DuplicateScopedHandle);
            }
        }
        self.tenant.validate_tenant_purpose_epoch(
            tenant_scope_id,
            tenant_key_id,
            tenant_key_epoch,
        )?;
        if handles.contains(self.tenant.handle.as_str()) {
            return Err(ReferenceError::DuplicateScopedHandle);
        }
        Ok(())
    }
}

/// Versioned operation envelope for migration away from v1's generic digest
/// strings. The references bind identity/resource equivalence within one
/// operation without putting globally stable patient hashes into the graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationBindingV2 {
    pub schema_version: u32,
    pub operation_id: String,
    pub attempt_id: String,
    pub purpose: String,
    pub action: String,
    pub source_resource_version: String,
    pub revocation_epoch: u64,
    pub references: OperationReferenceSetV2,
}

impl OperationBindingV2 {
    pub fn validate(
        &self,
        operation_scope_id: &str,
        tenant_scope_id: &str,
        tenant_key_id: &str,
        tenant_key_epoch: u64,
    ) -> Result<(), ReferenceError> {
        if self.schema_version != 2 {
            return Err(ReferenceError::UnsupportedSchemaVersion);
        }
        if [
            self.operation_id.as_str(),
            self.attempt_id.as_str(),
            self.purpose.as_str(),
            self.action.as_str(),
            self.source_resource_version.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(ReferenceError::IncompleteOperationContext);
        }
        self.references.validate(
            operation_scope_id,
            tenant_scope_id,
            tenant_key_id,
            tenant_key_epoch,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation_ref(handle_byte: char) -> ScopedReference {
        ScopedReference {
            scheme: ReferenceScheme::TransactionRandomV1,
            scope: ReferenceScope::Operation,
            scope_id: "op-scope-opaque-001".into(),
            issuer_id: "issuer:identity-broker".into(),
            key_id: None,
            key_epoch: None,
            handle: format!("op1:{}", handle_byte.to_string().repeat(64)),
        }
    }

    fn tenant_ref() -> ScopedReference {
        ScopedReference {
            scheme: ReferenceScheme::TenantPurposeEpochKeyedV1,
            scope: ReferenceScope::TenantPurposeEpoch,
            scope_id: "tenant-purpose-epoch-opaque-7".into(),
            issuer_id: "issuer:identity-broker".into(),
            key_id: Some("tenant-pseudonym-key-7".into()),
            key_epoch: Some(7),
            handle: format!("tpk1:{}", "b".repeat(64)),
        }
    }

    fn valid_set() -> OperationReferenceSet {
        OperationReferenceSet {
            patient: operation_ref('a'),
            actor: operation_ref('c'),
            recipient: operation_ref('d'),
            tenant: tenant_ref(),
        }
    }

    fn valid_operation_binding_v2() -> OperationBindingV2 {
        OperationBindingV2 {
            schema_version: 2,
            operation_id: "operation-opaque-001".into(),
            attempt_id: "attempt-opaque-001".into(),
            purpose: "treatment-coordination".into(),
            action: "disclose".into(),
            source_resource_version: "4".into(),
            revocation_epoch: 17,
            references: OperationReferenceSetV2 {
                patient: operation_ref('a'),
                actor: operation_ref('c'),
                recipient: operation_ref('d'),
                source_resource: operation_ref('e'),
                resource_set: operation_ref('f'),
                consent_revision: operation_ref('1'),
                policy_revision: operation_ref('2'),
                idempotency_key: operation_ref('3'),
                tenant: tenant_ref(),
            },
        }
    }

    #[test]
    fn operation_binding_v2_accepts_fully_scoped_references() {
        assert!(valid_operation_binding_v2()
            .validate(
                "op-scope-opaque-001",
                "tenant-purpose-epoch-opaque-7",
                "tenant-pseudonym-key-7",
                7
            )
            .is_ok());
    }

    #[test]
    fn operation_binding_v2_rejects_cross_operation_resource_reference() {
        let mut binding = valid_operation_binding_v2();
        binding.references.resource_set.scope_id = "op-scope-opaque-other".into();
        assert_eq!(
            binding.validate(
                "op-scope-opaque-001",
                "tenant-purpose-epoch-opaque-7",
                "tenant-pseudonym-key-7",
                7
            ),
            Err(ReferenceError::WrongScopeForScheme)
        );
    }

    #[test]
    fn operation_binding_v2_rejects_reused_handle_across_reference_roles() {
        let mut binding = valid_operation_binding_v2();
        binding.references.resource_set.handle = binding.references.patient.handle.clone();
        assert_eq!(
            binding.validate(
                "op-scope-opaque-001",
                "tenant-purpose-epoch-opaque-7",
                "tenant-pseudonym-key-7",
                7
            ),
            Err(ReferenceError::DuplicateScopedHandle)
        );
    }

    #[test]
    fn operation_binding_v2_rejects_wrong_schema_version() {
        let mut binding = valid_operation_binding_v2();
        binding.schema_version = 1;
        assert_eq!(
            binding.validate(
                "op-scope-opaque-001",
                "tenant-purpose-epoch-opaque-7",
                "tenant-pseudonym-key-7",
                7
            ),
            Err(ReferenceError::UnsupportedSchemaVersion)
        );
    }

    #[test]
    fn operation_binding_v2_rejects_missing_authority_context() {
        let mut binding = valid_operation_binding_v2();
        binding.purpose.clear();
        assert_eq!(
            binding.validate(
                "op-scope-opaque-001",
                "tenant-purpose-epoch-opaque-7",
                "tenant-pseudonym-key-7",
                7
            ),
            Err(ReferenceError::IncompleteOperationContext)
        );
    }

    #[test]
    fn accepts_operation_scoped_random_handles_and_tenant_epoch_reference() {
        assert!(valid_set()
            .validate(
                "op-scope-opaque-001",
                "tenant-purpose-epoch-opaque-7",
                "tenant-pseudonym-key-7",
                7
            )
            .is_ok());
    }

    #[test]
    fn rejects_reused_operation_reference_outside_its_scope() {
        assert_eq!(
            operation_ref('a').validate_operation_scope("op-scope-opaque-002"),
            Err(ReferenceError::WrongScopeForScheme)
        );
    }

    #[test]
    fn rejects_stable_tenant_pseudonym_where_operation_scope_is_required() {
        let mut references = valid_set();
        references.patient = tenant_ref();
        assert_eq!(
            references.validate(
                "op-scope-opaque-001",
                "tenant-purpose-epoch-opaque-7",
                "tenant-pseudonym-key-7",
                7
            ),
            Err(ReferenceError::WrongScopeForScheme)
        );
    }

    #[test]
    fn rejects_tenant_key_epoch_mismatch() {
        assert_eq!(
            tenant_ref().validate_tenant_purpose_epoch(
                "tenant-purpose-epoch-opaque-7",
                "tenant-pseudonym-key-7",
                8
            ),
            Err(ReferenceError::WrongScopeForScheme)
        );
    }

    #[test]
    fn rejects_tenant_key_reference_without_key_id() {
        let mut reference = tenant_ref();
        reference.key_id = None;
        assert_eq!(reference.validate(), Err(ReferenceError::MissingKeyId));
    }

    #[test]
    fn rejects_non_hex_or_malformed_opaque_handles() {
        let mut reference = operation_ref('a');
        reference.handle = "op1:patient-123".into();
        assert_eq!(
            reference.validate(),
            Err(ReferenceError::MalformedOpaqueHandle)
        );
    }

    #[test]
    fn sd_jwt_reference_is_presentation_scoped_and_never_embeds_a_token() {
        let reference = ScopedReference {
            scheme: ReferenceScheme::SdJwtDisclosureV1,
            scope: ReferenceScope::Presentation,
            scope_id: "presentation-scope-opaque-3".into(),
            issuer_id: "issuer:identity-broker".into(),
            key_id: Some("issuer-signing-key-4".into()),
            key_epoch: None,
            handle: format!("sdjwt1:{}", "e".repeat(64)),
        };
        assert!(reference.validate().is_ok());
    }

    #[test]
    fn bbs_reference_is_explicitly_key_epoch_bound() {
        let reference = ScopedReference {
            scheme: ReferenceScheme::BbsDerivedProofV1,
            scope: ReferenceScope::Presentation,
            scope_id: "presentation-scope-opaque-4".into(),
            issuer_id: "issuer:experimental-bbs".into(),
            key_id: Some("bbs-issuer-key-2".into()),
            key_epoch: Some(2),
            handle: format!("bbs1:{}", "f".repeat(64)),
        };
        assert!(reference.validate().is_ok());
    }
}
