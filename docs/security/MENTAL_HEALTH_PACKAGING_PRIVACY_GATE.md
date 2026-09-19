# Mental-Health Packaging Privacy Gate

Status: temporary P0 containment for issue #149.

## Why this exists

The `mental_health` integrity/coordinator crates are workspace members, but the current standard `health` DNA manifest does not package them.

That distinction is important. The source currently defines sensitive mental-health app entries without explicit private visibility, while HDK 0.6.x entry definitions default to public. If this zome were added to a DNA and used in its current form, sensitive plaintext app-entry content could be published to the DHT.

This gate preserves the safe current composition: compile and test the zome, but do not package it for deployment until a privacy-qualified storage/sharing design exists.

## What the gate proves

Only this narrow statement:

> The checked repository subject's DNA YAML manifests do not contain a live `mental_health` packaging reference.

It does **not** prove:

- that historical or custom DNA bundles never packaged these zomes;
- that the current mental-health source is safe to deploy;
- that protected data is encrypted;
- that legal/regulatory requirements are satisfied;
- that clinical workflows are valid or effective.

## Current source/package split

At the inspected baseline:

- `Cargo.toml` includes the mental-health integrity/coordinator crates as Tier-3 workspace members;
- `dna/dna.yaml` packages only the Tier-1 core zomes;
- `happ.yaml` points to the resulting `health.dna`.

The workspace may therefore compile a zome that the deployable DNA does not include. CI must preserve that distinction until privacy qualification is complete.

## Gate mechanics

`scripts/check-sensitive-zome-packaging.sh` scans DNA YAML manifests under `dna/` and `dnas/`.

It removes YAML comments and fails on any remaining `mental_health` token. The intentionally broad rule blocks not only the canonical zome name, but also attempts to hide the same unqualified component behind a renamed zome while retaining a mental-health artifact/dependency path.

The gate is independent of Rust compilation so a green workspace build cannot be mistaken for deployment/privacy qualification.

## Required replacement evidence

Do not delete/bypass this gate merely to enable the zome.

A replacement change should bind an exact source/WASM/DNA subject and demonstrate, at minimum:

1. storage classes are explicit: intentionally public metadata, private-local content, and encrypted shareable content;
2. protected plaintext is never published as a public DHT entry;
3. unauthorized peers cannot retrieve/decrypt protected content;
4. authorized sharing requires exact active consent/capability;
5. routing/sensitivity metadata cannot be modified to downgrade protection;
6. access/decryption events are auditable;
7. psychotherapy notes and SUD counseling notes can have independent consent semantics;
8. cross-agent sharing does not incorrectly rely on Holochain private entries, whose content is author-local;
9. conductor-level multi-agent tests exercise the actual packaged DNA, not just Rust helpers;
10. qualification records exact source, toolchain, WASM, and DNA identities.

The PR that first re-enables mental-health packaging should update or replace this gate in the same reviewable evidence lineage rather than relying on an out-of-band waiver.
