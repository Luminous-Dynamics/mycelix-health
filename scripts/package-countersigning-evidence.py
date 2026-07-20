#!/usr/bin/env python3
"""Create a deterministic checksum manifest for verified live evidence."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

REQUIRED = (
    "LIVE_COUNTERSIGNING_VERIFIED.json",
    "evidence.json",
    "verified-summary.json",
    "execution-audits.json",
)


def fail(message: str) -> None:
    raise SystemExit(f"countersigning evidence packaging failed: {message}")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: package-countersigning-evidence.py EVIDENCE_DIR")
    root = Path(sys.argv[1]).resolve()
    if not root.is_dir():
        fail("evidence directory does not exist")
    missing = [name for name in REQUIRED if not (root / name).is_file()]
    if missing:
        fail(f"verified evidence is incomplete: {', '.join(missing)}")
    marker = json.loads((root / REQUIRED[0]).read_text())
    if marker.get("schema_version") != 1 or marker.get("status") != "verified":
        fail("success marker is not a verified schema-v1 result")
    if not marker.get("scenario_id") or not marker.get("participant_action_hashes"):
        fail("success marker omits its scenario or participant action hashes")
    files = {
        name: {"sha256": sha256(root / name), "size_bytes": (root / name).stat().st_size}
        for name in sorted(REQUIRED)
    }
    manifest = {
        "schema_version": 1,
        "scenario_id": marker["scenario_id"],
        "status": "verified",
        "files": files,
    }
    target = root / "MANIFEST.json"
    if target.exists():
        fail("MANIFEST.json already exists; refusing to overwrite evidence")
    target.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(f"packaged verified countersigning evidence: {target}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, TypeError, KeyError) as error:
        fail(str(error))
