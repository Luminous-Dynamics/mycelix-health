# FHIR R4 Provenance, AuditEvent and Consent — Mycelix mapping v1

**Status:** standards-mapping proposal plus synthetic FHIR R4 core validator fixtures. No production ingestion, authorization, signing, or audit-store integration is claimed.

## Why three native resources

Mycelix must not collapse resource generation, resource use, and permission into one generic provenance edge or a `verified` boolean.

FHIR R4 gives these concepts distinct resources:

| Resource | Native question | Mycelix use |
|---|---|---|
| `Provenance` | What activity, entities and agents produced or influenced this resource version? | Source/derivation graph and reproducible resource lineage |
| `AuditEvent` | What security-relevant event occurred, who participated, and what was accessed? | Append-only access, disclosure, execution and policy-decision audit trail |
| `Consent` | Which recipients/actions/purposes/time constraints are represented in a consent choice? | Input to an independently enforced authority/consent registry |

The HL7 R4 Provenance specification describes generation/derivation context, while AuditEvent describes usage and other activity. AuditEvent records can be emitted by multiple participating actors; duplicate entries are expected and can expose gaps or inconsistencies in an enterprise workflow. Provenance can point to specific historical versions, and one target may have several Provenance records describing distinct activities.

Primary R4 references:
- Provenance: https://hl7.org/fhir/R4/provenance.html
- AuditEvent: https://hl7.org/fhir/R4/auditevent.html
- Consent: https://hl7.org/fhir/R4/consent.html

## 1. Provenance → evidence generation and derivation

Map the Provenance resource into typed evidence relationships:

| FHIR element | Required Mycelix interpretation |
|---|---|
| `target` | Exact generated/revised resource reference; preserve version-specific history reference when supplied |
| `occurred[x]` | Time/period the activity occurred, if known |
| `recorded` | When the Provenance assertion itself was recorded/updated |
| `agent.who` | Claimed participant identity; resolve independently to authenticated principal/organization/device registry |
| `agent.type` / `role` | Participant function, not proof of trust or authorization |
| `entity.role` + `entity.what` | Inputs to the activity, such as source, derivation, revision or quotation entity |
| `activity` | Activity classification, such as transform, ingest, generate or revise |
| `policy` | Policy identifier context, not proof that the policy was valid or satisfied |

### Invariants

- A Provenance target must resolve to an unambiguous resource identity. If version identity matters, retain the version reference exactly; do not sort opaque `meta.versionId` values numerically or lexically.
- Treat `recorded`, `occurred[x]`, source `meta.lastUpdated`, clinical event time, specimen/analysis/result time, and local ingestion time as distinct clocks.
- A FHIR reference and agent label are assertions. Verify issuer signatures, source registration, referenced payload digests, signer identity and authority independently.
- One target can have multiple provenance activities. Preserve the activity/revision chain; do not replace it with a single “latest provenance” record.
- A Provenance resource can itself need Provenance. Preserve attestation lineage rather than allowing the evidence graph to self-authorize.

## 2. AuditEvent → access and execution accountability

Use AuditEvent for security-relevant use of information and system activity, including reads, searches, exports, ingestion attempts, synchronization, authorization decisions, consent changes, conflict resolution and rejected requests.

Important R4 fields:
- `type` identifies the event category.
- `recorded` is the time the audit event was recorded.
- `agent` captures participants and whether each was the requestor.
- `source.observer` identifies the source that detected/reported the event.
- `purposeOfEvent` captures stated purpose of use.
- `entity` can identify resources involved in the event.
- `outcome` records event success/failure where represented.

### Invariants

- AuditEvent is not a Provenance substitute. Accessing a resource does not establish how the resource was created; creating a resource does not capture every access to it.
- Keep audit event records append-only in the Mycelix audit store. Corrections should be represented by additional linked events, not silent edits or deletion of prior events.
- Multiple application, gateway, source and conductor AuditEvents may represent the same user journey. Preserve actor/reporting-system identity and correlate them; do not collapse by timestamp or assume repeated entries are accidental duplicates.
- Bind an audit event to the actual authenticated principal, request ID, exact resource/version, decision receipt, recipient and purpose. The FHIR fields alone do not cryptographically prove those facts.
- AuditEvent content is security/privacy administrator data. Patient-facing access reports should be purpose-limited projections, not unrestricted raw audit log access.

## 3. Consent → policy input, never a self-authorizing token

FHIR R4 Consent expresses healthcare-consumer policy choices. In R4, the resource is currently scoped to the privacy use case even though broader uses are anticipated. Its enforcement mechanics are explicitly outside the Consent resource specification.

Required/core fields include:
- `status` and `scope`;
- one or more `category` entries;
- `patient` for the privacy scope (FHIR has a 0..1 element cardinality but an invariant requires a patient for privacy scope);
- `policyRule` or `policy` context;
- optional `provision` exceptions with time, actor, action, purpose and data constraints.

The R4 `dateTime` element means when the Consent resource was created/indexed. It is not automatically the time the underlying agreement was signed or became effective. Signature evidence may be represented by Provenance.

### Authorization gate above FHIR validity

A syntactically valid Consent must never directly grant access. The production authority resolver must independently establish:

