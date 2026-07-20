#!/usr/bin/env python3
"""Normalize `nix path-info --recursive --json` into stable release evidence."""
from __future__ import annotations

import argparse
import json
import os
import pathlib
import tempfile
from typing import Any


class ClosureError(ValueError):
    pass


def normalize(raw: Any) -> dict[str, Any]:
    if isinstance(raw, dict):
        rows = []
        for path, metadata in raw.items():
            if not isinstance(metadata, dict):
                raise ClosureError("Nix closure metadata must be objects")
            row = dict(metadata)
            row.setdefault("path", path)
            rows.append(row)
    elif isinstance(raw, list):
        rows = raw
    else:
        raise ClosureError("Nix closure JSON must be an object or list")
    normalized = []
    for item in rows:
        if not isinstance(item, dict):
            raise ClosureError("Nix closure entries must be objects")
        path = item.get("path") or item.get("storePath")
        if not isinstance(path, str) or not path.startswith("/nix/store/"):
            raise ClosureError("Nix closure entry lacks a store path")
        references = item.get("references", [])
        if references is None:
            references = []
        if not isinstance(references, list) or not all(isinstance(v, str) for v in references):
            raise ClosureError(f"invalid references for {path}")
        nar_hash = item.get("narHash") or item.get("nar_hash")
        nar_size = item.get("narSize") if "narSize" in item else item.get("nar_size")
        if not isinstance(nar_hash, str) or not nar_hash:
            raise ClosureError(f"missing NAR hash for {path}")
        if not isinstance(nar_size, int) or nar_size < 0:
            raise ClosureError(f"missing NAR size for {path}")
        normalized.append({
            "path": path,
            "nar_hash": nar_hash,
            "nar_size": nar_size,
            "references": sorted(references),
        })
    normalized.sort(key=lambda item: item["path"])
    if len({item["path"] for item in normalized}) != len(normalized):
        raise ClosureError("duplicate Nix store path")
    return {
        "schema_version": 1,
        "component_count": len(normalized),
        "components": normalized,
    }


def write_create_only(path: pathlib.Path, value: dict[str, Any]) -> None:
    if path.exists():
        raise ClosureError(f"refusing to overwrite {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, sort_keys=True)
        handle.write("\n")
    os.chmod(path, 0o600)


def self_test() -> None:
    raw = {
        "/nix/store/b-example": {
            "narHash": "sha256-b",
            "narSize": 2,
            "references": ["/nix/store/a-example"],
            "registrationTime": 123,
        },
        "/nix/store/a-example": {
            "narHash": "sha256-a",
            "narSize": 1,
            "references": [],
        },
    }
    value = normalize(raw)
    assert [item["path"] for item in value["components"]] == [
        "/nix/store/a-example", "/nix/store/b-example"
    ]
    assert "registrationTime" not in json.dumps(value)
    with tempfile.TemporaryDirectory() as tmp:
        path = pathlib.Path(tmp) / "closure.json"
        write_create_only(path, value)
        assert json.loads(path.read_text()) == value
    print("Nix closure normalization self-test: ok")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return 0
    if not args.input or not args.output:
        raise ClosureError("--input and --output are required")
    write_create_only(args.output.resolve(), normalize(json.loads(args.input.read_text())))
    print(args.output.resolve())
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ClosureError, OSError, ValueError, json.JSONDecodeError) as error:
        print(f"Nix closure normalization error: {error}")
        raise SystemExit(1)
