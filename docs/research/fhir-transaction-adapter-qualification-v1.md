# FHIR R4 Transaction Adapter Qualification v1

**Status:** synthetic qualification contract and deterministic evidence evaluator. No production FHIR endpoint has been qualified by this document.

## Positioning relative to existing conformance tooling

This profile complements established FHIR conformance tooling; it is not a replacement for it.

- Run the official FHIR R4 validator and the applicable implementation-guide validator for structural/profile conformance.
- Reuse Inferno Test Kits for their intended API-conformance scope, including the ONC Standardized API and SMART App Launch test suites where applicable.
- Run this profile after those checks to qualify the *deployed instance's* transactional and recovery behavior under synthetic adversarial histories.

Inferno is an existing framework for authoring and running portable FHIR API conformance suites, and ONC lists it for the §170.315(g)(10) standardized API criterion with FHIR R4 and US Core. Mycelix should compose with those tools and add the transactional authorization/recovery probes they are not designed to replace.

A green FHIR validator or Inferno suite does not imply that a Consent revocation race is closed, that a cross-system outbox is durable, or that a lost remote acknowledgement can be reconciled without a duplicate side effect.

## Why qualification must be active

FHIR R4 transaction Bundles can submit a set of server interactions as one atomic transaction, where the server accepts all changes or rejects the set. The R4 specification explicitly notes that transactions and conditional create/update/delete are complex and are not expected to be implemented by every server. A transaction can be useful for storing a target resource with its Provenance, but the actual deployed server must be tested.

A CapabilityStatement describes capabilities for a FHIR version and may describe a deployed instance, generic software capability, or requirements. It is valuable for discovery and compatibility, but is an interim statement and does not demonstrate that the configured instance behaves correctly under rollback, race, stale version, or response-loss conditions.

The R4 REST API also does not itself define authentication, authorization, or audit collection. Those controls must be qualified as separate application/server behavior.

Normative references:
- FHIR R4 HTTP: https://hl7.org/fhir/R4/http.html
- FHIR R4 Bundle: https://hl7.org/fhir/R4/bundle.html
- FHIR R4 CapabilityStatement: https://hl7.org/fhir/R4/capabilitystatement.html
- FHIR R4 Consent: https://hl7.org/fhir/R4/consent.html
- SMART App Launch: https://hl7.org/fhir/smart-app-launch/

## Qualification unit

Qualification is scoped to one concrete deployed instance, not a vendor or product family. The evidence record binds:

- exact FHIR base URL (including path and exact origin);
- FHIR version and a CapabilityStatement of kind `instance`;
- CapabilityStatement implementation URL and advertised server-level `transaction` interaction;
- exact SHA-256 digest of the captured CapabilityStatement;
- implementation/software identity and version;
- probe-suite identifier/version, run time, and individual evidence artifact digests;
- the result's freshness window.

A change to the base URL, FHIR version, software/build identity, or CapabilityStatement digest requires fresh evidence. CapabilityStatement claims do not substitute for active probes.

## Decision contract

The evaluator returns exactly one state:

- `qualified`: every required probe is present exactly once, passed, has a well-formed evidence digest, is fresh, and is bound to the same server software and CapabilityStatement digest.
- `not_qualified`: the captured deployed server explicitly does not meet a required FHIR version/capability, or an active required probe failed.
- `defer`: evidence is missing, duplicated, malformed, stale, future-dated beyond allowed skew, unavailable, or internally inconsistent.

Neither `not_qualified` nor `defer` permits a release. Missing or unavailable evidence is not a policy denial about the server's behavior; it is an inability to establish qualification.

The shell evaluator is intentionally small and deterministic. It validates the submitted evidence record and the binding between its fields. It does **not** fetch a FHIR endpoint, run the probes, independently fetch evidence blobs, verify digital signatures, or cryptographically recompute the supplied artifact digests. A qualification controller must perform those steps before producing a signed evidence record. The evaluator's output is only as trustworthy as the evidence-ingestion boundary.

## Required active probes

All probes use synthetic, non-PHI resources and isolated test tenants.

