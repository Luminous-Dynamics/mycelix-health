#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 MANIFEST.json OUTPUT_CANDIDATE.json" >&2
  exit 64
fi
MANIFEST="$1"
OUTPUT="$2"
ROOT="$(git rev-parse --show-toplevel)"
FILTER="$ROOT/scripts/reference-clinical-authority-candidate.jq"
[[ -s "$MANIFEST" ]] || { echo "Missing manifest: $MANIFEST" >&2; exit 1; }
[[ -s "$FILTER" ]] || { echo "Missing reference policy filter: $FILTER" >&2; exit 1; }
jq -f "$FILTER" "$MANIFEST" > "$OUTPUT"
