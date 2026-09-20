# Patient v2 Production Migration Result Core

Reference semantics for PATIENT-PRIV-006 (#205).

## Core theorem

```text
migration mechanism qualified
    != rehearsal passed
    != legacy writes frozen
    != production migration executed
    != destination verified
    != activation authorized
```

This crate covers the missing boundary between a successful migration rehearsal
and evidence that the **actual frozen production source** was transformed into the
**exact protected destination lineage** later proposed for activation.

## Exact source context

`MigrationContext` binds:

- exact deployment identity;
- exact cutover-freeze evidence ID;
- exact frozen legacy-state digest;
- exact non-zero freeze epoch;
- exact rehearsal evidence ID;
- exact migration plan/schema reference.

A production result for any other context is rejected.

## Destination identity

`DestinationIdentity` binds:

- protected destination root commitment;
- protected manifest commitment;
- non-zero key epoch;
- non-zero policy epoch;
- non-zero generation;
- exact predecessor root for generation > 1.

Generation 1 cannot claim a predecessor. Later generations require one.

## Route completeness

`RouteAccounting` carries a commitment plus required/migrated/unresolved counts.
Admission requires:

```text
unresolved_routes = 0
migrated_routes = required_routes
```

The production receipt independently requires positive adapter evidence for:

- idempotency;
- conflict freedom;
- source→destination verification;
- protected payload verification;
- protected topology verification;
- historical-public-exposure acknowledgement.

Payload verification and topology verification are deliberately separate. A
ciphertext destination is not sufficient if the discovery/link graph still leaks
Patient identity or sensitive relationships.

## Registration and generations

The public `MigrationRegistry` is scoped to **one exact `MigrationContext`**.
Alternate deployment/freeze/rehearsal/plan contexts are rejected rather than
coexisting behind one registry.

For that one context:

- exact same receipt replay is idempotent;
- a different result at the same generation is a conflict;
- generation N requires active generation N-1;
- its predecessor root must equal the exact previous destination root;
- only the latest active generation is eligible for an activation binding.

`ActivationMigrationBinding` is the typed object intended to be consumed by a
later composition gate with PATIENT-PRIV-005/#204. This crate does not itself
activate Patient v2.

## Post-migration abort

Once protected envelopes/manifests/indexes have actually been emitted, the
ordinary pre-migration `CutoverAbortReceipt` from #204 is no longer sufficient.

`PostMigrationAbortReceipt` must bind the exact migration/context/destination and
positively assert separately verified evidence that:

- the destination lineage is quarantined;
- active discovery is disabled;
- the abandoned locator is inactive;
- abandoned capabilities are inactive;
- re-enabling legacy writes is explicitly authorized;
- historical public/DHT observations may persist.

This is a **quarantine/reachability theorem, not erasure**. The crate never claims
that peer-retained ciphertext or historical DHT observations were deleted.

After successful abandonment, the strict registry permanently retires the whole
freeze lineage. Older generations cannot become activation-eligible again and
no new migration result may be registered under that abandoned freeze/context.
A later migration attempt must use a fresh cutover lineage.

## Time boundary

Post-migration abort authority is finite and the registry maintains a monotonic
security-time observation. Once an abort is observed expired at time T, retrying
with a time before T is rejected as clock rollback.

Trusted production time remains a separate adapter theorem.

## Deliberate boundaries

This crate does **not**:

- execute the actual migration;
- encrypt or decrypt Patient data;
- inspect Holochain DHT state;
- generate destination commitments;
- verify signatures/governance authority;
- establish a trusted clock;
- prove that quarantine occurred outside the supplied verified adapter evidence;
- erase peer-retained data;
- activate Patient v2;
- establish privacy, clinical validity, HIPAA, POPIA, GDPR, or other legal compliance.

It defines the fail-closed structural contract that production migration,
verification, quarantine, and activation adapters must satisfy.
