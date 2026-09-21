# Durable qualification-verifier mint producer

This crate implements the QUAL-EVID-009C3 composition theorem between:

- `mycelix-qualification-verifier-mint-producer` (#232 / 009C1), and
- `mycelix-qualification-verifier-freshness-store` (#234 / 009C2).

It is intentionally a composition layer, not a replacement implementation of either parent.

## Required mint ordering

```text
store/signer policy coherence
→ durable challenge consume
→ durable fsync/rename/fsync completes
→ require Fresh status
→ delegate to real #232 producer
→ request/evidence validation
→ hybrid signing/self-verification
→ provider-attested receipt
```

The wrapper has no in-memory-only fallback path.

## Challenge issue ordering

The lower producer creates the random nonce first. The exact nonce/window is then persisted through #234. The challenge is returned only after durable persistence succeeds.

If persistence fails after the process-local nonce was created, the wrapper becomes unusable for that operation path and never returns the hidden nonce. A later hardening may make this an explicit permanent fail-stop flag for all subsequent operations if operational testing demonstrates that is preferable.

## Restart profile

V1 deliberately does not reconstruct #232's private in-memory challenge map from disk.

Therefore an outstanding challenge issued before a process restart may remain represented in #234 but cannot silently regain process-local authority. The caller requests a new challenge.

Consumed challenges remain consumed across normal restart.

## Policy/key-lineage coherence

The wrapper compares #234's durable head with the exact producer identity on construction and before every issue/mint:

- policy generation;
- hybrid key-lineage commitment.

Mismatch denies.

## External anchor

`freshness_head()` exposes only the opaque #234 head for the separate #216/#221 monotonic-witness layer. Local durability does not prove whole-volume rollback resistance.

## Evidence-constructor boundary

The real `mint(...)` takes #232's `VerifierOwnedMintEvidenceV1`, whose fields remain private and which has no public production constructor in the current stack.

This child does not add a test or application backdoor. The first positive end-to-end real-evidence mint must come from the concrete verifier-evidence integration.

## Non-claims

This crate does not establish:

- independent PASS status for #232 or #234;
- external anti-rollback currentness;
- exclusive OS process locking;
- trusted wall-clock provenance;
- endpoint/peer/capability authentication;
- HSM/TPM/secure-element key custody;
- 009D consumer verification/private token mint;
- Patient-v2 activation.
