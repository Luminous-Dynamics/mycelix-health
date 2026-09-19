# MH-001 Implementation Plan

Parent semantic contract: `docs/mental-health/INSTRUMENT_SEMANTICS_V1.md`

## Narrow first implementation

The first executable change should remove the generic fallback severity mapping from `interpret_score(...)` and make unsupported instruments explicit.

Recommended shape:

```rust
pub enum ScoreInterpretation {
    Scored {
        severity: Severity,
        interpretation: String,
        follow_up_recommended: bool,
    },
    Unsupported {
        reason: String,
    },
}
```

`create_screening(...)` should refuse to persist a fabricated scored interpretation for an unsupported instrument. A later tranche can preserve unscored response sets in a dedicated entry type if desired.

## Explicitly supported v1 scorers

The current implementation contains explicit scoring branches only for:

- PHQ-9;
- GAD-7;
- PHQ-2;
- AUDIT;
- DAST-10;
- CAGE.

PCL-5, MDQ, EPDS, PSC-17, C-SSRS and `Custom` must not fall through to a generic severity mapping.

C-SSRS should receive a separate triage/safety semantic contract rather than being treated as a summed generic severity score.

## Test requirements

Add focused tests for:

- PCL-5 => unsupported until exact scorer/spec is implemented;
- MDQ => unsupported;
- EPDS => unsupported;
- PSC-17 => unsupported;
- C-SSRS => unsupported by generic scorer;
- Custom => unsupported;
- existing explicit PHQ-9/GAD-7/PHQ-2/AUDIT/DAST-10/CAGE behavior remains unchanged for this narrow tranche.

## Follow-up

After this fail-closed patch is qualified, introduce `InstrumentSpecRef`, scorer identity, applicability metadata and versioned scoring registries in a separate child PR. Do not combine the immediate safety repair with a broad storage-schema migration.
