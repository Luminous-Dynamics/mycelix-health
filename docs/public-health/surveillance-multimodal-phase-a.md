# Multimodal Defensive Surveillance — Phase A

Status: specification / adversarial-corpus preparation only.

Tracks: #241–#250. Cross-repo analytical consumers: `Luminous-Dynamics/symthaea#135`, `#203`, `#205`, `#5558`, `#5559`.

## Purpose

Phase A freezes semantics and adversarial fixtures before any new live provider integration or new runtime authority. It composes the existing public-health surveillance stack rather than creating a parallel biohazard subsystem.

## Core architecture

```text
mycelix-health
  evidence identity / release / authority / privacy
  replay / lineage / corrections / assurance
  multimodal source semantics
  coverage / measurement / ethics / event evidence
          |
          v
Symthaea
  semantic series binding
  sampling admissibility
  change / persistence / corroboration
  evidence settling
  competing hypotheses / PHI fusion
          |
          v
external human / institutional authority
```

## Phase-A invariants

```text
measurement != hazard
statistical anomaly != outbreak
pathogen marker detected != viable/infectious agent
sampled population != target population
below detection limit != zero
not measured != not detected
article != event
article volume != independent-event count
privacy admissible != ethically proportionate
aggregate genomic summary != phenotype inference
recommendation != authorization
```

No universal `BiohazardScore`, `TrustScore`, or scalar may flatten evidence identity, authenticity, privacy, ethics, sampling, coverage, lineage, hypothesis support, or operational authority.

## Canonical ownership

- Existing aggregate surveillance evidence remains owned by `mycelix-health` #10–#18.
- #23 owns replay-safe semantic ingestion.
- #24 owns typed claim promotion.
- #26 owns provenance/lineage relationships.
- #27 owns correction/retraction/supersession.
- #29 owns explicit disconnected assurance.
- #241 owns additional multimodal source-profile classification.
- #242 owns coverage and representativeness evidence.
- #243 owns detection/quantification/censoring semantics.
- #244 owns proportionality/equity/stewardship review evidence.
- #245 owns non-numeric event-based public-health intelligence candidates.
- #247 owns the aggregate genomic-surveillance summary boundary.
- #248 owns closed-world multimodal composition qualification.
- #249 owns implementation sequencing.
- #250 owns the shared synthetic adversarial corpus.
- Symthaea #5558 owns indicator + event evidence fusion at the competing-hypothesis layer.
- Symthaea #5559 owns evidence-settling composition.

## Development order

### Gate 0 — qualify the existing foundation

Do not let any higher green result launder an unqualified lower dependency.

```text
#10 -> #11 -> #12 -> #13 -> #14 -> #15 -> #16 -> #18
```

### Phase A — specifications and fixtures

Phase A may add only documentation, fixtures, static validators, or test-vector definitions that do not change existing evidence identities or introduce live I/O.

### Phase B — pure semantic crates

After Gate 0 is dependency-qualified:

```text
#242 coverage
-> #243 measurement-state semantics
-> #241 multimodal profile mappings
-> #244 ethics/governance semantics
-> #245 event-signal semantics
-> #247 aggregate genomic summaries (separate/optional)
```

### Phase C — bridge prerequisites

Finish replay/correction/lineage/claim-promotion and then Symthaea #205/#203/R2/R4.

### Phase D — closed-world reasoning

Implement #5559, #5558 and #248 using only synthetic fixtures.

### Phase E — one bounded live adapter

Only after exact closed-world composition passes, introduce one anonymous/read-only public aggregate source with exact provider/profile semantics, strict request/page/byte/time limits, existing OPSEC disclosure authority, execution receipts, immutable capture provenance, no automatic knowledge admission, and no publication/action authority.

## Minimal-evidence review doctrine

Every PR should answer:

1. What exact proposition does this change establish?
2. What nearby stronger propositions does it explicitly not establish?
3. Which existing primitive is reused instead of duplicated?
4. Which `Unknown` states remain first-class?
5. Which hostile substitution would reveal accidental claim promotion?
6. What is the smallest reversible next step when evidence is insufficient?
7. Does any output accidentally grant collection, publication, clinical, emergency, or operational authority?

Prefer the smallest sufficient claim. New evidence should remain provisional until the profile-required persistence, currentness/correction, dependency, coverage and contradiction checks have been evaluated.

## Safety boundary

This program does not include pathogen engineering/design, virulence/transmissibility/host-range optimization, immune/countermeasure evasion, culture/propagation procedures, autonomous sample handling, raw sequence processing in the public surveillance path, diagnosis, treatment recommendation, autonomous emergency declaration, or automated restrictions on people.

## Claim ceiling

A future Phase-A PASS freezes exact semantics, fixtures, expected classifications and review rules only. It does not establish epidemiological performance, source truth, pathogen identity, outbreak status, clinical validity, legal compliance, universal ethical acceptability, deployment readiness, or operational authority.
