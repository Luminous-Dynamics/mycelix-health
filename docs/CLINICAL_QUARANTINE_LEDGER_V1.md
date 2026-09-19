# Clinical Quarantine & Reconciliation Ledger v1

## Purpose

Clinical ingestion must be allowed to say **not safe to promote yet** without either dropping evidence or placing rejected protected health information (PHI) onto a shared ledger.

This tranche defines a privacy-preserving public reconciliation lineage for FHIR/clinical inputs that fail strict semantic promotion.

## Split-storage rule

The quarantine system has two logically separate stores:

### Sensitive local/encrypted store

May contain:

- original FHIR/clinical payload;
- patient identifiers;
- source-system/resource identifiers;
- human-readable validation messages;
- local remediation context;
- authorized reviewer notes.

This store must use the appropriate local/institutional encryption and access-control boundary. v1 deliberately does not define its backend.

### Shared reconciliation ledger

`QuarantineEvent` contains only:

- random opaque case nonce;
- keyed payload commitment;
- typed scope;
- closed typed reason codes;
- immutable sequence/state;
- transition timestamp;
- producer artifact digest;
- exact predecessor-event digest;
- opaque reviewer-authority commitment when review begins;
- decision artifact digest at resolution;
- promoted artifact digest only when promotion succeeds.

It contains **no field for raw payload, patient ID, resource ID, source URL, practitioner identifier, free-form error text, or reviewer notes**.

## Why keyed commitments

A plain SHA-256 hash of a clinical record is deterministic. If an attacker can guess or obtain a candidate payload, they may hash it and test whether it matches a public quarantine record.

v1 therefore makes the sensitive payload identity an `OpaqueCommitment` using one of:

- HMAC-SHA-256;
- keyed BLAKE3.

The commitment key remains outside the shared record.

This reduces confirmation/dictionary risk but does not make metadata public by default. Deployments must still apply appropriate authorization, minimization, retention, and jurisdictional policy to the ledger itself.

## Immutable state machine

The only valid promotion lineage is:

`Pending -> UnderReview -> ResolvedPromoted`

or:

`Pending -> UnderReview -> ResolvedDiscarded`

`UnderReview -> UnderReview` is allowed for continued/reassigned review while preserving lineage.

Terminal states cannot transition.

Every event after sequence zero binds the exact predecessor event digest and increments sequence by one. Payload commitment, scope, case identity, and reason set cannot change across a case lineage.

## Human authority

Promotion/discard resolution requires an opaque reviewer-authority commitment. This structure does not itself establish the reviewer's authority; a later verifier must bind that commitment to an authorized credential/scope decision.

No direct `Pending -> Promoted` transition exists in v1.

## Reason taxonomy

FHIR conformance failures map into a closed `QuarantineReasonCode` taxonomy. The mapper intentionally discards the source issue's human-readable message and resource identifier.

The absence of an `Other(String)` variant is deliberate: free text would become an easy accidental PHI exfiltration channel.

New reason codes require an explicit code/schema review.

## FHIR integration

`reason_from_fhir_issue()` maps the strict FHIR promotion profile into quarantine reason codes without copying sensitive identifiers/messages.

A later adapter should:

1. retain the rejected raw resource only in authorized encrypted/local storage;
2. compute a keyed commitment over the canonical payload or protected envelope;
3. create one `Pending` public event with typed reason codes;
4. present the local item to an authorized reconciliation workflow;
5. append review/resolution events without rewriting history;
6. if promoted, bind the terminal event to the digest of the exact promoted semantic artifact.

## Non-goals

v1 does not:

- store raw clinical data;
- choose a local PHI storage backend;
- define reviewer credential verification;
- automatically repair clinical records;
- automatically promote corrected records;
- claim that a keyed commitment makes a ledger non-sensitive;
- replace institutional retention/legal-hold policy.

## Next

1. Add an encrypted/local payload-store interface whose handles are never serialized into the shared event.
2. Bind reviewer authority commitments to verified Mycelix practitioner credentials.
3. Integrate strict FHIR conformance reports with quarantine-event creation.
4. Add corrected-resource reconciliation that reruns the full semantic/conformance path from the beginning.
5. Require a fresh ClinicalFact/artifact digest at successful promotion rather than reusing the rejected payload identity.
6. Add retention/expiry policy as a separately authorized layer rather than deleting immutable evidence silently.
