#!/usr/bin/env python3
"""Build a canonical final CI evidence receipt after all workflow gates pass.

The product subject and the CI harness are separate trust identities. This script
must execute from an independently checked-out harness tree; the subject under test
cannot define the verifier or final receipt-builder bytes that qualify itself.
Evidence storage must also remain outside both source checkouts.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import subprocess
from typing import Any, NoReturn

SCHEMA = "mycelix-health-ci-evidence/v1"
SOURCE_VERIFICATION_SCHEMA = "mycelix-health-parent-source-verification/v1"
REPOSITORY = "Luminous-Dynamics/mycelix-health"
HEX = frozenset("0123456789abcdefABCDEF")


def fail(message: str) -> NoReturn:
    raise SystemExit(f"CI evidence receipt failed: {message}")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    try:
        return sha256_bytes(path.read_bytes())
    except OSError as error:
        fail(f"cannot hash required file {path}: {error}")


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def full_sha(value: str, *, label: str) -> str:
    if len(value) != 40 or any(char not in HEX for char in value):
        fail(f"{label} must be a full 40-character hexadecimal Git SHA")
    return value.lower()


def file_digest_from_sidecar(path: pathlib.Path) -> str:
    sidecar = pathlib.Path(str(path) + ".sha256")
    try:
        expected = sidecar.read_text(encoding="utf-8").strip().split()[0]
    except (OSError, IndexError) as error:
        fail(f"cannot read digest sidecar for {path}: {error}")
    if len(expected) != 64 or any(char not in HEX for char in expected):
        fail(f"invalid SHA-256 sidecar for {path}")
    actual = sha256_file(path)
    if actual.lower() != expected.lower():
        fail(f"digest sidecar mismatch for {path}")
    return actual


def read_json(path: pathlib.Path, *, label: str) -> tuple[dict[str, Any], bytes]:
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {label} {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must contain a JSON object")
    return value, raw


def git(repo: pathlib.Path, *args: str, label: str) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), *args],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        fail(f"cannot read {label}: {error}")
    return result.stdout.strip()


def git_head(repo: pathlib.Path, *, label: str) -> str:
    return full_sha(git(repo, "rev-parse", "HEAD", label=label), label=label)


def git_tree(repo: pathlib.Path, *, label: str) -> str:
    return full_sha(git(repo, "rev-parse", "HEAD^{tree}", label=label), label=label)


def require_product_tracked_clean(repo: pathlib.Path) -> None:
    status = git(repo, "status", "--porcelain=v1", "--untracked-files=no", label="product tracked status")
    if status:
        fail("product checkout has tracked/index drift")


def require_harness_fully_clean(repo: pathlib.Path) -> None:
    status = git(repo, "status", "--porcelain=v1", "--untracked-files=all", label="harness status")
    if status:
        fail("qualification harness checkout is not fully clean")


def required_text(path: pathlib.Path, *, label: str) -> str:
    try:
        value = path.read_text(encoding="utf-8").strip()
    except OSError as error:
        fail(f"cannot read {label} {path}: {error}")
    if not value:
        fail(f"{label} is empty: {path}")
    return value


def required_env(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        fail(f"required GitHub Actions environment variable is missing: {name}")
    return value


def optional_env(name: str, default: str = "unknown") -> str:
    value = os.environ.get(name, default).strip()
    return value or default


def require_outside(root: pathlib.Path, candidate: pathlib.Path, *, label: str) -> None:
    try:
        candidate.relative_to(root)
    except ValueError:
        return
    fail(f"{label} must be outside source checkout {root}")


def build(args: argparse.Namespace) -> dict[str, Any]:
    if required_env("GITHUB_ACTIONS").lower() != "true":
        fail("authoritative CI evidence may only be emitted inside GitHub Actions")
    if required_env("GITHUB_REPOSITORY").lower() != REPOSITORY.lower():
        fail("authoritative CI evidence is bound to the mycelix-health repository")
    if required_env("GITHUB_EVENT_NAME") != "workflow_dispatch":
        fail("CI evidence v1 requires the reviewed workflow_dispatch admission path")

    workspace = args.workspace.resolve()
    harness = args.harness_root.resolve()
    evidence = args.evidence_dir.resolve()
    require_outside(workspace, evidence, label="evidence directory")
    require_outside(harness, evidence, label="evidence directory")
    if not evidence.is_dir():
        fail(f"evidence directory does not exist: {evidence}")

    subject_sha = full_sha(args.subject_sha, label="declared subject")
    harness_sha = full_sha(args.harness_sha, label="declared harness")
    observed_subject = git_head(workspace, label="observed subject")
    observed_harness = git_head(harness, label="observed harness")
    if observed_subject != subject_sha:
        fail("declared subject does not equal checked-out Health HEAD")
    if observed_harness != harness_sha:
        fail("declared harness does not equal checked-out harness HEAD")
    require_product_tracked_clean(workspace)
    require_harness_fully_clean(harness)
    subject_tree = git_tree(workspace, label="product tree SHA")
    harness_tree = git_tree(harness, label="harness tree SHA")

    workflow_sha = full_sha(required_env("GITHUB_WORKFLOW_SHA"), label="executing workflow SHA")
    if workflow_sha != harness_sha:
        fail("executing GitHub workflow SHA does not equal declared harness SHA")
    workflow_ref = required_env("GITHUB_WORKFLOW_REF")
    expected_workflow_marker = f"/{args.workflow}@"
    if not workflow_ref.startswith(f"{REPOSITORY}/") or expected_workflow_marker not in workflow_ref:
        fail("executing GitHub workflow ref does not match the declared harness workflow path")

    workflow = (harness / args.workflow).resolve()
    try:
        workflow.relative_to(harness)
    except ValueError:
        fail("workflow path escapes CI harness checkout")
    if not workflow.is_file():
        fail(f"workflow source does not exist in harness: {workflow}")

    action_pins = {
        "checkout": full_sha(args.checkout_action_sha, label="checkout action SHA"),
        "install_nix": full_sha(args.install_nix_action_sha, label="install-nix action SHA"),
        "upload_artifact": full_sha(args.upload_artifact_action_sha, label="upload-artifact action SHA"),
    }
    workflow_text = required_text(workflow, label="workflow source")
    required_action_refs = (
        f"actions/checkout@{action_pins['checkout']}",
        f"cachix/install-nix-action@{action_pins['install_nix']}",
        f"actions/upload-artifact@{action_pins['upload_artifact']}",
    )
    for action_ref in required_action_refs:
        if action_ref not in workflow_text:
            fail(f"recorded action pin is not present in exact workflow bytes: {action_ref}")

    cargo_lock = workspace / "Cargo.lock"
    flake_lock = workspace / "flake.lock"
    rust_toolchain = workspace / "rust-toolchain.toml"
    profile = workspace / "release/ci-parent-source-profile-v1.json"
    materializer = harness / "scripts/materialize-parent-mycelix.py"
    source_verifier = harness / "scripts/verify-parent-source-materialization.py"
    receipt_builder = harness / "scripts/build-ci-evidence-receipt.py"
    receipt_verifier = harness / "scripts/verify-ci-evidence-receipt.py"
    cargo_metadata = evidence / "cargo-metadata.json"
    flake_metadata = evidence / "flake-metadata.json"
    ci_drv = evidence / "ci-dev-shell.drv"

    for required in (
        cargo_lock,
        flake_lock,
        rust_toolchain,
        profile,
        materializer,
        source_verifier,
        receipt_builder,
        receipt_verifier,
        cargo_metadata,
        flake_metadata,
        ci_drv,
    ):
        if not required.is_file():
            fail(f"required CI evidence input is missing: {required}")

    source_verification_path = (evidence / args.source_verification).resolve()
    try:
        source_verification_path.relative_to(evidence)
    except ValueError:
        fail("source verification path escapes evidence directory")
    source_verification, source_raw = read_json(source_verification_path, label="source verification")
    if source_verification.get("schema") != SOURCE_VERIFICATION_SCHEMA:
        fail("unexpected source-verification schema")
    source_verification_sha = file_digest_from_sidecar(source_verification_path)
    if source_raw != canonical_bytes(source_verification):
        fail("source-verification report is not canonical JSON")

    if args.mode == "pinned":
        expected_source_authority = "VerifiedPinnedDependencySourceMaterialization"
        final_authority = "ReproducibleRepositoryCIEvidence"
    else:
        expected_source_authority = "VerifiedCompatibilityObservation"
        final_authority = "CompatibilityOnly"
    if source_verification.get("authority") != expected_source_authority:
        fail(
            f"source verification authority mismatch: expected {expected_source_authority}, "
            f"observed {source_verification.get('authority')!r}"
        )
    if source_verification.get("mode") != args.mode:
        fail("source verification mode does not match requested CI evidence mode")
    if source_verification.get("profile_sha256") != sha256_file(profile):
        fail("source verification profile digest does not match the product subject profile")
    if source_verification.get("materializer_sha256") != sha256_file(materializer):
        fail("source verification materializer digest does not match the exact harness")
    if source_verification.get("verifier_sha256") != sha256_file(source_verifier):
        fail("source verification verifier digest does not match the exact harness")

    for source, sidecar_name in (
        (cargo_lock, "Cargo.lock.sha256"),
        (flake_lock, "flake.lock.sha256"),
        (rust_toolchain, "rust-toolchain.toml.sha256"),
    ):
        sidecar = evidence / sidecar_name
        try:
            expected = sidecar.read_text(encoding="utf-8").strip().split()[0]
        except (OSError, IndexError) as error:
            fail(f"cannot read build-input digest {sidecar}: {error}")
        if sha256_file(source) != expected:
            fail(f"build-input digest drifted: {source.name}")

    run_id = required_env("GITHUB_RUN_ID")
    run_attempt = required_env("GITHUB_RUN_ATTEMPT")
    if not run_id.isdigit():
        fail("GITHUB_RUN_ID must be numeric")
    if not run_attempt.isdigit():
        fail("GITHUB_RUN_ATTEMPT must be numeric")

    report = {
        "schema": SCHEMA,
        "authority": final_authority,
        "result": "pass",
        "mode": args.mode,
        "repository": REPOSITORY,
        "subject": {
            "sha": subject_sha,
            "tree_sha": subject_tree,
        },
        "harness": {
            "sha": harness_sha,
            "tree_sha": harness_tree,
            "workflow_path": args.workflow,
            "workflow_ref": workflow_ref,
            "workflow_sha256": sha256_file(workflow),
            "materializer_sha256": sha256_file(materializer),
            "source_verifier_sha256": sha256_file(source_verifier),
            "receipt_builder_sha256": sha256_file(receipt_builder),
            "receipt_verifier_sha256": sha256_file(receipt_verifier),
            "action_pins": action_pins,
        },
        "source_verification": {
            "sha256": source_verification_sha,
            "authority": source_verification["authority"],
            "observed_source": source_verification.get("observed_source"),
            "exports": source_verification.get("exports"),
        },
        "build_inputs": {
            "cargo_lock_sha256": sha256_file(cargo_lock),
            "flake_lock_sha256": sha256_file(flake_lock),
            "rust_toolchain_sha256": sha256_file(rust_toolchain),
            "parent_profile_sha256": sha256_file(profile),
            "cargo_metadata_sha256": sha256_file(cargo_metadata),
            "flake_metadata_sha256": sha256_file(flake_metadata),
            "ci_dev_shell_drv": required_text(ci_drv, label="CI dev-shell derivation"),
        },
        "execution": {
            "event_name": required_env("GITHUB_EVENT_NAME"),
            "run_id": run_id,
            "run_attempt": run_attempt,
            "runner_os": required_env("RUNNER_OS"),
            "runner_arch": required_env("RUNNER_ARCH"),
            "runner_image_os": optional_env("ImageOS"),
            "runner_image_version": optional_env("ImageVersion"),
        },
        "claims": [
            "All preceding mandatory gates in the exact recorded harness completed successfully before this receipt was created.",
            "The product subject and qualification harness are separate exact Git identities.",
            "The executing GitHub workflow identity equals the reviewed harness identity recorded in this receipt.",
            "The exact product and harness tree identities were clean under their declared cleanliness policies when this receipt was created.",
            "The exact subject, exact harness/workflow bytes, independently verified dependency source, committed lock, and Nix environment inputs are bound into this receipt.",
        ],
        "non_claims": [
            "This receipt is not clinical validation, calibration, effectiveness, or safety evidence.",
            "This receipt grants no clinician/patient presentation, diagnosis, prescribing, dispensing, administration, or treatment authority.",
            "This receipt does not establish legal/regulatory compliance, clearance, or approval.",
            "Runner fingerprints do not establish bit-for-bit hermeticity across arbitrary kernels or CPUs.",
        ],
    }
    if args.mode == "compatibility":
        report["claims"].append(
            "The result applies only to the exact sampled parent source recorded by the independent source verifier."
        )
        report["non_claims"].append(
            "CompatibilityOnly cannot satisfy pinned exact-subject qualification or promotion prerequisites."
        )

    output = args.output.resolve()
    try:
        output.relative_to(evidence)
    except ValueError:
        fail("CI evidence output must remain inside the declared evidence directory")
    sidecar = pathlib.Path(str(output) + ".sha256")
    for path in (output, sidecar):
        if path.exists() or path.is_symlink():
            fail(f"refusing to overwrite CI evidence artifact: {path}")
    output.parent.mkdir(parents=True, exist_ok=True)
    report_bytes = canonical_bytes(report)
    with output.open("xb") as handle:
        handle.write(report_bytes)
    with sidecar.open("xb") as handle:
        handle.write((sha256_bytes(report_bytes) + "\n").encode())
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("pinned", "compatibility"), required=True)
    parser.add_argument("--subject-sha", required=True)
    parser.add_argument("--harness-sha", required=True)
    parser.add_argument("--workspace", type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument("--harness-root", type=pathlib.Path, required=True)
    parser.add_argument("--evidence-dir", type=pathlib.Path, required=True)
    parser.add_argument("--source-verification", default="parent-source-verification.json")
    parser.add_argument("--workflow", required=True)
    parser.add_argument("--checkout-action-sha", required=True)
    parser.add_argument("--install-nix-action-sha", required=True)
    parser.add_argument("--upload-artifact-action-sha", required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    return parser.parse_args()


def main() -> None:
    report = build(parse_args())
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
