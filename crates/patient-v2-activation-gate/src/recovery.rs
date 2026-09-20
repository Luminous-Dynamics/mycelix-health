use core::fmt;

use crate::core::{self, ActivationState as InnerActivationState};

pub use crate::core::{
    evaluate_evidence, ActivationAuthorityReceipt, ActivationCandidate, ActivationPhase,
    AuthorityReceiptError, AuthorityRef, ContractId, ContractRef, CoordinatorWasmSetDigest,
    CutoverFreezeReceipt, DeploymentIdentity, DeploymentIdentityError, DnaManifestDigest,
    EvidenceGateError, EvidenceId, EvidenceReceipt, EvidenceScope, EvidenceSetReceipt,
    EvidenceState, IntegrityWasmSetDigest, LegacyStateDigest, MigrationRehearsalReceipt,
    OpaqueIdError, PatientWriteTarget, ProofLine, ProofRequirement, ProtectedReadFailure,
    ReadResolution, RequirementScope, SourceCommit, ToolchainDigest, TransitionError,
    ACTIVATION_CONTRACT_V2, REQUIRED_PROOFS,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbortReasonDigest([u8; 32]);

impl AbortReasonDigest {
    pub fn new(bytes: [u8; 32]) -> Result<Self, OpaqueIdError> {
        if bytes == [0; 32] {
            return Err(OpaqueIdError::AllZero);
        }
        Ok(Self(bytes))
    }
}

impl fmt::Debug for AbortReasonDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AbortReasonDigest([redacted])")
    }
}

/// A separately verified authority artifact that permits abandoning a frozen
/// pre-activation cutover. It does not authorize activation or migration.
#[derive(Clone, PartialEq, Eq)]
pub struct CutoverAbortReceipt {
    authority_ref: AuthorityRef,
    deployment: DeploymentIdentity,
    freeze: CutoverFreezeReceipt,
    reason_digest: AbortReasonDigest,
    issued_at_micros: i64,
    expires_at_micros: i64,
}

impl fmt::Debug for CutoverAbortReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CutoverAbortReceipt")
            .field("authority_ref", &self.authority_ref)
            .field("deployment", &self.deployment)
            .field("freeze", &self.freeze)
            .field("reason_digest", &self.reason_digest)
            .field("issued_at_micros", &self.issued_at_micros)
            .field("expires_at_micros", &self.expires_at_micros)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutoverAbortReceiptError {
    InvalidTimeWindow,
}

impl CutoverAbortReceipt {
    pub fn from_verified_adapter(
        authority_ref: AuthorityRef,
        deployment: DeploymentIdentity,
        freeze: CutoverFreezeReceipt,
        reason_digest: AbortReasonDigest,
        issued_at_micros: i64,
        expires_at_micros: i64,
    ) -> Result<Self, CutoverAbortReceiptError> {
        if expires_at_micros <= issued_at_micros {
            return Err(CutoverAbortReceiptError::InvalidTimeWindow);
        }
        Ok(Self {
            authority_ref,
            deployment,
            freeze,
            reason_digest,
            issued_at_micros,
            expires_at_micros,
        })
    }

    fn valid_at(&self, now_micros: i64) -> Result<(), CutoverAbortError> {
        if now_micros < self.issued_at_micros {
            return Err(CutoverAbortError::AuthorityNotYetValid);
        }
        if now_micros >= self.expires_at_micros {
            return Err(CutoverAbortError::AuthorityExpired);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutoverAbortError {
    InvalidState,
    NoCandidate,
    NoFreeze,
    DeploymentMismatch,
    FreezeMismatch,
    AuthorityNotYetValid,
    AuthorityExpired,
    ClockRollback,
}

/// Canonical activation state exposed by the crate. The inner activation core is
/// intentionally private so callers cannot bypass the cross-cutover monotonic
/// clock or explicit abort semantics by constructing an inner state directly.
#[derive(Clone)]
pub struct ActivationState {
    inner: InnerActivationState,
    candidate: Option<ActivationCandidate>,
    freeze: Option<CutoverFreezeReceipt>,
    last_security_time_micros: Option<i64>,
}

impl fmt::Debug for ActivationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActivationState")
            .field("phase", &self.phase())
            .field("candidate", &self.candidate)
            .field("freeze", &self.freeze)
            .field("last_security_time_micros", &self.last_security_time_micros)
            .finish()
    }
}

impl Default for ActivationState {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivationState {
    pub fn new() -> Self {
        Self {
            inner: InnerActivationState::new(),
            candidate: None,
            freeze: None,
            last_security_time_micros: None,
        }
    }

