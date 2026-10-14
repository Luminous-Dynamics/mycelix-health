# garden-site-domain

A Rust-first domain contract for MED-BIO-002: privacy-first, offline garden
onboarding and region-bound, expert-reviewed non-clinical growing catalogues.

## Privacy by construction

`GardenSiteProfileV1` deliberately contains no field for a person's name,
exact street address, GPS coordinates, patient ID, medical history, or
traditional/sacred knowledge. It uses a region token selected from a reviewed catalogue manifest (not free
text) and supports local-only operation by default. Profile IDs must have an
opaque UUID shape; generate them locally and randomly rather than deriving them
from names or addresses. The crate structurally checks an opt-in reference and
timestamp but does not authenticate consent or authorize network access.
Withdrawal is represented separately. This state alone cannot delete any
already-synchronized copy, so a future sync adapter must authenticate consent,
re-check revocation, and verify the remote-copy deletion/export workflow.

The schema stores interface preferences rather than the reason someone needs
them. Do not add health or disability diagnoses to the profile.

## Fail-closed catalogue selection

An approved and unexpired catalogue must match the profile's jurisdiction and
region. Species-specific non-clinical cultivation guidance remains blocked when
site facts are missing, land-use permission is unresolved or denied, site hazard
review is incomplete, or the review only permits general planning.

Every visible entry needs separate evidence for:
- taxon identity review;
- conservation clearance for cultivation;
- authorization to disclose its public name/record;
- reviewed horticultural guidance.

Unknown, restricted, prohibited, stale, or unreviewed records are not shown.
A valid catalogue may contain **zero entries**. The crate never fills that gap
with model-generated species recommendations.

The gate checks the shape and scope of supplied evidence references only; it
does not authenticate documents, determine permit applicability, prove soil
safety, or replace botanists, conservation experts, landholders, community
governance, or qualified counsel. Passing this local checklist is not a
therapeutic recommendation or a statement that a plant is safe to consume.

## Data handling boundaries

- Start local-only; don't require registration for paper/on-device planning.
- Don't store exact household coordinates or addresses.
- Don't collect clinical or family health data for garden planning.
- Don't store traditional/sacred knowledge in catalogue entries. Only a public
  label with separate disclosure review can be surfaced.
- Sync is opt-in, and revocation must lead to a separately verified remote-copy
  deletion/export flow. This crate models the intent but doesn't perform I/O.
- Printed/exported profiles should use broad locality and a deliberate
  disclosure preview; exports are not automatically anonymous.

## Run in the Mycelix-Health workspace

```sh
cargo test -p garden-site-domain --target "$(rustc -vV | awk '/^host:/{print $2}')"
```

The fixture uses explicitly synthetic records only. It provides no real plant
catalogue and makes no medical, legal, agronomic or regulatory claims.
