#![forbid(unsafe_code)]
//! Fail-closed administrative-authorization reference semantics.
//!
//! Two state machines are intentionally separated:
//! - ordinary admin authorization: dependency failure always denies;
//! - one-time bootstrap: explicit principal + DNA lineage + token proof.
//!
//! Bootstrap can never be activated by an authorization dependency failure.

use core::fmt;

pub const ADMIN_AUTH_CONTRACT_V1: u16 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrincipalId([u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MembershipProofId([u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DnaLineageId([u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BootstrapTokenDigest([u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdError {
    AllZero,
}

macro_rules! opaque32 {
    ($ty:ident) => {
        impl $ty {
            pub fn new(bytes: [u8; 32]) -> Result<Self, IdError> {
                if bytes == [0u8; 32] {
                    return Err(IdError::AllZero);
                }
                Ok(Self(bytes))
            }
        }

        impl fmt::Debug for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($ty), "([redacted])"))
            }
        }
    };
}

opaque32!(PrincipalId);
opaque32!(MembershipProofId);
opaque32!(DnaLineageId);
opaque32!(BootstrapTokenDigest);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdminDependencyFailure {
    ConsentZomeUnavailable,
    Network,
    Authentication,
    UnauthorizedCall,
    Countersigning,
    Decode,
    RegistryMissing,
    RegistryMalformed,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdminEvidence {
    ConfirmedAdmin {
        principal: PrincipalId,
        membership_proof: MembershipProofId,
        registry_epoch: u64,
    },
    ConfirmedNonAdmin {
        principal: PrincipalId,
        registry_epoch: u64,
    },
    DependencyFailure(AdminDependencyFailure),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenyReason {
    NotAdmin,
    DependencyUnavailable,
    InvalidRegistryEpoch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdminDecision {
    Allow {
        principal: PrincipalId,
        membership_proof: MembershipProofId,
        registry_epoch: u64,
        contract_version: u16,
    },
    Deny(DenyReason),
}

impl AdminDecision {
    pub fn allowed(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }
}

/// Ordinary administrative authorization.
///
/// No dependency failure can produce `Allow` through this function.
pub fn authorize_admin(evidence: AdminEvidence) -> AdminDecision {
    match evidence {
        AdminEvidence::ConfirmedAdmin {
            principal,
            membership_proof,
            registry_epoch,
        } if registry_epoch > 0 => AdminDecision::Allow {
            principal,
            membership_proof,
            registry_epoch,
            contract_version: ADMIN_AUTH_CONTRACT_V1,
        },
        AdminEvidence::ConfirmedAdmin {
            registry_epoch: 0, ..
        } => AdminDecision::Deny(DenyReason::InvalidRegistryEpoch),
        AdminEvidence::ConfirmedNonAdmin { .. } => AdminDecision::Deny(DenyReason::NotAdmin),
        AdminEvidence::DependencyFailure(_) => {
            AdminDecision::Deny(DenyReason::DependencyUnavailable)
        }
    }
}

/// Explicit bootstrap configuration. This must come from a separately trusted
/// installation/genesis path, not from "first caller wins" application logic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootstrapConfig {
    pub dna_lineage: DnaLineageId,
    pub bootstrap_principal: PrincipalId,
    pub token_digest: BootstrapTokenDigest,
    pub not_before_micros: i64,
    pub expires_at_micros: i64,
}

impl BootstrapConfig {
    pub fn new(
        dna_lineage: DnaLineageId,
        bootstrap_principal: PrincipalId,
        token_digest: BootstrapTokenDigest,
        not_before_micros: i64,
        expires_at_micros: i64,
    ) -> Result<Self, BootstrapConfigError> {
        if expires_at_micros <= not_before_micros {
            return Err(BootstrapConfigError::InvalidWindow);
        }
        Ok(Self {
            dna_lineage,
            bootstrap_principal,
            token_digest,
            not_before_micros,
            expires_at_micros,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootstrapConfigError {
    InvalidWindow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BootstrapState {
    Uninitialized(BootstrapConfig),
    Initialized {
        dna_lineage: DnaLineageId,
        founding_admin: PrincipalId,
        registry_epoch: u64,
        initialized_at_micros: i64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootstrapAttempt {
    pub caller: PrincipalId,
    pub dna_lineage: DnaLineageId,
    pub token_digest: BootstrapTokenDigest,
    pub now_micros: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootstrapError {
    AlreadyInitialized,
    WrongDnaLineage,
    WrongPrincipal,
    WrongToken,
    NotYetValid,
    Expired,
    RegistryEpochOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootstrapReceipt {
    pub dna_lineage: DnaLineageId,
    pub founding_admin: PrincipalId,
    pub registry_epoch: u64,
    pub initialized_at_micros: i64,
    pub contract_version: u16,
}

/// Consume the one-time bootstrap authority and irreversibly transition to the
/// normal admin-registry state.
pub fn consume_bootstrap(
    state: &mut BootstrapState,
    attempt: BootstrapAttempt,
) -> Result<BootstrapReceipt, BootstrapError> {
    let config = match state {
        BootstrapState::Initialized { .. } => return Err(BootstrapError::AlreadyInitialized),
        BootstrapState::Uninitialized(config) => config.clone(),
    };

    if attempt.dna_lineage != config.dna_lineage {
        return Err(BootstrapError::WrongDnaLineage);
    }
    if attempt.caller != config.bootstrap_principal {
        return Err(BootstrapError::WrongPrincipal);
    }
    if attempt.token_digest != config.token_digest {
        return Err(BootstrapError::WrongToken);
    }
    if attempt.now_micros < config.not_before_micros {
        return Err(BootstrapError::NotYetValid);
    }
    if attempt.now_micros >= config.expires_at_micros {
        return Err(BootstrapError::Expired);
    }

    let registry_epoch = 1u64
        .checked_add(0)
        .ok_or(BootstrapError::RegistryEpochOverflow)?;
    let receipt = BootstrapReceipt {
        dna_lineage: config.dna_lineage,
        founding_admin: config.bootstrap_principal,
        registry_epoch,
        initialized_at_micros: attempt.now_micros,
        contract_version: ADMIN_AUTH_CONTRACT_V1,
    };

    *state = BootstrapState::Initialized {
        dna_lineage: receipt.dna_lineage,
        founding_admin: receipt.founding_admin,
        registry_epoch: receipt.registry_epoch,
        initialized_at_micros: receipt.initialized_at_micros,
    };

    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(byte: u8) -> PrincipalId {
        PrincipalId::new([byte; 32]).unwrap()
    }

    fn proof(byte: u8) -> MembershipProofId {
        MembershipProofId::new([byte; 32]).unwrap()
    }

    fn dna(byte: u8) -> DnaLineageId {
        DnaLineageId::new([byte; 32]).unwrap()
    }

    fn token(byte: u8) -> BootstrapTokenDigest {
        BootstrapTokenDigest::new([byte; 32]).unwrap()
    }

    #[test]
    fn confirmed_admin_with_nonzero_epoch_is_allowed() {
        let decision = authorize_admin(AdminEvidence::ConfirmedAdmin {
            principal: principal(1),
            membership_proof: proof(2),
            registry_epoch: 7,
        });
        assert!(decision.allowed());
        let AdminDecision::Allow { registry_epoch, .. } = decision else {
            panic!("expected allow");
        };
        assert_eq!(registry_epoch, 7);
    }

    #[test]
    fn non_admin_is_denied() {
        let decision = authorize_admin(AdminEvidence::ConfirmedNonAdmin {
            principal: principal(1),
            registry_epoch: 2,
        });
        assert_eq!(decision, AdminDecision::Deny(DenyReason::NotAdmin));
    }

    #[test]
    fn every_dependency_failure_denies() {
        let failures = [
            AdminDependencyFailure::ConsentZomeUnavailable,
            AdminDependencyFailure::Network,
            AdminDependencyFailure::Authentication,
            AdminDependencyFailure::UnauthorizedCall,
            AdminDependencyFailure::Countersigning,
            AdminDependencyFailure::Decode,
            AdminDependencyFailure::RegistryMissing,
            AdminDependencyFailure::RegistryMalformed,
            AdminDependencyFailure::Other,
        ];
        for failure in failures {
            let decision = authorize_admin(AdminEvidence::DependencyFailure(failure));
            assert_eq!(
                decision,
                AdminDecision::Deny(DenyReason::DependencyUnavailable)
            );
        }
    }

    #[test]
    fn zero_registry_epoch_cannot_authorize() {
        let decision = authorize_admin(AdminEvidence::ConfirmedAdmin {
            principal: principal(1),
            membership_proof: proof(2),
            registry_epoch: 0,
        });
        assert_eq!(
            decision,
            AdminDecision::Deny(DenyReason::InvalidRegistryEpoch)
        );
    }

    fn config() -> BootstrapConfig {
        BootstrapConfig::new(dna(3), principal(4), token(5), 100, 200).unwrap()
    }

    fn valid_attempt() -> BootstrapAttempt {
        BootstrapAttempt {
            caller: principal(4),
            dna_lineage: dna(3),
            token_digest: token(5),
            now_micros: 150,
        }
    }

    #[test]
    fn bootstrap_requires_explicit_config_and_consumes_once() {
        let mut state = BootstrapState::Uninitialized(config());
        let receipt = consume_bootstrap(&mut state, valid_attempt()).unwrap();
        assert_eq!(receipt.registry_epoch, 1);
        assert!(matches!(state, BootstrapState::Initialized { .. }));
        assert_eq!(
            consume_bootstrap(&mut state, valid_attempt()),
            Err(BootstrapError::AlreadyInitialized)
        );
    }

    #[test]
    fn wrong_bootstrap_principal_is_denied() {
        let mut state = BootstrapState::Uninitialized(config());
        let mut attempt = valid_attempt();
        attempt.caller = principal(9);
        assert_eq!(
            consume_bootstrap(&mut state, attempt),
            Err(BootstrapError::WrongPrincipal)
        );
        assert!(matches!(state, BootstrapState::Uninitialized(_)));
    }

    #[test]
    fn wrong_dna_lineage_is_denied() {
        let mut state = BootstrapState::Uninitialized(config());
        let mut attempt = valid_attempt();
        attempt.dna_lineage = dna(9);
        assert_eq!(
            consume_bootstrap(&mut state, attempt),
            Err(BootstrapError::WrongDnaLineage)
        );
    }

    #[test]
    fn wrong_bootstrap_token_is_denied() {
        let mut state = BootstrapState::Uninitialized(config());
        let mut attempt = valid_attempt();
        attempt.token_digest = token(9);
        assert_eq!(
            consume_bootstrap(&mut state, attempt),
            Err(BootstrapError::WrongToken)
        );
    }

    #[test]
    fn bootstrap_is_time_bounded() {
        let mut early = BootstrapState::Uninitialized(config());
        let mut early_attempt = valid_attempt();
        early_attempt.now_micros = 99;
        assert_eq!(
            consume_bootstrap(&mut early, early_attempt),
            Err(BootstrapError::NotYetValid)
        );

        let mut late = BootstrapState::Uninitialized(config());
        let mut late_attempt = valid_attempt();
        late_attempt.now_micros = 200;
        assert_eq!(
            consume_bootstrap(&mut late, late_attempt),
            Err(BootstrapError::Expired)
        );
    }

    #[test]
    fn bootstrap_window_must_be_finite_and_nonempty() {
        assert_eq!(
            BootstrapConfig::new(dna(3), principal(4), token(5), 100, 100),
            Err(BootstrapConfigError::InvalidWindow)
        );
        assert_eq!(
            BootstrapConfig::new(dna(3), principal(4), token(5), 101, 100),
            Err(BootstrapConfigError::InvalidWindow)
        );
    }

    #[test]
    fn authorization_dependency_failure_cannot_mutate_bootstrap_state() {
        let state = BootstrapState::Uninitialized(config());
        let before = state.clone();
        let decision = authorize_admin(AdminEvidence::DependencyFailure(
            AdminDependencyFailure::ConsentZomeUnavailable,
        ));
        assert!(!decision.allowed());
        assert_eq!(state, before);
    }

    #[test]
    fn no_first_caller_wins_behavior() {
        let mut state = BootstrapState::Uninitialized(config());
        let attacker = BootstrapAttempt {
            caller: principal(8),
            dna_lineage: dna(3),
            token_digest: token(5),
            now_micros: 150,
        };
        assert_eq!(
            consume_bootstrap(&mut state, attacker),
            Err(BootstrapError::WrongPrincipal)
        );
        let receipt = consume_bootstrap(&mut state, valid_attempt()).unwrap();
        assert_eq!(receipt.founding_admin, principal(4));
    }

    #[test]
    fn all_zero_ids_are_rejected() {
        assert_eq!(PrincipalId::new([0; 32]), Err(IdError::AllZero));
        assert_eq!(MembershipProofId::new([0; 32]), Err(IdError::AllZero));
        assert_eq!(DnaLineageId::new([0; 32]), Err(IdError::AllZero));
        assert_eq!(BootstrapTokenDigest::new([0; 32]), Err(IdError::AllZero));
    }

    #[test]
    fn opaque_material_is_redacted_in_debug_output() {
        assert_eq!(format!("{:?}", principal(1)), "PrincipalId([redacted])");
        assert_eq!(format!("{:?}", proof(2)), "MembershipProofId([redacted])");
        assert_eq!(format!("{:?}", token(3)), "BootstrapTokenDigest([redacted])");
    }
}
