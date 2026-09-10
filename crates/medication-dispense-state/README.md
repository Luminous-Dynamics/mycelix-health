# mycelix-medication-dispense-state

Conflict-preserving read model for finalized medication dispensing.

This crate consumes already DHT-valid dispense authorization/finalization/equivocation records and derives a deterministic medication-dispense view. It deliberately does not infer global absence from a local read set and does not collapse conflicting refill-slot evidence into a winner.

See `docs/security/MEDICATION_DISPENSE_STATE_V1.md` for the security and qualification contract.
