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
                    RequirementScope::Reference(contract) => EvidenceScope::Reference(contract),
                    RequirementScope::Deployment => EvidenceScope::Deployment(candidate.deployment()),
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

fn rehearsal(candidate: ActivationCandidate) -> MigrationRehearsalReceipt {
    MigrationRehearsalReceipt::from_verified_adapter(
        EvidenceId::new(b(102)).unwrap(),
        EvidenceState::Qualified,
        candidate.deployment(),
        LegacyStateDigest::new(b(101)).unwrap(),
        1,
        true,
        true,
        true,
        true,
    )
}

fn abort_receipt(candidate: ActivationCandidate, freeze: CutoverFreezeReceipt) -> CutoverAbortReceipt {
    CutoverAbortReceipt::from_verified_adapter(
        AuthorityRef::new(b(110)).unwrap(),
        candidate.deployment(),
        freeze,
        AbortReasonDigest::new(b(111)).unwrap(),
        10,
        100,
    )
    .unwrap()
}

fn authority(
    candidate: ActivationCandidate,
    evidence: &EvidenceSetReceipt,
    freeze: CutoverFreezeReceipt,
    rehearsal: MigrationRehearsalReceipt,
) -> ActivationAuthorityReceipt {
    ActivationAuthorityReceipt::from_verified_adapter(
        AuthorityRef::new(b(120)).unwrap(),
        candidate.deployment(),
        evidence.transcript_bytes(),
        EvidenceId::new(b(100)).unwrap(),
        LegacyStateDigest::new(b(101)).unwrap(),
        1,
        EvidenceId::new(b(102)).unwrap(),
        10,
        100,
        true,
    )
    .unwrap()
}

#[test]
fn abort_is_rejected_after_v2_is_active() {
    let candidate = candidate();
    let evidence = evidence(candidate);
    let freeze = freeze(candidate);
    let rehearsal = rehearsal(candidate);
    let authority = authority(candidate, &evidence, freeze, rehearsal);
    let abort = abort_receipt(candidate, freeze);

    let mut state = ActivationState::new();
    state.prepare_candidate(candidate).unwrap();
    state.admit_evidence(evidence).unwrap();
    state.admit_cutover_freeze(freeze).unwrap();
    state.admit_rehearsal(rehearsal).unwrap();
    state.admit_activation_authority(authority, 50).unwrap();
    state.authorize_activation(60).unwrap();
    state.activate_v2(70).unwrap();

    assert_eq!(state.phase(), ActivationPhase::V2Active);
    assert_eq!(write_target(&state), PatientWriteTarget::ProtectedV2);
    assert_eq!(state.abort_cutover(abort, 80), Err(CutoverAbortError::InvalidState));
    assert_eq!(state.phase(), ActivationPhase::V2Active);
    assert_eq!(write_target(&state), PatientWriteTarget::ProtectedV2);
}

#[test]
fn abort_is_rejected_after_legacy_is_retired() {
    let candidate = candidate();
    let evidence = evidence(candidate);
    let freeze = freeze(candidate);
    let rehearsal = rehearsal(candidate);
    let authority = authority(candidate, &evidence, freeze, rehearsal);
    let abort = abort_receipt(candidate, freeze);

    let mut state = ActivationState::new();
    state.prepare_candidate(candidate).unwrap();
    state.admit_evidence(evidence).unwrap();
    state.admit_cutover_freeze(freeze).unwrap();
    state.admit_rehearsal(rehearsal).unwrap();
    state.admit_activation_authority(authority, 50).unwrap();
    state.authorize_activation(60).unwrap();
    state.activate_v2(70).unwrap();
    state.retire_legacy_v1().unwrap();

    assert_eq!(state.phase(), ActivationPhase::LegacyV1ReadOnly);
    assert_eq!(state.abort_cutover(abort, 80), Err(CutoverAbortError::InvalidState));
    assert_eq!(write_target(&state), PatientWriteTarget::ProtectedV2);
}