    pub fn phase(&self) -> ActivationPhase {
        self.inner.phase()
    }

    fn observe_security_time(&mut self, now_micros: i64) -> Result<(), CutoverAbortError> {
        if self
            .last_security_time_micros
            .is_some_and(|last| now_micros < last)
        {
            return Err(CutoverAbortError::ClockRollback);
        }
        self.last_security_time_micros = Some(now_micros);
        Ok(())
    }

    fn observe_transition_time(&mut self, now_micros: i64) -> Result<(), TransitionError> {
        if self
            .last_security_time_micros
            .is_some_and(|last| now_micros < last)
        {
            return Err(TransitionError::ClockRollback);
        }
        self.last_security_time_micros = Some(now_micros);
        Ok(())
    }

    pub fn prepare_candidate(
        &mut self,
        candidate: ActivationCandidate,
    ) -> Result<(), TransitionError> {
        self.inner.prepare_candidate(candidate)?;
        self.candidate = Some(candidate);
        Ok(())
    }

    pub fn admit_evidence(&mut self, evidence: EvidenceSetReceipt) -> Result<(), TransitionError> {
        self.inner.admit_evidence(evidence)
    }

    pub fn admit_cutover_freeze(
        &mut self,
        freeze: CutoverFreezeReceipt,
    ) -> Result<(), TransitionError> {
        self.inner.admit_cutover_freeze(freeze)?;
        self.freeze = Some(freeze);
        Ok(())
    }

    pub fn admit_rehearsal(
        &mut self,
        rehearsal: MigrationRehearsalReceipt,
    ) -> Result<(), TransitionError> {
        self.inner.admit_rehearsal(rehearsal)
    }

    pub fn admit_activation_authority(
        &mut self,
        authority: ActivationAuthorityReceipt,
        now_micros: i64,
    ) -> Result<(), TransitionError> {
        self.observe_transition_time(now_micros)?;
        self.inner.admit_activation_authority(authority, now_micros)
    }

    pub fn authorize_activation(&mut self, now_micros: i64) -> Result<(), TransitionError> {
        self.observe_transition_time(now_micros)?;
        self.inner.authorize_activation(now_micros)
    }

    pub fn activate_v2(&mut self, now_micros: i64) -> Result<(), TransitionError> {
        self.observe_transition_time(now_micros)?;
        self.inner.activate_v2(now_micros)
    }

    pub fn retire_legacy_v1(&mut self) -> Result<(), TransitionError> {
        self.inner.retire_legacy_v1()
    }

