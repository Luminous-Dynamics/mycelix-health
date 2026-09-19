# Mycelix Health: Engineering Evidence Status

**Last updated**: 2026-09-19  
**Status**: ACTIVE SECURITY / PRIVACY HARDENING — NO REGULATORY COMPLIANCE CLAIM

This document tracks engineering controls and their **evidence state**. It is not a legal determination that Mycelix Health complies with HIPAA, 42 CFR Part 2, GDPR, POPIA, or any other law.

A source file, function name, passing unit test, or feature flag is not by itself sufficient evidence that a control works for the packaged/deployed system.

## Evidence vocabulary

| State | Meaning |
|---|---|
| **P0 BLOCKED** | Source review found a security/privacy property that must be repaired before the control can be relied upon. |
| **SOURCE PRESENT / UNQUALIFIED** | Relevant implementation exists, but exact packaged/runtime evidence is incomplete or not yet executed. |
| **SOURCE-STAGED / CI QUEUED** | A dedicated qualification subject exists, but its evidence lane has not yet produced an executable PASS. |
| **PARTIAL** | Some required properties are implemented, while adjacent properties or migration/deployment evidence remain open. |
| **NOT ASSESSED** | No current engineering or legal conclusion is made. |
| **NOT IMPLEMENTED** | The required product/control path is not present. |

A future **QUALIFIED** label should name the exact source/WASM/DNA/toolchain/evidence lineage it covers.

---

## Root privacy and authority blockers

### P0 — packaged Tier-1 PHI visibility

The current standard `health` DNA packages `patient`, `provider`, `records`, `prescriptions`, `consent`, and `bridge`. Source review found multiple public-by-default app-entry types containing patient-linked health, identity, relationship, prescription, consent, audit, and federation detail.

This means coordinator authorization checks must **not** be treated as a confidentiality boundary for those public DHT entries.

Tracked by:

- #164 — unify Tier-1 PHI storage privacy;
- #157 — consent/audit metadata privacy;
- #159 / #161 — metadata-minimized ProtectedEnvelopeV2;
- #169 — 42-entry packaged Tier-1 storage inventory and drift gate.

The `records` zome already has a stronger production direction: legacy plaintext PHI is blocked by default and the encrypted-record path authenticates its clear routing metadata. That does not repair plaintext/public storage in other packaged zomes or the metadata leakage of the v1 encrypted envelope.

### P0 — admin authorization fail-open

`require_admin_authorization()` currently allows an admin-gated operation when the consent-zome admin check cannot be reached/decoded, under a bootstrap fallback.

Authorization dependency failure must reduce availability, never increase authority.

Tracked by #176.

### P0 — chained audit evidence integrity

The current chained-audit path can:

- treat decode/unexpected response failures as an empty/genesis chain;
- derive sequence from total access-log count rather than verified chained predecessors;
- choose a predecessor from collection position rather than explicit chain identity;
- ignore failure to persist the chain-extension metadata after the ordinary log succeeds.

Therefore the existing SHA-256 code does **not yet establish a qualified append-only tamper-evident audit theorem**.

Tracked by #178.

---

## 42 CFR Part 2 — engineering mapping only

Legal applicability and sufficiency require independent counsel/review. The table below only maps current software evidence.

| Control area | Engineering evidence state | Current evidence / blocker |
|---|---|---|
| Consent tracking | **P0 BLOCKED / SOURCE PRESENT** | Consent types and category/purpose scopes exist, but detailed consent policy is public-by-default in the packaged consent zome. #157/#164/#163. |
| Consent revocation | **PARTIAL / UNQUALIFIED** | `revoke_consent()` exists. Comments describe downstream decryption-grant propagation, but the current proxy-reencryption/grant path is not a demonstrated cryptographic revocation mechanism. #163. |
| Encryption at rest / distributed storage | **P0 BLOCKED / PARTIAL** | `records::EncryptedRecord` provides a client-side AEAD route, but patient/prescription/consent/bridge PHI storage remains a root P0 and v1 envelope metadata is revealing. #164/#159. |
| Segregated sensitive-category access | **SOURCE PRESENT / UNQUALIFIED** | Sensitive-category checks reject blanket `All` consent and fail closed on transport/decode failures in the reviewed shared path. Exact packaged multi-agent qualification remains pending. |
| Audit integrity | **P0 BLOCKED** | Chained-audit source exists but continuity/persistence/failure semantics are not qualified. #178. |
| Re-disclosure prevention | **SOURCE PRESENT / UNQUALIFIED** | `check_redisclosure()` is fail-closed when consent verification cannot be completed; full packaged/adversarial evidence remains pending. |
| Breach/anomaly detection | **SOURCE PRESENT / UNQUALIFIED** | Detection code exists; no current claim is made that it satisfies legal breach-notification obligations or has complete runtime qualification. |
| Minors / guardian controls | **PARTIAL / UNQUALIFIED** | Source contains guardian/sensitive-category restrictions, but jurisdiction-specific semantics and multi-agent qualification are incomplete. |

