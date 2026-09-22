# Public-health surveillance fixtures

These fixtures are synthetic, non-clinical, and non-operational. They exist to freeze semantics and adversarial expectations for the defensive surveillance program before implementation or live-provider integration.

Current corpus:

- `multimodal-phase-a-v1.yaml` — synthetic source, measurement, coverage, lineage, event, correction, ethics and aggregate genomic-summary fixtures.
- `multimodal-phase-a-v1.expected.yaml` — expected typed classifications and forbidden claim-promotion targets.

The fixtures intentionally contain no patient records, exact household data, raw pathogen/genomic sequences, wet-lab procedures, credentials, diagnosis, treatment recommendations, outbreak declarations or action authority.

Canonical fixture hashes should be introduced only when the owning domain has a reviewed canonical encoding. Do not hash ad hoc YAML/JSON serialization and treat that as scientific identity.