    /// Abandon a frozen cutover and restore legacy-v1 writes. This is never an
    /// in-place waiver: all candidate/evidence/freeze/rehearsal/activation state
    /// is destroyed. A later cutover must start again with a fresh candidate and
    /// a fresh freeze/rehearsal/authority chain.
    pub fn abort_cutover(
        &mut self,
        receipt: CutoverAbortReceipt,
        now_micros: i64,
    ) -> Result<(), CutoverAbortError> {
        match self.phase() {
            ActivationPhase::LegacyWritesFrozen
            | ActivationPhase::MigrationRehearsed
            | ActivationPhase::ActivationAuthorityAdmitted
            | ActivationPhase::V2ActivationAuthorized => {}
            _ => return Err(CutoverAbortError::InvalidState),
        }

        self.observe_security_time(now_micros)?;
        let candidate = self.candidate.ok_or(CutoverAbortError::NoCandidate)?;
        let freeze = self.freeze.ok_or(CutoverAbortError::NoFreeze)?;
        if receipt.deployment != candidate.deployment() {
            return Err(CutoverAbortError::DeploymentMismatch);
        }
        if receipt.freeze != freeze {
            return Err(CutoverAbortError::FreezeMismatch);
        }
        receipt.valid_at(now_micros)?;

        let observed_time = self.last_security_time_micros;
        self.inner = InnerActivationState::new();
        self.candidate = None;
        self.freeze = None;
        self.last_security_time_micros = observed_time;
        Ok(())
    }
}

pub fn write_target(state: &ActivationState) -> PatientWriteTarget {
    core::write_target(&state.inner)
}

pub fn resolve_read(
    state: &ActivationState,
    protected_result: Result<(), ProtectedReadFailure>,
) -> ReadResolution {
    core::resolve_read(&state.inner, protected_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(value: u8) -> [u8; 32] {
        [value; 32]
    }

    fn deployment() -> DeploymentIdentity {
        DeploymentIdentity::new(
            SourceCommit::new(b(1)).unwrap(),
            DnaManifestDigest::new(b(2)).unwrap(),
            IntegrityWasmSetDigest::new(b(3)).unwrap(),
            CoordinatorWasmSetDigest::new(b(4)).unwrap(),
            ToolchainDigest::new(b(5)).unwrap(),
            1,
        )
        .unwrap()
    }

    fn candidate() -> ActivationCandidate {
        ActivationCandidate::new(deployment())
    }

    fn evidence(candidate: ActivationCandidate) -> EvidenceSetReceipt {
        let receipts: Vec<_> = REQUIRED_PROOFS
            .iter()
            .enumerate()
            .map(|(index, requirement)| {
                EvidenceReceipt::from_adapter(
                    requirement.line,
                    EvidenceId::new(b((index + 10) as u8)).unwrap(),
                    EvidenceState::Qualified,
                    match requirement.scope {
                        RequirementScope::Reference(contract) => {
                            EvidenceScope::Reference(contract)
                        }
                        RequirementScope::Deployment => {
                            EvidenceScope::Deployment(candidate.deployment())
                        }
                    },
                )
            })
            .collect();
        evaluate_evidence(candidate, &receipts).unwrap()
    }

    fn freeze(candidate: ActivationCandidate) -> CutoverFreezeReceipt {
        CutoverFreezeReceipt::from_verified_adapter(
            EvidenceId::new(b(100)).unwrap(),
            EvidenceState::Qualified,
            candidate.deployment(),
            LegacyStateDigest::new(b(101)).unwrap(),
            1,
            true,
        )
    }

    fn frozen_state() -> (ActivationState, ActivationCandidate, CutoverFreezeReceipt) {
        let candidate = candidate();
        let evidence = evidence(candidate);
        let freeze = freeze(candidate);
        let mut state = ActivationState::new();
        state.prepare_candidate(candidate).unwrap();
        state.admit_evidence(evidence).unwrap();
        state.admit_cutover_freeze(freeze).unwrap();
        (state, candidate, freeze)
    }

    fn abort_receipt(
        candidate: ActivationCandidate,
        freeze: CutoverFreezeReceipt,
        start: i64,
        end: i64,
    ) -> CutoverAbortReceipt {
        CutoverAbortReceipt::from_verified_adapter(
            AuthorityRef::new(b(110)).unwrap(),
            candidate.deployment(),
            freeze,
            AbortReasonDigest::new(b(111)).unwrap(),
            start,
            end,
        )
        .unwrap()
    }

    #[test]
    fn authorized_abort_restores_legacy_writes_and_discards_cutover() {
        let (mut state, candidate, freeze) = frozen_state();
        assert_eq!(write_target(&state), PatientWriteTarget::DenyDuringCutover);
        state
            .abort_cutover(abort_receipt(candidate, freeze, 10, 100), 50)
            .unwrap();
        assert_eq!(state.phase(), ActivationPhase::LegacyV1Writable);
        assert_eq!(write_target(&state), PatientWriteTarget::LegacyV1);
        assert_eq!(state.admit_cutover_freeze(freeze), Err(TransitionError::InvalidState));
    }

    #[test]
    fn abort_requires_exact_deployment_and_freeze() {
        let (mut state, candidate, freeze) = frozen_state();
        let other_freeze = CutoverFreezeReceipt::from_verified_adapter(
            EvidenceId::new(b(120)).unwrap(),
            EvidenceState::Qualified,
            candidate.deployment(),
            LegacyStateDigest::new(b(121)).unwrap(),
            2,
            true,
        );
        let wrong = abort_receipt(candidate, other_freeze, 10, 100);
        assert_eq!(state.abort_cutover(wrong, 50), Err(CutoverAbortError::FreezeMismatch));
        assert_eq!(state.phase(), ActivationPhase::LegacyWritesFrozen);
        assert_eq!(freeze, state.freeze.unwrap());
    }

    #[test]
    fn abort_is_time_bounded_and_rollback_safe() {
        let (mut state, candidate, freeze) = frozen_state();
        let receipt = abort_receipt(candidate, freeze, 10, 100);
        assert_eq!(state.abort_cutover(receipt.clone(), 100), Err(CutoverAbortError::AuthorityExpired));
        assert_eq!(state.abort_cutover(receipt, 99), Err(CutoverAbortError::ClockRollback));
    }

    #[test]
    fn abort_is_impossible_before_freeze_or_after_activation() {
        let mut state = ActivationState::new();
        let candidate = candidate();
        let freeze = freeze(candidate);
        let receipt = abort_receipt(candidate, freeze, 10, 100);
        assert_eq!(state.abort_cutover(receipt, 50), Err(CutoverAbortError::InvalidState));
    }
}
