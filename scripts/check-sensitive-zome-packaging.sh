#!/usr/bin/env bash
set -euo pipefail

# P0 containment gate for #149.
#
# The mental_health zomes are workspace members, but their current sensitive
# app-entry storage model is not yet privacy-qualified for packaging. HDK 0.6.x
# app entries default to public unless explicitly configured private.
#
# Until a replacement storage/sharing design is qualified, no production DNA
# manifest may contain a live mental_health token. This intentionally does NOT
# block workspace compilation, unit tests, or isolated qualification work.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

manifest_roots=()
[[ -d dna ]] && manifest_roots+=(dna)
[[ -d dnas ]] && manifest_roots+=(dnas)

if ((${#manifest_roots[@]} == 0)); then
  echo "privacy packaging gate: no DNA manifest roots found; refusing to claim a pass" >&2
  exit 1
fi

mapfile -t manifests < <(
  find "${manifest_roots[@]}" -type f \( -name '*.yaml' -o -name '*.yml' \) -print | sort
)

if ((${#manifests[@]} == 0)); then
  echo "privacy packaging gate: no DNA YAML manifests found; refusing to claim a pass" >&2
  exit 1
fi

failed=0
for manifest in "${manifests[@]}"; do
  # Remove full-line and trailing YAML comments before scanning. The containment
  # rule is intentionally broader than a zome-name regex: any live
  # `mental_health` token in a packaged DNA manifest requires review while #149
  # remains open. This prevents bypass by renaming the zome but leaving a
  # mental_health WASM path or dependency reference.
  live_yaml="$(
    sed -E 's/[[:space:]]+#.*$//' "$manifest" \
      | grep -vE '^[[:space:]]*#' \
      || true
  )"

  if printf '%s\n' "$live_yaml" | grep -En 'mental_health(_integrity)?'; then
    echo "ERROR: $manifest contains an unqualified live mental_health packaging reference." >&2
    failed=1
  fi
done

if ((failed)); then
  cat >&2 <<'EOF'

Mental-health packaging is blocked by P0 privacy issue #149.
The current mental_health entry model contains sensitive plaintext entry types
whose storage/sharing semantics are not yet qualified for deployment.

Do not bypass this gate by renaming the zome or artifact. Qualify the storage
design first, then replace this temporary containment gate in the same reviewed
change that introduces the exact privacy evidence.
EOF
  exit 1
fi

echo "privacy packaging gate: PASS — no live mental_health references in DNA YAML manifests"
