# Mycelix Care Disclosure Core

Reference semantics for CARE-DISC-001 (#171).

This crate is deliberately isolated from the production `mycelix-health` workspace. It freezes minimum-necessary disclosure and bounded Symthaea care-task invariants before any Holochain, cryptographic, UI, or model integration.

## Boundary

```text
CareVaultRecord
      |
      | explicit claims only
      v
DisclosureProjection
      |
      | finite, purpose/action bounded
      v
SymthaeaCareTaskEnvelope
      |
      | inference only, non-delegable
      v
TaskOutputReceipt
```

The API intentionally has no:

- wildcard `AllRecords` selector;
- vault/master key;
- root care capability;
- ambient Holochain capability;
- model-training switch on a care task;
- delegation switch on a care task;
- clinical diagnosis/order output authority for Symthaea.

## What the tests establish

The tests exercise representation/attenuation invariants including:

- non-empty explicit claim selection;
- purpose/action compatibility;
- duplicate rejection;
- finite validity windows;
- monotonic child projection attenuation;
- immutable source provenance/authority during attenuation;
- task inputs as a subset of the projection;
- care tasks cannot derive from research/model-training purposes;
- care tasks cannot outlive their projection;
- comparative spiritual tasks require an explicit spiritual-care purpose;
- care tasks are structurally non-delegable and inference-only;
- output receipts preserve task/projection/source provenance;
- high-authority clinical input never promotes AI output authority.

## Non-claims

A green test run does **not** establish encryption security, Holochain confidentiality, valid clinical consent, professional scope-of-practice, minimum-necessary compliance in any jurisdiction, clinical efficacy, or AI safety. Those require separate evidence lines.

## Follow-on

1. bind `VaultRecordId` to qualified ProtectedEnvelopeV2 identities;
2. bind `CapabilityRef` to a qualified CARE-CAP capability proof;
3. add an explicit derived-projection issuance timestamp and output-use receipt semantics if the v1 review accepts them;
4. integrate the local CareVault index in Leptos without publishing its semantic index;
5. adapt the task envelope to Symthaea's authority kernel through a narrow, provenance-preserving bridge.