1. **Transaction rollback atomicity.** Submit a transaction with one valid synthetic write and one intentionally invalid operation. Verify the response and then independently read the target: no partial write may remain.
2. **Target + Provenance + AuditEvent.** Submit a successful transaction that writes a synthetic target and its Provenance and AuditEvent. Verify exact references, resource versions, and that all expected entries are present.
3. **Stale version precondition.** Read a resource version, mutate it in a competing session, then submit an update with the stale version precondition. Verify rejection and no dependent partial write.
4. **Conditional create deduplication.** Concurrently submit equivalent conditional creates. Verify one logical resource exists and every response resolves to that same resource.
5. **Commit-time Consent guard.** Race Consent revocation against the exact release commit. Only a release serialized before revocation may complete. A pre-read followed by an unguarded transaction is a failure. The guard may be a vendor-specific transaction hook or another server-side mechanism; the standard transaction interaction alone does not establish Consent enforcement.
6. **Audit and outbox atomic intent.** Verify the release intent, audit intent, and durable outbox row are committed in one backend transaction before any remote delivery begins. Where a FHIR transaction cannot include the required local state, this capability is not established.
7. **Lost-response reconciliation.** Drop the response after a committed synthetic operation. Retry/lookup with the same operation key and verify the original outcome is returned without a second external side effect.
8. **Principal/patient/tenant binding.** Mutate each binding dimension in negative controls. Verify the transaction is rejected and no side effect occurs.

The manifest at `research/fhir-transaction-adapter-qualification/manifest-v1.json` is the machine-readable source of required probe IDs and policy ceilings.

## The boundary that must not be blurred

A FHIR transaction can be atomic inside one FHIR server. It does not atomically include a different consent registry, Holochain DHT, Mycelix gateway database, or remote recipient. The Mycelix release transaction contract therefore remains capability-qualified:

- A same-server operation can be considered only if commit-time consent/revocation checks, version checks, idempotency reservation, audit intent, and outcome recording are proven inside the server's actual transaction boundary.
- A separate local authority ledger and remote FHIR endpoint require an outbox, stable operation identity, remote idempotency where supported, and reconciliation of unknown delivery outcomes.
- An outbox reduces the chance of losing work; it does not by itself make a remote side effect exactly once.
- If a remote endpoint cannot establish the result of a previous delivery, the operation remains deferred. Do not issue a new idempotency key as a workaround.

## Evidence freshness and drift

The v1 evaluator uses a maximum evidence age of 24 hours and a maximum future clock skew of 300 seconds. These are initial engineering policy values, not HL7 requirements. Tighten or relax them only through a versioned policy change with an explicit threat analysis.

The CapabilityStatement digest and software version are compared against the values embedded in the probe evidence. If either changes, the old probe evidence cannot qualify the new target. A real qualification controller should capture the endpoint response, canonicalize its exact bytes under a documented profile, compute SHA-256, and retain the original artifact with a signed observation envelope.

## Benchmark targets

These are qualification gates, not results already achieved:

| Dimension | Target |
|---|---:|
| Partial writes after failed transaction | 0 |
| Duplicate logical resources per idempotency key | 0 |
| Releases serialized after revocation | 0 |
| Stale-version commits | 0 |
| Missing audit intent for committed release | 0 |
| Repeated external side effects after lost acknowledgement | 0 |
| Required probe coverage | 100% |
| Evidence-to-server/CapabilityStatement binding | 100% |
| Stale or unknown evidence treated as qualified | 0 |

Measure latency, transaction size, probe cost, rollback behavior, throughput under 1/10/100/1000 contenders, and time-to-reconcile separately. Do not optimize speed by relaxing the non-compensatory safety gates.

## Running the evaluator

```sh
bash scripts/evaluate-fhir-transaction-qualification.sh \
  research/fhir-transaction-adapter-qualification/example-evidence-v1.json
```

Exit codes: `0` qualified, `2` not qualified, `3` defer, `64` usage/dependency error, `65` malformed input. Set `MYCELIX_NOW_EPOCH` to a fixed Unix timestamp for deterministic tests.

The included example intentionally returns `defer`: it has not been populated with actual server probe evidence.
