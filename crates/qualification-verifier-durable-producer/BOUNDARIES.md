# Boundaries

## What this crate proves when qualified

Only the composition semantics:

1. the durable store policy generation and key-lineage commitment must equal the signer identity;
2. an issued challenge is returned only after exact nonce/window persistence succeeds;
3. a mint attempt durably consumes the exact request challenge before lower producer delegation;
4. expired durable challenges are burned and never delegated;
5. lower producer failure after durable consumption does not restore the challenge;
6. durable-store time-floor rollback is denied before process-local issue/delegation;
7. no raw producer/store accessor or in-memory-only fallback is part of the public API.

## Parent proof lines remain independent

A green child run does not retroactively qualify:

- #232 / the producer crypto/provider theorem;
- #234 / the filesystem durability theorem.

The child workflow reruns relevant parent tests only as supporting composition evidence.

## Real evidence remains closed

`VerifierOwnedMintEvidenceV1` remains a private-field parent type with no public production constructor. This crate does not add one.

Therefore the child does not claim a successful real-evidence end-to-end mint until the concrete verifier-evidence path is integrated.

## Restart behavior

Outstanding pre-restart challenges are not rehydrated into #232's in-memory map. This is intentional fail-closed reduced availability.

## Persistence failure after local issue

If #232 issues a process-local challenge but #234 fails to persist it, the wrapper enters a poisoned fail-stop state and returns no challenge. The caller must discard/restart the wrapper rather than continue from split freshness state.

## External rollback/tamper resistance

The local freshness store is not an adversarial monotonic authority. Whole-volume rollback and hostile local-writer replacement remain #216/#221 (or hardware monotonic-state) concerns.

## Process/transport boundary

This crate has no IPC transport, peer credentials, capability token, endpoint pin, single-instance lock, service manager theorem or key-custody theorem.

## Product boundary

A provider receipt remains portable evidence, not #224/#208 product authority. 009D must independently verify producer/provider evidence before constructing private authority tokens.
