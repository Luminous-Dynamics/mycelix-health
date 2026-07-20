#!/usr/bin/env python3
"""Generate a deterministic CycloneDX 1.6 SBOM from committed lockfiles."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import re
import sys
import tomllib
import urllib.parse
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = ROOT / "release" / "supply-chain-policy.json"
DEFAULT_OUTPUT = ROOT / "release" / "health-v1.sbom.cdx.json"


class SbomError(ValueError):
    pass


def purl_name(name: str) -> str:
    return urllib.parse.quote(name, safe="@/")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def npm_name_from_path(path: str) -> str | None:
    if "node_modules/" not in path:
        return None
    tail = path.rsplit("node_modules/", 1)[1]
    if tail.startswith("@"):
        parts = tail.split("/")
        return "/".join(parts[:2]) if len(parts) >= 2 else None
    return tail.split("/", 1)[0]


def npm_parent_path(path: str) -> str:
    if "/node_modules/" in path:
        return path.rsplit("/node_modules/", 1)[0]
    return ""


def resolve_npm_dependency(packages: dict[str, Any], owner_path: str, dependency: str) -> str | None:
    cursor = owner_path
    while True:
        candidate = f"{cursor}/node_modules/{dependency}" if cursor else f"node_modules/{dependency}"
        if candidate in packages:
            return candidate
        if not cursor:
            return None
        cursor = npm_parent_path(cursor)


def npm_components(project_name: str, directory: str, lockfile: Path) -> tuple[list[dict], list[dict]]:
    lock = json.loads(lockfile.read_text())
    packages = lock.get("packages")
    if not isinstance(packages, dict):
        raise SbomError(f"{lockfile.relative_to(ROOT)} has no packages map")
    components: list[dict] = []
    dependencies: list[dict] = []
    refs: dict[str, str] = {}

    root = packages.get("")
    if not isinstance(root, dict):
        raise SbomError(f"{lockfile.relative_to(ROOT)} has no root package")
    root_ref = f"npm:{project_name}:root"
    refs[""] = root_ref
    components.append(
        {
            "type": "application",
            "bom-ref": root_ref,
            "name": root.get("name", project_name),
            "version": root.get("version", "0.0.0"),
            "purl": f"pkg:npm/{purl_name(root.get('name', project_name))}@{root.get('version', '0.0.0')}",
            "properties": [
                {"name": "mycelix:ecosystem", "value": "npm"},
                {"name": "mycelix:project_directory", "value": directory},
                {"name": "mycelix:lockfile", "value": str(lockfile.relative_to(ROOT))},
            ],
        }
    )

    for path, item in sorted(packages.items()):
        if not path or not isinstance(item, dict):
            continue
        name = npm_name_from_path(path)
        version = item.get("version")
        if not name or not isinstance(version, str):
            raise SbomError(f"cannot identify npm component at {path}")
        ref = f"npm:{project_name}:{path}"
        refs[path] = ref
        hashes = []
        integrity = item.get("integrity")
        if isinstance(integrity, str) and integrity.startswith("sha512-"):
            try:
                raw = base64.b64decode(integrity[7:], validate=True)
            except ValueError as error:
                raise SbomError(f"invalid npm integrity for {project_name}:{path}") from error
            hashes.append({"alg": "SHA-512", "content": raw.hex()})
        properties = [
            {"name": "mycelix:ecosystem", "value": "npm"},
            {"name": "mycelix:project", "value": project_name},
            {"name": "mycelix:lock_path", "value": path},
            {"name": "mycelix:dependency_scope", "value": "development" if item.get("dev") else "production"},
        ]
        resolved = item.get("resolved")
        if isinstance(resolved, str):
            properties.append({"name": "mycelix:resolved", "value": resolved})
        component = {
            "type": "library",
            "bom-ref": ref,
            "name": name,
            "version": version,
            "purl": f"pkg:npm/{purl_name(name)}@{version}",
            "scope": "excluded" if item.get("dev") else "required",
            "properties": properties,
        }
        if hashes:
            component["hashes"] = hashes
        if isinstance(item.get("license"), str):
            component["licenses"] = [{"license": {"id": item["license"]}}]
        components.append(component)

    for path, item in sorted(packages.items()):
        if not isinstance(item, dict):
            continue
        ref = refs.get(path)
        if not ref:
            continue
        names: set[str] = set()
        for field in ("dependencies", "optionalDependencies"):
            value = item.get(field)
            if isinstance(value, dict):
                names.update(str(name) for name in value)
        children = []
        for name in sorted(names):
            target = resolve_npm_dependency(packages, path, name)
            if target is not None and target in refs:
                children.append(refs[target])
        dependencies.append({"ref": ref, "dependsOn": sorted(set(children))})

    return components, dependencies


CARGO_DEP_RE = re.compile(r"^(?P<name>[^ ]+)(?: (?P<version>\d+\.\d+\.\d+(?:[-+][^ ]+)?))?(?: \((?P<source>.+)\))?$")


def cargo_components(lockfile: Path) -> tuple[list[dict], list[dict]]:
    lock = tomllib.loads(lockfile.read_text())
    packages = lock.get("package")
    if not isinstance(packages, list):
        raise SbomError("Cargo.lock has no package list")
    components: list[dict] = []
    dependencies: list[dict] = []
    refs_by_key: dict[tuple[str, str, str], str] = {}
    refs_by_name: dict[str, list[tuple[str, str, str]]] = {}

    for item in packages:
        name = item["name"]
        version = item["version"]
        source = item.get("source", "workspace")
        ref = f"cargo:{name}:{version}:{hashlib.sha256(source.encode()).hexdigest()[:12]}"
        key = (name, version, source)
        refs_by_key[key] = ref
        refs_by_name.setdefault(name, []).append(key)
        properties = [
            {"name": "mycelix:ecosystem", "value": "cargo"},
            {"name": "mycelix:source", "value": source},
        ]
        component: dict[str, Any] = {
            "type": "library",
            "bom-ref": ref,
            "name": name,
            "version": version,
            "purl": f"pkg:cargo/{purl_name(name)}@{version}",
            "scope": "required",
            "properties": properties,
        }
        checksum = item.get("checksum")
        if isinstance(checksum, str):
            component["hashes"] = [{"alg": "SHA-256", "content": checksum}]
        components.append(component)

    for item in packages:
        key = (item["name"], item["version"], item.get("source", "workspace"))
        ref = refs_by_key[key]
        children: list[str] = []
        for raw in item.get("dependencies", []):
            match = CARGO_DEP_RE.match(raw)
            if not match:
                raise SbomError(f"cannot parse Cargo dependency: {raw}")
            name = match.group("name")
            version = match.group("version")
            source = match.group("source")
            candidates = refs_by_name.get(name, [])
            if version:
                candidates = [candidate for candidate in candidates if candidate[1] == version]
            if source:
                candidates = [candidate for candidate in candidates if candidate[2] == source]
            if len(candidates) == 1:
                children.append(refs_by_key[candidates[0]])
            elif not candidates:
                raise SbomError(f"Cargo dependency not present in lockfile: {raw}")
            else:
                # Cargo omits a version only when the name resolves unambiguously in context;
                # keep all same-name locked candidates rather than inventing one.
                children.extend(refs_by_key[candidate] for candidate in candidates)
        dependencies.append({"ref": ref, "dependsOn": sorted(set(children))})

    return components, dependencies


def generate() -> dict:
    policy = json.loads(POLICY_PATH.read_text())
    npm_projects = policy["npm"]["projects"]
    components: list[dict] = []
    dependencies: list[dict] = []

    cargo_lock = ROOT / policy["cargo"]["lockfile"]
    cargo_components_list, cargo_dependencies = cargo_components(cargo_lock)
    components.extend(cargo_components_list)
    dependencies.extend(cargo_dependencies)

    for project in npm_projects:
        npm_component_list, npm_dependencies = npm_components(
            project["name"], project["directory"], ROOT / project["lockfile"]
        )
        components.extend(npm_component_list)
        dependencies.extend(npm_dependencies)

    components.sort(key=lambda component: component["bom-ref"])
    dependencies.sort(key=lambda dependency: dependency["ref"])
    metadata_component = {
        "type": "application",
        "bom-ref": "mycelix-health:health-v1",
        "name": "mycelix-health",
        "version": "health-v1",
        "properties": [
            {"name": "mycelix:release_manifest", "value": "release/health-v1.json"},
            {"name": "mycelix:supply_chain_policy", "value": "release/supply-chain-policy.json"},
            {"name": "mycelix:cargo_lock_sha256", "value": sha256_file(cargo_lock)},
            *[
                {
                    "name": f"mycelix:npm_lock_sha256:{project['name']}",
                    "value": sha256_file(ROOT / project["lockfile"]),
                }
                for project in npm_projects
            ],
        ],
    }
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "version": 1,
        "metadata": {"component": metadata_component},
        "components": components,
        "dependencies": [
            {
                "ref": "mycelix-health:health-v1",
                "dependsOn": sorted(
                    component["bom-ref"]
                    for component in components
                    if component.get("type") == "application" or component["bom-ref"].startswith("cargo:")
                ),
            },
            *dependencies,
        ],
    }


def canonical_bytes(document: dict) -> bytes:
    return (json.dumps(document, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true", help="fail when output differs from generated content")
    parser.add_argument("--print-digest", action="store_true")
    args = parser.parse_args()
    document = generate()
    payload = canonical_bytes(document)
    output = args.output if args.output.is_absolute() else ROOT / args.output
    if args.check:
        if not output.is_file() or output.read_bytes() != payload:
            raise SbomError(f"SBOM is stale: {output.relative_to(ROOT)}")
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(payload)
    if args.print_digest:
        print(hashlib.sha256(payload).hexdigest())
    else:
        print(f"generated {len(document['components'])} components: {output.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (SbomError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
        print(f"SBOM generation error: {error}", file=sys.stderr)
        raise SystemExit(1)
