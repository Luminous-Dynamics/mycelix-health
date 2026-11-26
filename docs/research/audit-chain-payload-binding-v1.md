# Audit Chain Payload Binding v1

**Status:** shared audit-integrity primitive and unit tests on a draft branch. This is not a completed production audit pipeline or authorship-verification implementation.

## Finding addressed

The previous shared helper chained sequence, timestamp, agent, caller-supplied `entry_hash` and predecessor hash, but did not recompute the digest from the typed `AuditEvent`. The verifier checked predecessor continuity and increasing sequence only; it did not validate the stored event digest or require a valid genesis.

That made the event-payload binding weaker than the API comments implied. This was a source-level contract finding; this change does not claim an exploit or prove that the coordinator path was bypassed.

## New primitive

`zomes/shared/src/lib.rs` now defines:

- `hash_event(event)`: SHA-256 over a domain-separated, length-prefixed serialization of the typed `AuditEvent`.
- `hash_entry(entry)`: a domain-separated outer-entry hash binding sequence, timestamp, raw agent key, event digest and an explicit predecessor option tag.
- `verify_chain(entries)`: rejects empty chains, requires genesis sequence 0 with no predecessor, recomputes each event digest, enforces contiguous sequence numbers and recomputes each predecessor link from the full prior record.
- `verify_chain_with_expected_head(entries, expected_head)`: additionally compares the computed tail hash with a separately trusted digest.

The separate expected-head commitment is important. A shorter valid prefix is itself a valid chain; without an independently stored expected head or checkpoint, a verifier cannot know that a valid suffix was removed.

## Tests added

- A changed typed event produces a different event digest.
- Mutating an event while retaining its stored digest fails chain verification.
- A genesis with a predecessor is rejected.
- A sequence gap is rejected.
- A predecessor link binds prior event and metadata.
- A trusted final head detects suffix truncation and tail metadata tampering.

## Explicit limits

This change does **not** yet:
- change `create_chained_audit_entry` to compute hashes from its full request payload;
- bind its supplied `agent` to `agent_info()?.agent_initial_pubkey`;
- read or consume the persisted current chain head before append;
- define per-patient chain ownership, concurrent append/fork behavior or idempotent request identity;
- provide a trusted external checkpoint/head persistence mechanism;
- independently verify Holochain action signatures or the DHT's authorship proofs;
- guarantee append-only retention, backup or independent audit export.

The Consent coordinator currently accepts caller-provided `entry_hash`, `event_description` and `agent`, and stores metadata in `DataAccessLog.access_reason`. That separate integration must be updated to construct the canonical typed payload, recompute its digest inside the trusted zome, bind the authenticated caller, and enforce the current chain head with consume-once semantics. See [issue #332](https://github.com/Luminous-Dynamics/mycelix-health/issues/332).

## Qualification

The change is source-authored and unit-tested on the branch. Cargo/rustfmt/Clippy and Holochain integration qualification remain pending until the exact-head workflow completes. No security certification or production-readiness claim is made.
