#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

output_root="${1:-target/reproducible-release}"
dry_run="${HEALTH_REPRODUCIBLE_BUILD_DRY_RUN:-0}"

if [[ -e "$output_root" ]]; then
  echo "reproducible build output already exists: $output_root" >&2
  exit 2
fi
if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
  echo "reproducible release builds require a clean tracked worktree" >&2
  exit 2
fi

revision="$(git rev-parse HEAD)"
source_tree="$(git rev-parse HEAD^{tree})"
source_date_epoch="$(git show -s --format=%ct "$revision")"
flake_lock_sha256="$(sha256sum flake.lock | awk '{print $1}')"
mkdir -p "$output_root"
chmod 700 "$output_root"

stage_build() {
  local label="$1"
  local build_root="$output_root/$label"
  local source_root="$build_root/source"
  local staged_root="$build_root/staged"
  local profile="$build_root/nix-profile"
  mkdir -p "$source_root" "$staged_root/wasm"
  git archive "$revision" | tar -x -C "$source_root"
  python3 - "$build_root/build-context.json" "$revision" "$source_tree" "$source_date_epoch" "$flake_lock_sha256" <<'PY'
import json, os, pathlib, sys
path = pathlib.Path(sys.argv[1])
value = {
    "schema_version": 1,
    "source_revision": sys.argv[2],
    "source_tree": sys.argv[3],
    "source_date_epoch": int(sys.argv[4]),
    "flake_lock_sha256": sys.argv[5],
    "locale": "C",
    "timezone": "UTC",
    "build_command": "nix develop --profile <profile> --command scripts/build-release-zomes.sh && hc dna pack && hc app pack",
}
with path.open("x", encoding="utf-8") as handle:
    json.dump(value, handle, indent=2, sort_keys=True)
    handle.write("\n")
os.chmod(path, 0o600)
PY
  if [[ "$dry_run" == "1" ]]; then
    echo "DRY-RUN [$label]: build clean source export at $source_root"
    return 0
  fi
  (
    cd "$source_root"
    export SOURCE_DATE_EPOCH="$source_date_epoch" TZ=UTC LC_ALL=C LANG=C
    nix develop --profile "$profile" --command bash -euo pipefail -c '
      scripts/build-release-zomes.sh
      hc dna pack dna/ -o dna/health.dna
      hc app pack . -o mycelix-health.happ
    '
  )
  nix path-info --recursive --json "$(readlink -f "$profile")" > "$build_root/nix-closure.raw.json"
  python3 scripts/normalize-nix-closure.py \
    --input "$build_root/nix-closure.raw.json" \
    --output "$build_root/nix-closure.json"
  rm "$build_root/nix-closure.raw.json"
  mapfile -t packages < <(python3 "$source_root/scripts/check-release-manifest.py" --print-packages)
  for package in "${packages[@]}"; do
    cp "$source_root/target/wasm32-unknown-unknown/release/${package}.wasm" "$staged_root/wasm/${package}.wasm"
  done
  cp "$source_root/dna/health.dna" "$staged_root/health.dna"
  cp "$source_root/mycelix-health.happ" "$staged_root/mycelix-health.happ"
  find "$staged_root" -type f -exec chmod 600 {} +
}

stage_build build-a
stage_build build-b

if [[ "$dry_run" == "1" ]]; then
  echo "DRY-RUN: compare build-a and build-b artifacts plus normalized Nix closures"
  exit 0
fi

python3 scripts/compare-release-builds.py \
  --first "$output_root/build-a/staged" \
  --second "$output_root/build-b/staged" \
  --first-context "$output_root/build-a" \
  --second-context "$output_root/build-b" \
  --output "$output_root/reproducibility-report.json"

python3 - "$output_root" "$revision" "$source_tree" <<'PY'
import hashlib
import json
import os
import pathlib
import sys

root = pathlib.Path(sys.argv[1]).resolve()
revision = sys.argv[2]
source_tree = sys.argv[3]
report = root / "reproducibility-report.json"
closure = root / "build-a/nix-closure.json"
context = root / "build-a/build-context.json"
value = {
    "schema_version": 2,
    "status": "verified",
    "source_revision": revision,
    "source_tree": source_tree,
    "reproducibility_report_sha256": hashlib.sha256(report.read_bytes()).hexdigest(),
    "nix_closure_sha256": hashlib.sha256(closure.read_bytes()).hexdigest(),
    "build_context_sha256": hashlib.sha256(context.read_bytes()).hexdigest(),
    "build_roots": ["build-a", "build-b"],
    "source_export": "git archive of the exact source revision",
}
target = root / "reproducible-build-provenance.json"
with target.open("x", encoding="utf-8") as handle:
    json.dump(value, handle, indent=2, sort_keys=True)
    handle.write("\n")
os.chmod(target, 0o600)
PY

echo "$output_root/reproducibility-report.json"