1. the patient/opaque subject binding and tenant match;
2. the originating signed directive/authoritative source and applicable legal/policy domain;
3. whether the record is an authoritative directive or a derivative/index;
4. signer/representative identity and signature requirements for that domain;
5. current status, effective period, revocation, and supersession;
6. recipient identity and organization/client registration;
7. permitted action, resource categories, purpose of use and security labels;
8. applicable exceptions, delegations, emergency break-glass rules and obligations;
9. the exact source connection, data scope and downstream recipient;
10. consume-once or replay-safe authorization receipt semantics for the action.

An `active` code alone is insufficient. An `inactive`, `rejected`, `entered-in-error` or expired Consent must not be used as an active grant. Absence of an applicable directive is not permission. A Consent resource may be a derivative that lacks legal fidelity to the original signed directive; follow source references only through authorized pathways.

## 4. Relationship to the Mycelix evidence graph and Symthaea

Preserve separate edge families:
- **generation/derivation edges** from Provenance entity/target and activity records;
- **access/use edges** from AuditEvent and immutable execution receipts;
- **authority inputs** from Consent plus independently verified grant/registry state;
- **clinical semantic edges** (supports/contradicts/same assertion) from qualified clinical assertion evaluation, not from Provenance or AuditEvent alone.

Symthaea may reason over a minimum-necessary projection of verified evidence and may propose information acquisition. It must not receive raw patient identifiers, unrestricted AuditEvent logs, signing keys, hidden evaluator truth or the restricted source-reference manifest. A positive EVSI, prediction or provenance edge cannot authorize access.

## 5. Official validator fixture expansion

The pinned HL7 Validator CLI 7.0.6 / FHIR R4 4.0.1 corpus grows from 16 to 23 fully synthetic cases:

| Fixture | Expected FHIR core result | What it controls |
|---|---|---|
| `valid_provenance_version_specific_source_entity` | valid | version-specific target, agent, source entity |
| `invalid_provenance_missing_target` | invalid | required Provenance target |
| `valid_audit_event_with_agent_and_source` | valid | type, recorded, agent/requestor, source observer |
| `invalid_audit_event_missing_source` | invalid | required AuditEvent source reporter |
| `valid_active_privacy_consent_with_patient` | valid | privacy Consent and patient invariant |
| `valid_inactive_privacy_consent_not_an_authority_grant` | valid | FHIR syntax is not an active permission |
| `invalid_privacy_consent_missing_patient` | invalid | privacy Consent patient invariant |

The validator lane uses terminology mode `n/a`; its results qualify FHIR R4 structural/core invariants only. It does not validate terminology bindings, jurisdiction-specific profiles, signature/legal enforceability, source authenticity, consent enforcement or clinical truth.

## 6. Source-audited current implementation boundary

The current repository has partial native infrastructure, but not the complete FHIR mapping:

- `zomes/fhir_bridge/coordinator/src/lib.rs` handles Patient, Observation, Condition, MedicationRequest/Statement, AllergyIntolerance, Immunization, Procedure, DiagnosticReport and CarePlan. Other resource types are added to `IngestReport.unknown_types`; their raw Provenance, AuditEvent and Consent content is not persisted by that switch.
- `FhirResourceAnchor` currently holds source key, resource type, internal hash and first/last-ingestion timestamps. It does not itself carry source resource version or payload digest. The separate source-revision work must supply that missing version/digest authority.
- `zomes/consent/integrity` already defines patient-bound consent, data categories, permissions, grantee variants, date ranges, encounter restrictions and `DataAccessLog`.
- `zomes/consent/coordinator::check_authorization` currently checks active status/expiry/revocation, grantee branch, category, exclusions and permission. The inspected request DTO does not carry clinical event time or encounter context, so it cannot enforce `ConsentScope.date_range` or `encounter_hashes`. Provider and Organization grantee branches are explicitly fail-closed pending resolver/membership work. See [issue #333](https://github.com/Luminous-Dynamics/mycelix-health/issues/333).
- The existing `DataAccessLog` is useful but coarser than a complete FHIR AuditEvent: it does not natively bind exact resource ID/version/digest, request ID, intended recipient, purpose, or a typed success/failure outcome in the inspected schema.
- The shared chained-audit helper hashes a supplied `entry_hash`, while the coordinator entry point stores a caller-provided event description and digest in `DataAccessLog.access_reason`. The current path needs to recompute the canonical event digest, bind actor identity to the authenticated caller, and independently verify the persisted chain. See [issue #332](https://github.com/Luminous-Dynamics/mycelix-health/issues/332).

These are source-level implementation gaps, not proof of an exploited deployment. The new FHIR validator fixtures qualify R4 core syntax only and do not close these custody, authorization or audit gaps.

## 7. Remaining engineering work

- Add supported-resource ingestion and persistence for Provenance, AuditEvent and Consent; do not treat the gateway's current `z.any()` Bundle entry as semantic qualification.
- Preserve version-specific Provenance target refs and all independent clock fields in the source revision ledger.
- Build a source-attested Provenance verifier and immutable AuditEvent writer with independent replay/receipt controls, tracked in #332.
- Connect Consent indexes to the current grant/revocation registry and enforce date/encounter/resource/recipient/purpose scope, tracked in #333.
- Add multi-agent Holochain integration tests and independent security review.
- Add a terminology/implementation-guide lane separately from the current R4 core validator, whose mode `-tx n/a` does not prove terminology bindings or legal consent validity.

Synthetic validator fixtures only; no PHI. No clinical safety, legal compliance, production-readiness or SOTA claim follows from this increment.
