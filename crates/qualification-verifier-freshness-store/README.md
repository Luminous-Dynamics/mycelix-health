# Qualification Verifier Freshness Store V1

This standalone crate implements the local crash/restart durability half of QUAL-EVID-009C2 / #233.

It is intentionally a **local consistency + durability theorem**, not a complete adversarial rollback/tamper theorem.

## State model

The store is an append-only logical event chain persisted as one atomically replaced file.

Event classes:

```text
Genesis
IssueChallenge
ConsumeChallenge
RotatePolicy
AdvanceTime
```

Every event binds:

```text
store lineage
contiguous sequence
exact predecessor event digest
non-regressing observed security time
event-specific payload
```

The current event digest transitively commits the complete preceding local history.

Derived state contains:

```text
lineage id
sequence
head digest
security-time floor
policy generation
current hybrid key-lineage commitment
active challenges
consumed challenge tombstones
```

## Durable write ordering

The qualified Linux reference uses:

```text
encode + replay candidate history
        ↓
write same-directory temp file
        ↓
fsync(temp file)
        ↓
atomic rename over current store
        ↓
fsync(parent directory)
        ↓
publish new in-memory state
```

`consume_challenge()` completes this durable commit before returning `Fresh` or `Expired`. A caller can therefore order producer work as:

```text
durable consume
→ request/evidence verification
→ signing
```

A normal crash after durable consumption but before verification/signing must reload the challenge as consumed.

## Load verification

Every load reconstructs state from the full event history and rejects:

- unknown file/event schema;
- empty/oversized history;
- wrong lineage;
- non-contiguous sequence;
- wrong predecessor digest;
- stored event-digest mismatch;
- time regression;
- challenge nonce reuse/replay;
- malformed challenge windows;
- policy-generation rollback/skip;
- key-lineage-preserving 'rotation';
- truncated or trailing bytes.

## External anchor identity

The exact local state intended for later anti-rollback witnessing is represented by `FreshnessStoreHeadV1`:

```text
lineage_id
sequence
head_digest
security_time_floor_micros
policy_generation
key_lineage_commitment
```

`head_digest` transitively commits all earlier store events, while the remaining fields are explicit semantic state for the Health anti-rollback adapter. A follow-up adapter should canonicalize these six fields into the Health #216/#221 generic monotonic-witness state commitment rather than interpreting the store file directly.

## Deliberate whole-volume rollback demonstration

The test corpus explicitly saves an older valid store image, advances/consumes the live state, restores the older bytes and demonstrates that the local store accepts the old image as internally valid.

That is **not a defect hidden by the tests**. It freezes the boundary:

```text
hash-chained + fsynced local state
!=
proof that a newer state never existed
```

Production mint authority therefore requires a separately retained monotonic witness / hardware counter / independent checkpoint authority.

## Local malicious-writer boundary

Event digests are unkeyed SHA-256 commitments. A malicious actor with arbitrary write access to the store can construct a new self-consistent history. V1 detects corruption and internal inconsistency; it does not authenticate the local writer.

Process isolation, filesystem permissions and external witnessing remain separate authority layers.

## Single-writer boundary

V1 does not implement multi-process locking or compare-and-swap coordination. Exactly one verifier instance must own a store lineage at a time. Concurrent writers can otherwise race from the same predecessor and `rename` is not a consensus protocol.

The daemon/process tranche must prove exclusive store ownership before using this reference in production.

## Portability

The implementation is compiled only on Unix-like targets for the persistence path. The dedicated qualification lane runs on Ubuntu Linux and can establish only that exact platform theorem. No Windows/macOS/filesystem portability claim follows automatically.

## Scalability

Consumed nonces are retained as tombstones and history is replayed in full. V1 intentionally prefers simple replay safety over unqualified compaction. Checkpoint/compaction requires a future theorem that preserves replay history and external-anchor continuity.

## Privacy

The store contains only verifier freshness/policy metadata and opaque commitments/nonces. It requires no patient identifier, PHI, care payload, therapy/spiritual content or raw qualification prose.
