#!/usr/bin/env python3
"""Fail when packaged Tier-1 Holochain entry types drift from the privacy inventory.

This is deliberately a *classification* gate, not a claim that the current DNA is
private. Known P0 visibility defects remain represented in the inventory until
their storage models are migrated and separately qualified.

No third-party Python packages are required.
"""

from __future__ import annotations

import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "dna" / "dna.yaml"
INVENTORY = ROOT / "docs" / "security" / "tier1_phi_storage_inventory.json"

ENTRY_RE = re.compile(
    r"pub\s+enum\s+EntryTypes\s*\{(?P<body>.*?)\n\}",
    re.DOTALL,
)
VARIANT_RE = re.compile(r"^\s*([A-Za-z][A-Za-z0-9_]*)\s*\(", re.MULTILINE)
INTEGRITY_NAME_RE = re.compile(r"^\s*-\s+name:\s+([A-Za-z0-9_]+_integrity)\s*$", re.MULTILINE)

VALID_STORAGE_CLASSES = {
    "intentionally_public",
    "protected_shareable",
    "protected_shareable_v1_migrate_v2",
    "private_local",
    "opaque_public_commitment",
    "split_required",
}


def fail(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def packaged_integrity_zomes() -> list[str]:
    text = MANIFEST.read_text(encoding="utf-8")
    # Only inspect the integrity stanza. Coordinator dependencies repeat integrity
    # zome names and are not additional packaged entry-type authorities.
    integrity_text = text.split("\ncoordinator:", 1)[0]
    names = INTEGRITY_NAME_RE.findall(integrity_text)
    if not names:
        fail(f"no integrity zomes found in {MANIFEST.relative_to(ROOT)}")
    if len(names) != len(set(names)):
        fail("duplicate integrity zome names in DNA manifest")
    return names


def source_path(zome_name: str) -> Path:
    stem = zome_name.removesuffix("_integrity")
    return ROOT / "zomes" / stem / "integrity" / "src" / "lib.rs"


def entry_variants(zome_name: str) -> list[str]:
    path = source_path(zome_name)
    if not path.is_file():
        fail(f"packaged integrity zome has no expected source file: {path.relative_to(ROOT)}")
    text = path.read_text(encoding="utf-8")
    match = ENTRY_RE.search(text)
    if not match:
        fail(f"could not locate EntryTypes enum in {path.relative_to(ROOT)}")
    variants = VARIANT_RE.findall(match.group("body"))
    if not variants:
        fail(f"EntryTypes enum has no tuple variants in {path.relative_to(ROOT)}")
    if len(variants) != len(set(variants)):
        fail(f"duplicate EntryTypes variants in {path.relative_to(ROOT)}")
    return variants


def load_inventory() -> dict:
    try:
        return json.loads(INVENTORY.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot load inventory: {exc}")


def main() -> None:
    inventory = load_inventory()
    entries = inventory.get("entries")
    if not isinstance(entries, list):
        fail("inventory.entries must be an array")

    classified: dict[tuple[str, str], dict] = {}
    for index, item in enumerate(entries):
        if not isinstance(item, dict):
            fail(f"inventory entry {index} is not an object")
        zome = item.get("zome")
        variant = item.get("variant")
        key = (zome, variant)
        if not isinstance(zome, str) or not isinstance(variant, str):
            fail(f"inventory entry {index} requires string zome and variant")
        if key in classified:
            fail(f"duplicate inventory classification: {zome}::{variant}")
        storage_class = item.get("desired_storage_class")
        if storage_class not in VALID_STORAGE_CLASSES:
            fail(f"invalid desired_storage_class for {zome}::{variant}: {storage_class!r}")
        for required in (
            "current_visibility",
            "contains_patient_identifier",
            "contains_health_relationship_or_contact_detail",
            "current_write_state",
            "risk",
            "reason",
            "replacement",
        ):
            if required not in item:
                fail(f"missing {required} for {zome}::{variant}")
        if storage_class == "intentionally_public" and item["contains_patient_identifier"]:
            fail(
                f"intentionally_public entry is marked patient-identifying: {zome}::{variant}; "
                "split it or document a different reviewed representation"
            )
        classified[key] = item

    packaged = packaged_integrity_zomes()
    actual: set[tuple[str, str]] = set()
    counts: dict[str, int] = {}
    for zome in packaged:
        variants = entry_variants(zome)
        counts[zome] = len(variants)
        actual.update((zome, variant) for variant in variants)

    classified_keys = set(classified)
    missing = sorted(actual - classified_keys)
    stale = sorted(classified_keys - actual)

    if missing:
        print("Unclassified packaged EntryTypes variants:", file=sys.stderr)
        for zome, variant in missing:
            print(f"  - {zome}::{variant}", file=sys.stderr)
    if stale:
        print("Stale inventory entries not present in packaged EntryTypes:", file=sys.stderr)
        for zome, variant in stale:
            print(f"  - {zome}::{variant}", file=sys.stderr)
    if missing or stale:
        raise SystemExit(1)

    declared_count = inventory.get("subject", {}).get("entry_count")
    if declared_count != len(actual):
        fail(f"subject.entry_count={declared_count!r}, actual packaged entry count={len(actual)}")

    class_counts = Counter(item["desired_storage_class"] for item in classified.values())
    risk_counts = Counter(item["risk"] for item in classified.values())
    by_zome = defaultdict(list)
    for (zome, variant), item in classified.items():
        by_zome[zome].append((variant, item["desired_storage_class"], item["risk"]))

    print("Tier-1 PHI storage inventory: CLASSIFICATION PASS")
    print(f"  manifest: {MANIFEST.relative_to(ROOT)}")
    print(f"  inventory: {INVENTORY.relative_to(ROOT)}")
    print(f"  packaged integrity zomes: {len(packaged)}")
    print(f"  classified entry variants: {len(actual)}")
    for zome in packaged:
        print(f"    {zome}: {counts[zome]}")
    print("  desired storage classes:")
    for name in sorted(class_counts):
        print(f"    {name}: {class_counts[name]}")
    print("  recorded risk labels:")
    for name in sorted(risk_counts):
        print(f"    {name}: {risk_counts[name]}")
    print()
    print("NOTE: this PASS proves classification completeness only.")
    print("It does NOT prove PHI confidentiality, migration safety, deployment state, or legal compliance.")


if __name__ == "__main__":
    main()
