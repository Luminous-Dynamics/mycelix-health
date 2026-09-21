# QUAL-EVID-009C2 Freshness Store Boundaries

## A successful exact-head qualifier may establish

- deterministic V1 freshness event/file framing;
- exact event-digest predecessor chaining;
- contiguous per-store event sequencing;
- replay-derived active/consumed challenge state;
- consume state is persisted before success is returned;
- normal crash/restart preserves consumed challenge tombstones;
- challenge expiry persists across restart;
- security-time floor persists and cannot regress through normal API/replay;
- verifier policy generation advances exactly one generation at a time;
- policy rotation must change the hybrid key-lineage commitment;
- file/event corruption and internal chain inconsistency fail closed;
- temp-file-only simulated crash does not replace the last durable main file;
- exact restart reconstructs the same `FreshnessStoreHeadV1`;
- Ubuntu/Linux `write -> fsync(file) -> rename -> fsync(directory)` execution for the tested filesystem/runner.

## It does not establish

- adversarial whole-volume rollback resistance;
- malicious local writer authenticity;
- keyed/MACed or signed local history;
- multi-process writer exclusion;
- distributed consensus;
- trusted production time;
- TPM/HSM/secure-element monotonicity;
- generic transparency/witness correctness;
- cross-platform filesystem durability;
- HSM/private signing-key custody;
- daemon endpoint/process authentication;
- construction of #232 verifier-owned evidence;
- producer/provider receipt validity;
- product-side authority or #224 token minting;
- #208 cutover / Patient-v2 activation.

## Whole-volume rollback rule

A restored older but internally valid complete store may be accepted by V1. This is intentionally demonstrated by a source test. Before the freshness head can support production mint authority, the current head must be reconciled with a separately retained monotonic authority such as Health #216/#221, hardware monotonic state, or an independently administered checkpoint service.

Failure to establish external currentness must reduce availability, never increase authority.

## Malicious local writer rule

SHA-256 event chaining detects corruption and inconsistent predecessor/sequence histories but does not authenticate the writer. An attacker with arbitrary write access can recompute a replacement self-consistent history. Filesystem/process isolation plus external witnessing remain required for an adversarial theorem.

## Single-writer rule

The store is single-writer V1. A production daemon must enforce exclusive ownership of one store lineage. Concurrent writers racing from one predecessor are outside this theorem.

## External anchor handoff

The future canonical Health state commitment must bind all six `FreshnessStoreHeadV1` fields:

```text
lineage_id
sequence
head_digest
security_time_floor_micros
policy_generation
key_lineage_commitment
```

No subset is sufficient for the anti-rollback handoff.

## Compaction rule

V1 has no history compaction. Consumed challenge tombstones remain replay-visible. Any future compaction/checkpoint must preserve nonce replay safety and exact external-anchor continuity and receive its own qualification.
