#!/usr/bin/env bash
set -euo pipefail

# P0 containment gate for #149.
#
# The mental_health zomes are compiled as workspace members but their current
# app-entry storage model is not yet privacy-qualified for packaging. HDK 0.6.x
# app entries default to public unless explicitly configured private, and the
# current mental_health EntryTypes include sensitive plaintext payloads.
#
# Until an exact encrypted/private sharing design is qualified, no production
# DNA manifest may package either mental_health_integrity or its coordinator.
# This gate intentionally does NOT block workspace compilation or isolated tests.

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
  # Match either the canonical zome name or a manifest path that would package
  # one of the mental-health WASM artifacts. Comments containing the words alone
  # do not trip the gate unless they look like an actual name/path declaration.
  if grep -En \
    '(^|[[:space:]-])name:[[:space:]]*mental_health(_integrity)?([[:space:]]|$)|(^|[[:space:]])(path|bundled):[^#\n]*mental_health(_integrity)?\.wasm([[:space:]]|$)' \
    "$manifest"; then
    echo "ERROR: $manifest packages an unqualified mental_health zome." >&2
    failed=1
  fi
done

if ((failed)); then
  cat >&2 <<'EOF'

Mental-health packaging is blocked by P0 privacy issue #149.
The current mental_health entry model contains sensitive plaintext entry types
whose storage/sharing semantics are not yet qualified for deployment.

Do not bypass this gate by renaming the zome. Qualify the storage design first,
then replace this temporary containment gate in the same reviewed/qualified
change that introduces the exact privacy evidence.
EOF
  exit 1
fi

echo "privacy packaging gate: PASS — mental_health is not present in packaged DNA manifests"
