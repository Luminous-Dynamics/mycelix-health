#!/usr/bin/env python3
"""Fail when packaged Tier-1 Holochain link types drift from the privacy inventory.

This is a classification/completeness gate. It does not prove that current DHT
link topology is confidential or that every authority stores every link op.
No third-party Python packages are required.
"""

from __future__ import annotations

import json
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "dna" / "dna.yaml"
INVENTORY = ROOT / "docs" / "security" / "tier1_link_topology_inventory.json"

LINK_RE = re.compile(
    r"pub\s+enum\s+LinkTypes\s*\{(?P<body>.*?)\n\}",
    re.DOTALL,
)
VARIANT_RE = re.compile(
    r"^\s*([A-Za-z][A-Za-z0-9_]*)\s*,\s*(?://.*)?$",
    re.MULTILINE,
)
INTEGRITY_NAME_RE = re.compile(
    r"^\s*-\s+name:\s+([A-Za-z0-9_]+_integrity)\s*$",
    re.MULTILINE,
)

VALID_TOPOLOGY_CLASSES = {
    "intentionally_public",
    "opaque_scoped_index",
    "private_local_index",
    "remove_global_index",
    "split_required",
}
VALID_RISK = {"low", "moderate", "high", "critical"}


def fail(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def packaged_integrity_zomes() -> list[str]:
    text = MANIFEST.read_text(encoding="utf-8")
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


def link_variants(zome_name: str) -> list[str]:
    path = source_path(zome_name)
    if not path.is_file():
        fail(f"packaged integrity zome has no expected source file: {path.relative_to(ROOT)}")
    text = path.read_text(encoding="utf-8")
    match = LINK_RE.search(text)
    if not match:
        fail(f"could not locate LinkTypes enum in {path.relative_to(ROOT)}")
    variants = VARIANT_RE.findall(match.group("body"))
    if not variants:
        fail(f"LinkTypes enum has no unit variants in {path.relative_to(ROOT)}")
    if len(variants) != len(set(variants)):
        fail(f"duplicate LinkTypes variants in {path.relative_to(ROOT)}")
    return variants


def load_inventory() -> dict:
    try:
        return json.loads(INVENTORY.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot load inventory: {exc}")


def main() -> None:
    inventory = load_inventory()
    links = inventory.get("links")
    if not isinstance(links, list):
        fail("inventory.links must be an array")

    declared_classes = inventory.get("topology_classes")
    if set(declared_classes or []) != VALID_TOPOLOGY_CLASSES:
        fail(
            "inventory.topology_classes must exactly match the checker-supported classes: "
            + ", ".join(sorted(VALID_TOPOLOGY_CLASSES))
        )

    classified: dict[tuple[str, str], dict] = {}
    for index, item in enumerate(links):
        if not isinstance(item, dict):
            fail(f"inventory link {index} is not an object")
        zome = item.get("zome")
        variant = item.get("variant")
        if not isinstance(zome, str) or not isinstance(variant, str):
            fail(f"inventory link {index} requires string zome and variant")
        key = (zome, variant)
        if key in classified:
            fail(f"duplicate inventory classification: {zome}::{variant}")

        topology_class = item.get("desired_topology_class")
        if topology_class not in VALID_TOPOLOGY_CLASSES:
            fail(f"invalid desired_topology_class for {zome}::{variant}: {topology_class!r}")
        risk = item.get("risk")
        if risk not in VALID_RISK:
            fail(f"invalid risk label for {zome}::{variant}: {risk!r}")

        for required in (
            "current_visibility",
            "contains_patient_or_clinical_correlation",
            "reason",
            "replacement",
        ):
            if required not in item:
                fail(f"missing {required} for {zome}::{variant}")

        if item["current_visibility"] != "dht_link_operation":
            fail(f"unexpected current_visibility for {zome}::{variant}: {item['current_visibility']!r}")
        if not isinstance(item["contains_patient_or_clinical_correlation"], bool):
            fail(f"contains_patient_or_clinical_correlation must be boolean for {zome}::{variant}")
        if not isinstance(item["reason"], str) or not item["reason"].strip():
            fail(f"reason must be non-empty for {zome}::{variant}")
        if not isinstance(item["replacement"], str) or not item["replacement"].strip():
            fail(f"replacement must be non-empty for {zome}::{variant}")

        # Public discovery is acceptable only for rows explicitly classified as
        # non-patient/non-clinical correlation. A future exception must change
        # the model/checker deliberately rather than bypass this invariant.
        if topology_class == "intentionally_public" and item["contains_patient_or_clinical_correlation"]:
            fail(
                f"intentionally_public link is marked patient/clinical-correlating: {zome}::{variant}; "
                "use split_required or a protected topology class"
            )

        classified[key] = item

    packaged = packaged_integrity_zomes()
    actual: set[tuple[str, str]] = set()
    counts: dict[str, int] = {}
    for zome in packaged:
        variants = link_variants(zome)
        counts[zome] = len(variants)
        actual.update((zome, variant) for variant in variants)

    classified_keys = set(classified)
    missing = sorted(actual - classified_keys)
    stale = sorted(classified_keys - actual)

    if missing:
        print("Unclassified packaged LinkTypes variants:", file=sys.stderr)
        for zome, variant in missing:
            print(f"  - {zome}::{variant}", file=sys.stderr)
    if stale:
        print("Stale topology inventory entries not present in packaged LinkTypes:", file=sys.stderr)
        for zome, variant in stale:
            print(f"  - {zome}::{variant}", file=sys.stderr)
    if missing or stale:
        raise SystemExit(1)

    subject = inventory.get("subject", {})
    declared_count = subject.get("link_variant_count")
    if declared_count != len(actual):
        fail(f"subject.link_variant_count={declared_count!r}, actual packaged link count={len(actual)}")
    declared_zomes = subject.get("packaged_integrity_zome_count")
    if declared_zomes != len(packaged):
        fail(
            f"subject.packaged_integrity_zome_count={declared_zomes!r}, "
            f"actual packaged integrity zomes={len(packaged)}"
        )

    class_counts = Counter(item["desired_topology_class"] for item in classified.values())
    risk_counts = Counter(item["risk"] for item in classified.values())
    correlated = sum(bool(item["contains_patient_or_clinical_correlation"]) for item in classified.values())

    print("Tier-1 link topology inventory: CLASSIFICATION PASS")
    print(f"  manifest: {MANIFEST.relative_to(ROOT)}")
    print(f"  inventory: {INVENTORY.relative_to(ROOT)}")
    print(f"  packaged integrity zomes: {len(packaged)}")
    print(f"  classified link variants: {len(actual)}")
    print(f"  patient/clinical-correlating rows: {correlated}")
    for zome in packaged:
        print(f"    {zome}: {counts[zome]}")
    print("  desired topology classes:")
    for name in sorted(class_counts):
        print(f"    {name}: {class_counts[name]}")
    print("  recorded risk labels:")
    for name in sorted(risk_counts):
        print(f"    {name}: {risk_counts[name]}")
    print()
    print("NOTE: this PASS proves topology classification completeness only.")
    print("It does NOT prove DHT confidentiality, unlinkability, migration safety, or legal compliance.")


if __name__ == "__main__":
    main()