---

## HIPAA — engineering mapping only

This is not a HIPAA compliance determination.

| Control area | Engineering evidence state | Current evidence / blocker |
|---|---|---|
| Transport confidentiality | **NOT ASSESSED / UNQUALIFIED** | Do not infer end-to-end PHI confidentiality solely from transport/runtime stack. Application-layer storage/disclosure controls remain independently required. |
| PHI storage confidentiality | **P0 BLOCKED / PARTIAL** | Encrypted records path exists, but current packaged Tier-1 plaintext/public entry surfaces prevent a global confidentiality claim. #164. |
| Access control | **P0 BLOCKED / PARTIAL** | Ordinary consent checks are largely fail-closed in reviewed code, but admin authorization contains a fail-open bootstrap fallback. #176. |
| Audit logging | **P0 BLOCKED** | Ordinary logs exist; append-only chained integrity is not yet established. #178. |
| Minimum-necessary disclosure | **P0 BLOCKED / SOURCE-STAGED** | Category-scoped consent exists, but public PHI storage and lack of a qualified projection layer prevent a complete minimum-necessary theorem. CARE-DISC #171/#173. |
| Access / amendment workflow | **SOURCE PRESENT / UNQUALIFIED** | Amendment request/processing code exists; protected storage, authorization, provenance and packaged workflow evidence remain to be qualified. |
| Breach/anomaly detection | **SOURCE PRESENT / UNQUALIFIED** | Detection source exists. Legal breach determination/notification is separate from anomaly detection. |

---

## GDPR / data-protection engineering mapping only

No GDPR compliance determination is made here.

| Control area | Engineering evidence state | Current evidence / blocker |
|---|---|---|
| Lawful basis | **NOT ASSESSED** | Requires legal/product governance review. |
| Data minimization | **P0 BLOCKED / SOURCE-STAGED** | ProtectedEnvelopeV2 and CARE-DISC are being staged specifically to reduce payload/metadata exposure and create explicit selective disclosure. #159/#171. |
| Erasure / revocation | **PARTIAL / UNQUALIFIED** | Revocation/key-rotation concepts exist. Do not claim cryptographic recall of plaintext/keys already legitimately received by another party. |
| Data portability | **SOURCE PRESENT / UNQUALIFIED** | FHIR export code/UI exists; exact scope, provenance, authorization and interoperability evidence remains to be qualified. |
| Privacy by design | **ACTIVE HARDENING** | Local vault wrapper, encrypted-record path, consent controls and privacy research exist, but #164/#176/#178 block a completed privacy-by-design claim. |
| DPIA | **NOT ASSESSED / NOT RECORDED HERE** | Product/legal process evidence is outside this engineering status file unless explicitly attached. |

---

## Current security/privacy foundations worth preserving

These are meaningful implementation assets, but should not be promoted beyond their demonstrated scope.

### Encrypted record path

