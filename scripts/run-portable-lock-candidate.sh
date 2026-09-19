#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/run-portable-lock-candidate.sh \
    --subject-checkout PATH \
    --parent-checkout PATH \
    --output-dir PATH \
    [--subject-sha 40_HEX_SHA] \
    [--require-package NAME]... \
    [--skip-self-test]

Generates and independently verifies a review-only Cargo.lock repair candidate.

Safety properties:
  * never reconciles Cargo.lock in the caller's product checkout;
  * runs reconciliation only in a disposable detached worktree;
  * requires pinned parent-source verification;
  * runs portable adversarial harnesses first unless --skip-self-test is given;
  * emits an editable movement-review template whose items all begin unreviewed;
  * requires a separate sealing step before review statements become canonical evidence;
  * never copies Cargo.lock.candidate into the product checkout;
  * never commits, pushes, merges, or grants qualification authority.

The default required package is:
  mycelix-symthaea-clinical-distribution-trust-v3
EOF
}

HARNESS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SUBJECT_CHECKOUT=""
PARENT_CHECKOUT=""
OUTPUT_DIR=""
SUBJECT_SHA=""
RUN_SELF_TEST=1
REQUIRED_PACKAGES=("mycelix-symthaea-clinical-distribution-trust-v3")

while (($#)); do
  case "$1" in
    --subject-checkout)
      [[ $# -ge 2 ]] || { echo "missing value for --subject-checkout" >&2; exit 2; }
      SUBJECT_CHECKOUT="$2"
      shift 2
      ;;
    --parent-checkout)
      [[ $# -ge 2 ]] || { echo "missing value for --parent-checkout" >&2; exit 2; }
      PARENT_CHECKOUT="$2"
      shift 2
      ;;
    --output-dir)
      [[ $# -ge 2 ]] || { echo "missing value for --output-dir" >&2; exit 2; }
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --subject-sha)
      [[ $# -ge 2 ]] || { echo "missing value for --subject-sha" >&2; exit 2; }
      SUBJECT_SHA="$2"
      shift 2
      ;;
    --require-package)
      [[ $# -ge 2 ]] || { echo "missing value for --require-package" >&2; exit 2; }
      REQUIRED_PACKAGES+=("$2")
      shift 2
      ;;
    --skip-self-test)
      RUN_SELF_TEST=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ -n "$SUBJECT_CHECKOUT" ]] || { echo "--subject-checkout is required" >&2; exit 2; }
[[ -n "$PARENT_CHECKOUT" ]] || { echo "--parent-checkout is required" >&2; exit 2; }
[[ -n "$OUTPUT_DIR" ]] || { echo "--output-dir is required" >&2; exit 2; }

for command in git python3 nix; do
  command -v "$command" >/dev/null 2>&1 || {
    echo "required command is unavailable: $command" >&2
    exit 2
  }
done

canonical_path() {
  python3 - "$1" <<'PY'
import pathlib
import sys
print(pathlib.Path(sys.argv[1]).expanduser().resolve())
PY
}

SUBJECT_CHECKOUT="$(canonical_path "$SUBJECT_CHECKOUT")"
PARENT_CHECKOUT="$(canonical_path "$PARENT_CHECKOUT")"
OUTPUT_DIR="$(canonical_path "$OUTPUT_DIR")"

[[ -d "$SUBJECT_CHECKOUT/.git" || -f "$SUBJECT_CHECKOUT/.git" ]] || {
  echo "subject checkout is not a Git checkout: $SUBJECT_CHECKOUT" >&2
  exit 2
}
[[ -d "$PARENT_CHECKOUT/.git" || -f "$PARENT_CHECKOUT/.git" ]] || {
  echo "parent checkout is not a Git checkout: $PARENT_CHECKOUT" >&2
  exit 2
}
[[ ! -e "$OUTPUT_DIR" ]] || {
  echo "output directory already exists; refusing overwrite: $OUTPUT_DIR" >&2
  exit 2
}

if [[ -z "$SUBJECT_SHA" ]]; then
  SUBJECT_SHA="$(git -C "$SUBJECT_CHECKOUT" rev-parse HEAD)"
fi
[[ "$SUBJECT_SHA" =~ ^[0-9a-fA-F]{40}$ ]] || {
  echo "subject SHA must be a full 40-character hexadecimal Git object ID" >&2
  exit 2
}
SUBJECT_SHA="${SUBJECT_SHA,,}"
git -C "$SUBJECT_CHECKOUT" cat-file -e "${SUBJECT_SHA}^{commit}"

HARNESS_SHA="$(git -C "$HARNESS_ROOT" rev-parse HEAD)"
HARNESS_TREE="$(git -C "$HARNESS_ROOT" rev-parse 'HEAD^{tree}')"
if [[ -n "$(git -C "$HARNESS_ROOT" status --porcelain=v1 --untracked-files=all)" ]]; then
  echo "portable repair harness checkout must be fully clean" >&2
  exit 2
fi

mapfile -t REQUIRED_PACKAGES < <(
  printf '%s\n' "${REQUIRED_PACKAGES[@]}" | sed '/^$/d' | LC_ALL=C sort -u
)

if (( RUN_SELF_TEST )); then
  echo "==> Running portable repair adversarial harness"
  python3 "$HARNESS_ROOT/scripts/check-portable-lock-candidate.py"
  echo "==> Running dependency movement review adversarial harness"
  python3 "$HARNESS_ROOT/scripts/check-lock-movement-review.py"
fi

require_args=()
for package in "${REQUIRED_PACKAGES[@]}"; do
  require_args+=(--require-package "$package")
done

echo "==> Generating RepairCandidateOnly"
python3 "$HARNESS_ROOT/scripts/prepare-cargo-lock-candidate.py" \
  --subject-checkout "$SUBJECT_CHECKOUT" \
  --subject-sha "$SUBJECT_SHA" \
  --parent-checkout "$PARENT_CHECKOUT" \
  --output-dir "$OUTPUT_DIR" \
  "${require_args[@]}"

echo "==> Independently verifying candidate"
python3 "$HARNESS_ROOT/scripts/verify-cargo-lock-candidate.py" \
  --subject-checkout "$SUBJECT_CHECKOUT" \
  --parent-checkout "$PARENT_CHECKOUT" \
  --candidate-dir "$OUTPUT_DIR" \
  --verification "$OUTPUT_DIR/lock-candidate-verification.json" \
  "${require_args[@]}"

echo "==> Creating editable unreviewed dependency-movement template"
python3 "$HARNESS_ROOT/scripts/prepare-lock-movement-review.py" \
  --candidate-dir "$OUTPUT_DIR" \
  --output "$OUTPUT_DIR/lock-movement-review.template.json"

python3 - "$OUTPUT_DIR" "$SUBJECT_SHA" "$HARNESS_SHA" "$HARNESS_TREE" <<'PY'
import json
import pathlib
import sys

out = pathlib.Path(sys.argv[1])
subject_sha = sys.argv[2]
harness_sha = sys.argv[3]
harness_tree = sys.argv[4]

candidate = json.loads((out / "lock-candidate.json").read_text(encoding="utf-8"))
verification = json.loads((out / "lock-candidate-verification.json").read_text(encoding="utf-8"))
audit = json.loads((out / "lock-audit.json").read_text(encoding="utf-8"))
review = json.loads((out / "lock-movement-review.template.json").read_text(encoding="utf-8"))

assert candidate["authority"] == "RepairCandidateOnly"
assert verification["authority"] == "VerifiedRepairCandidateOnly"
assert review["authority"] == "ReviewTemplateOnly"
assert all(item["disposition"] == "unreviewed" for item in review["items"])
assert candidate["subject"]["sha"] == subject_sha
assert candidate["harness"]["sha"] == harness_sha
assert candidate["harness"]["tree_sha"] == harness_tree

summary = out / "REVIEW.md"
if summary.exists() or summary.is_symlink():
    raise SystemExit(f"refusing to overwrite review summary: {summary}")

def count(name):
    value = audit.get(name, [])
    return len(value) if isinstance(value, (list, dict)) else "?"

lines = [
    "# Portable Cargo.lock candidate review",
    "",
    f"- Subject: `{subject_sha}`",
    f"- Harness: `{harness_sha}`",
    f"- Harness tree: `{harness_tree}`",
    f"- Candidate authority: `{candidate['authority']}`",
    f"- Verification authority: `{verification['authority']}`",
    f"- Review-template authority: `{review['authority']}`",
    f"- Explicit movement items requiring review: {len(review['items'])}",
    f"- Lock changed: `{candidate['locks']['changed']}`",
    f"- Before lock SHA-256: `{candidate['locks']['before_sha256']}`",
    f"- Candidate lock SHA-256: `{candidate['locks']['candidate_sha256']}`",
    "",
    "## Movement counts",
    "",
    f"- Added local packages: {count('added_local_packages')}",
    f"- Removed local packages: {count('removed_local_packages')}",
    f"- Added registry tuples: {count('added_registry_packages')}",
    f"- Removed registry tuples: {count('removed_registry_packages')}",
    f"- Registry names with version-set changes: {count('changed_registry_versions')}",
    f"- Added git-source tuples: {count('added_git_packages')}",
    f"- Removed git-source tuples: {count('removed_git_packages')}",
    "",
    "## Mandatory boundary",
    "",
    "`VerifiedRepairCandidateOnly` is not approval to copy, commit, merge, or qualify this lockfile.",
    "Every item in the review template starts as `unreviewed` and requires an explicit disposition/rationale.",
    "The editable template is not canonical review evidence; seal it after review.",
    "Even `MechanicalReviewCoverageComplete` does not authenticate a human reviewer or grant commit authority.",
    "",
    "If the movement review is accepted separately, copy the exact candidate bytes into a new branch/commit and run fresh exact-head `--locked` qualification.",
]
summary.write_text("\n".join(lines) + "\n", encoding="utf-8")
print("\n".join(lines))
PY

cat <<EOF

Portable repair evidence is ready for explicit review.

Candidate lock:
  $OUTPUT_DIR/Cargo.lock.candidate

Dependency audit:
  $OUTPUT_DIR/lock-audit.json

Independent candidate verification:
  $OUTPUT_DIR/lock-candidate-verification.json

Editable movement review template (all items initially unreviewed):
  $OUTPUT_DIR/lock-movement-review.template.json

Human-readable summary:
  $OUTPUT_DIR/REVIEW.md

Review each template item by replacing `unreviewed` with one of:
  expected | necessary | accepted | rejected
and add a non-empty rationale. `review_reference` is optional context and is not
authenticated by this v1 mechanism.

After editing, seal the review into canonical create-only evidence:

  python3 "$HARNESS_ROOT/scripts/seal-lock-movement-review.py" \\
    --candidate-dir "$OUTPUT_DIR" \\
    --draft "$OUTPUT_DIR/lock-movement-review.template.json" \\
    --output "$OUTPUT_DIR/lock-movement-review.statement.json"

Then independently verify exact mechanical coverage:

  python3 "$HARNESS_ROOT/scripts/verify-lock-movement-review.py" \\
    --candidate-dir "$OUTPUT_DIR" \\
    --review "$OUTPUT_DIR/lock-movement-review.statement.json" \\
    --verification "$OUTPUT_DIR/lock-movement-review-verification.json"

Add --require-no-rejections only when the review is intended to demonstrate that
no movement has been explicitly rejected. `ReviewStatementOnly` and
`MechanicalReviewCoverageComplete` are still not permission to copy, commit, push,
merge, or qualify Cargo.lock.candidate.

No product Cargo.lock was copied, committed, pushed, merged, or qualified by this command.
EOF
