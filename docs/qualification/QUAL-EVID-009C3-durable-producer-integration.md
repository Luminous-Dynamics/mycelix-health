# QUAL-EVID-009C3 — Durable freshness / producer integration

## Purpose

Compose the source-staged isolated producer (#232 / 009C1) with the restart-durable freshness store (#234 / 009C2) without modifying either parent theorem.

## Required ordering

For mint requests, the composition must perform:

```text
validate local durable-store / signer policy identity
→ durable consume exact request challenge
→ fsync/rename/fsync completion inside freshness store
→ require Fresh consumption status
→ delegate to #232 producer
→ #232 consumes its process-local mirror
→ request/evidence checks
→ real hybrid signing/self-verification
→ provider-attested receipt
```

No verification or signing may occur before durable challenge consumption succeeds.

## Challenge issuance

The wrapper may obtain a randomly generated challenge from #232 and then persist the exact nonce/window into #234 before returning it to the caller.

If durable persistence fails, the challenge must not be returned. The process-local mirror may remain allocated but is unreachable through the wrapper and must not create an authority fallback.

## Restart profile

V1 is deliberately fail-closed across restart:

- consumed challenges remain consumed through #234;
- outstanding challenges from the prior process are **not** reconstructed into #232's in-memory store;
- therefore pre-restart outstanding challenges become unusable after restart and callers must request a fresh challenge.

This sacrifices availability rather than reconstructing process-local authority from local disk without a separate theorem.

## Policy/key-lineage coherence

At wrapper construction and before challenge issue/mint, require the freshness-store head to equal the exact producer identity on:

- policy generation;
- hybrid key-lineage commitment.

Mismatch denies. Store rotation and signer rotation must be coordinated by a later explicit API.

## No fallback

The composition API must never expose #232's raw producer or an in-memory-only challenge path. Selecting this wrapper means durable freshness is mandatory.

## External anti-rollback dependency

This composition establishes local crash/restart ordering only. It does not make #234 adversarially rollback-resistant.

Production mint authority still requires #216/#221 generic external witnessing (or separately qualified hardware monotonic state) over the #234 head. If that currentness proof is unavailable, production daemon authority must be unavailable.

## Non-claims

This tranche does not establish:

- full daemon transport/process identity;
- exclusive OS-level process locking;
- hostile local filesystem integrity;
- whole-volume rollback resistance;
- HSM/TPM key custody;
- product-side 009D verification/token mint;
- Patient-v2 activation.