- Client-side XChaCha20-Poly1305 protected-record path exists in `records`.
- Clear routing metadata is authenticated as AEAD associated data.
- Production defaults block legacy plaintext PHI in the records zome unless explicit dangerous migration features are enabled.
- ProtectedEnvelopeV2 (#159/#161) is intended to reduce semantic/patient metadata exposure rather than duplicate the encryption stack.

### Client vault wrapper

The Leptos portal currently has a v2 local vault wrapper with:

- browser cryptographic randomness;
- XChaCha20-Poly1305 authenticated wrapping;
- passphrase-derived local wrapping key;
- authenticated metadata;
- zeroizing secret handling;
- fail-closed handling of the obsolete XOR wrapper.

The remaining in-memory lifecycle boundary is tracked by #174/#175: UI `Unlocked` state must correspond to a real finite zeroizing session, not a discarded key plus boolean-like UI state.

### Sensitive-category authorization

Reviewed ordinary/sensitive authorization calls handle zome-call transport/authentication/decode failure as denial rather than permission. That fail-closed behavior is the model that admin authorization should follow.

### Re-disclosure guard

The reviewed shared `check_redisclosure()` path fails closed when it cannot verify the required re-consent. Its legal scope and exact packaged behavior still require separate qualification.

---

## Source-staged qualification subjects

The following narrow evidence lines are currently staged. A PR being open or CI being queued is **not a PASS**.

| Subject | Purpose | Current evidence state |
|---|---|---|
| #161 ProtectedEnvelopeV2 | Metadata-minimized public envelope / encrypted fine semantics | **SOURCE-STAGED / CI QUEUED** |
| #169 Tier-1 PHI inventory | Classify every packaged Tier-1 `EntryTypes` variant and reject drift | **SOURCE-STAGED / CI QUEUED** |
| #170 consent privacy characterization | Multi-agent canary for ungranted consent retrieval | **SOURCE-STAGED / IGNORED / UNEXECUTED** |
| #173 Care Disclosure Core | Explicit minimum-necessary projections + bounded Symthaea tasks | **SOURCE-STAGED / CI QUEUED** |
| #175 Vault Session Core | Finite zeroizing in-memory vault-key session | **SOURCE-STAGED / CI QUEUED** |
| #177 Care Capability Core | Private attenuable authorization, revocation and capsule-binding semantics | **SOURCE-STAGED / CI QUEUED** |

Qualification results must be updated here only after the exact run has executed and its subject identity/evidence has been reviewed.

---

## AI / Symthaea care boundary

Current architectural target:

- Symthaea never receives a patient's root vault key or root care capability;
- care tasks consume a minimum-necessary `DisclosureProjection`;
- tasks are short-lived and non-delegable;
- care inference does not imply model-training/research permission;
- AI outputs remain explicitly AI-origin (`AiDraft`, `AiHypothesis`, `EvidenceSummary`, etc.);
- professional acceptance/modification creates a new provenance-bearing human assertion rather than rewriting AI origin;
- spiritual/pastoral interpretation remains attributed and must not be promoted into clinical or metaphysical fact by the AI.

Reference semantics are tracked by #171/#173 and #163/#177. Runtime/model safety requires separate CARE-VAIL-style evidence.

---

## Evidence still required before stronger claims

1. **Tier-1 storage migration** — patient, prescriptions, consent/audit and bridge patient-linked data moved to explicitly reviewed public/private/protected representations.
2. **Admin fail-closed repair + explicit bootstrap** — #176.
3. **Audit-chain redesign and concurrency qualification** — #178.
4. **ProtectedEnvelopeV2 execution + crypto adapter** — representation semantics first, then standard KDF/AEAD/ML-KEM/key-wrap qualification.
5. **Care capability signature/identity adapter** — #163/#177 currently model semantics, not signature verification.
6. **Leptos vault-session integration** — UI state, timeout, lock, destruction and record hydration bound to the qualified session core.
7. **Multi-agent conductor tests** — exact packaged DNA/WASM identities, including direct peer visibility and authorization failure injection.
8. **Migration/deployment evidence** — determine whether affected v1 entry types exist in any real/test network and document non-recall limits.
9. **Independent cryptographic/security review** — after exact implementations stabilize.
10. **Independent regulatory/legal review** — separate from engineering qualification.
11. **Accessibility and clinical workflow validation** — independent product evidence lines.

---

## Recommendation

Treat Mycelix Health as an actively hardened research/product system with substantial security/privacy infrastructure, **not as an implementation-complete compliance system**.

Near-term priority is to close #164, #176 and #178, then qualify the narrow reference cores and migrate one packaged domain at a time. Marketing, regulatory filings, pilot agreements and security documentation should cite exact evidence scopes rather than broad labels such as “HIPAA compliant,” “Part 2 compliant,” or “immutable audit trail” until independent qualification supports those claims.
