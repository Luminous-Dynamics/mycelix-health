# mycelix-medication-activation-state

Pure reducer for canonical medication activation state.

The crate intentionally has no `active: bool` API. It distinguishes ordinary qualified activation, emergency override activation, explicit conflict, terminated history, and legacy-unqualified Active prescription records.

Inputs are expected to come from records that already passed the corresponding Holochain integrity zomes. The reducer rechecks semantic digest domains and terminal-event linkage and fails closed on orphan/mismatched lineages.

See `docs/security/MEDICATION_ACTIVATION_READ_MODEL_V1.md` for the security and migration contract.
