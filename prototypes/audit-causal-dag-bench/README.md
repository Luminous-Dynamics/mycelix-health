# Per-Agent Clinical Audit Streams + Causal DAG Benchmark v1

**Status:** standalone dependency-free Rust reference verifier and synthetic structural benchmark. It is not integrated into the Consent coordinator, does not validate Holochain signatures, and is not a production audit service.

## The paradigm shift

Do not manufacture one patient-global sequence for independent writers.

Holochain's source chain gives each agent its own signed append-only order. Within one stream, order comes from that stream's sequence and previous-action link. Across agents, an event is causally after another event only when it carries an explicit parent ActionHash. Two events on unrelated streams with no causal path remain **incomparable**, even when one has an earlier timestamp.

This is a partial-order model:

- Per-agent source-chain continuity is checked independently.
- Cross-agent dependencies are explicit parent edges.
- Patient and tenant boundaries are checked on every parent edge.
- A deterministic topological order is available for display only; it does not assert a total causal order.
- A missing parent or unavailable stream suffix produces `Incomplete`, never `Verified`.
- Contradictory identity, chain, checkpoint, scope or cycle evidence produces `Rejected`.

The model preserves local availability without inventing global consensus for facts that do not require a total order.

## Event and checkpoint contract

Each synthetic event includes:

- source ActionHash, stream identity, claimed author, per-stream sequence and previous ActionHash;
- patient and tenant scope;
- optional resource type, logical ID, version and content digest;
- event kind, request ID, authorization receipt, policy version, purpose, recipient, action and outcome;
- clinical event time and local observation time as separate metadata;
- explicit causal parent ActionHashes.

Each `TrustedStreamCheckpoint` binds a stream to an authenticated author and an expected terminal sequence/ActionHash. In production this value must be obtained from an independently trusted, signed checkpoint or receipt. The event set must not nominate its own trust anchor.

The adapter must independently verify Holochain action signatures, actual source-chain authorship, ActionHash identity and checkpoint signatures before constructing the trusted input. This prototype only checks that the supplied event author agrees with the supplied trusted checkpoint and that the source-chain/DAG relationships are internally consistent. It does not implement cryptography, Holochain calls, DHT publication or checkpoint persistence.

The prototype normalizes a stream to a sequence beginning at zero. A Holochain adapter must explicitly map the relevant source-chain action range and its checkpoint into this representation; it must not quietly omit a prefix or invent continuity.

## Verification semantics

| Condition | Result |
| --- | --- |
| All required stream actions and checkpoints are present; stream links, authors, scope and DAG are consistent | `Verified` |
| A checkpoint, stream action or causal parent is not yet visible | `Incomplete` |
| Conflicting ActionHash, wrong author, scope-crossing parent, broken predecessor, stale/uncheckpointed suffix, checkpoint mismatch or cycle | `Rejected` |

An incomplete view can become verifiable when peers publish missing evidence. It cannot be used as a verified complete audit history in the meantime.

Timestamps are deliberately excluded from the graph algorithm. Clock skew, identical timestamps, late publication, backfill and offline agents do not create causal edges. A wall-clock sort or stable display tie-break may be useful for a user interface, but neither is causal evidence.

## Frozen public challenge corpus

The 21-row TSV catalogue is executable: each row maps to a concrete synthetic graph and the tests assert the expected status and finding. Cases cover independent two-agent streams, explicit parent edges, out-of-order publication, clock skew, equal timestamps, missing checkpoints/actions/parents, author mismatch, cross-patient and cross-tenant links, predecessor mismatch, duplicate ActionHash observations, conflicting checkpoints, duplicate per-agent sequence numbers, checkpoint mismatch, uncheckpointed suffix, duplicate parent, self-parent and cycles.

The suite also checks:

- identical replicated observations coalesce without duplicating events;
- conflicting observations for one ActionHash are rejected;
- deterministic display output is invariant to input publication order;
- incomparable-pair counts stay separate from display ordering;
- a trusted head detects a missing terminal suffix as an incomplete view.

The public catalogue is a regression corpus, not a hidden security leaderboard. It contains no PHI.

## Run

From the repository root:

```bash
bash ./scripts/run-audit-causal-dag-benchmark.sh
```

The crate is isolated from the Holochain workspace and has no third-party dependencies. CI runs formatting, Clippy and tests in an independent job.

## Research benchmark targets (not achieved claims)

- 0 accepted wrong-author events after real Holochain signature verification.
- 0 accepted causal edges crossing patient or tenant scope.
- 0 accepted cyclic causal graphs.
- 100% detection of altered ActionHash payloads and mismatched trusted checkpoints in a production adapter conformance suite.
- 100% of missing-parent and missing-stream cases reported as incomplete, never complete.
- 0 duplicate side effects for the same idempotency key; this is an execution-adapter property and is not tested by this graph-only crate.
- Report stream verification latency, peer convergence time, unresolved-parent age, checkpoint freshness, causal-edge coverage and incomparable-pair count separately.
- Stress 2, 8, 32 and 128 agents with concurrent appends, moved-head races, offline partitions and out-of-order DHT publication.

Do not count correlated clock-skew and publication permutations as independent statistical samples. For a zero-failure claim, use truly independent stream/action fixtures and report confidence bounds separately from metamorphic regression coverage.

## Relationship to Mycelix Health work

- [#337](https://github.com/Luminous-Dynamics/mycelix-health/pull/337) provides a shared per-chain payload hash and expected-head verifier, but does not integrate the Consent coordinator or authenticate its actor.
- [#332](https://github.com/Luminous-Dynamics/mycelix-health/issues/332) tracks full payload, authenticated actor, append, idempotency and checkpoint integration.
- [#338](https://github.com/Luminous-Dynamics/mycelix-health/issues/338) describes the multi-agent topology and distributed acceptance tests this benchmark supports.

The next production increment must bind the actor to the authenticated Holochain call context, append to that actor's current chain head with moved-head reconciliation, persist independent checkpoints, and publish a receipt that distinguishes local commit, DHT publication, independent observation, verification and checkpointing.

No legal compliance, clinical safety, security certification, source-authenticity or production-readiness claim follows from this prototype.
