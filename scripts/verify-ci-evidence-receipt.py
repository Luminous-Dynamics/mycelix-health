#!/usr/bin/env python3
"""Independently verify canonical repository CI evidence receipts v1.

This verifier does not import the receipt builder. It independently re-binds the
product subject, reviewed harness, executing workflow, source-verification report,
committed build inputs, Nix environment evidence, and GitHub Actions execution
identity before emitting a create-only verification result.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import subprocess
from typing import Any, NoReturn

RECEIPT_SCHEMA = "mycelix-health-ci-evidence/v1"
SOURCE_VERIFICATION_SCHEMA = "mycelix-health-parent-source-verification/v1"
VERIFICATION_SCHEMA = "mycelix-health-ci-evidence-verification/v1"
REPOSITORY = "Luminous-Dynamics/mycelix-health"
HEX = frozenset("0123456789abcdefABCDEF")


def fail(message: str) -> NoReturn:
    raise SystemExit(f"CI evidence verification failed: {message}")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    try:
        return sha256_bytes(path.read_bytes())
    except OSError as error:
        fail(f"cannot hash required file {path}: {error}")


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def full_sha(value: Any, *, label: str) -> str:
    if not isinstance(value, str) or len(value) != 40 or any(char not in HEX for char in value):
        fail(f"{label} must be a full 40-character hexadecimal Git SHA")
    return value.lower()


def full_digest(value: Any, *, label: str) -> str:
    if not isinstance(value, str) or len(value) != 64 or any(char not in HEX for char in value):
        fail(f"{label} must be a hexadecimal SHA-256 digest")
    return value.lower()


def load_object(path: pathlib.Path, *, label: str) -> tuple[dict[str, Any], bytes]:
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {label} {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must be a JSON object")
    return value, raw


def sidecar_digest(path: pathlib.Path, *, label: str) -> str:
    sidecar = pathlib.Path(str(path) + ".sha256")
    try:
        raw = sidecar.read_text(encoding="utf-8").strip().split()[0]
    except (OSError, IndexError) as error:
        fail(f"cannot read {label} sidecar {sidecar}: {error}")
    expected = full_digest(raw, label=f"{label} sidecar")
    actual = sha256_file(path)
    if actual != expected:
        fail(f"{label} sidecar digest mismatch")
    return actual


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


def require_digest_field(container: dict[str, Any], key: str, actual: str, *, label: str) -> None:
    expected = full_digest(container.get(key), label=f"{label}.{key}")
    if expected != actual:
        fail(f"{label}.{key} does not match independently computed bytes")


def verify(args: argparse.Namespace) -> dict[str, Any]:
    if required_env("GITHUB_ACTIONS").lower() != "true":
        fail("authoritative CI verification may only run inside GitHub Actions")
    if required_env("GITHUB_REPOSITORY").lower() != REPOSITORY.lower():
        fail("CI evidence verification is bound to the mycelix-health repository")
    if required_env("GITHUB_EVENT_NAME") != "workflow_dispatch":
        fail("CI evidence verification v1 requires workflow_dispatch")

    workspace = args.workspace.resolve()
    harness = args.harness_root.resolve()
    evidence = args.evidence_dir.resolve()
    receipt_path = args.receipt.resolve()
    output = args.verification.resolve()
    require_outside(workspace, evidence, label="evidence directory")
    require_outside(harness, evidence, label="evidence directory")
    try:
        receipt_path.relative_to(evidence)
        output.relative_to(evidence)
    except ValueError:
        fail("receipt and verification artifacts must remain inside the evidence directory")

    receipt, receipt_raw = load_object(receipt_path, label="CI receipt")
    if receipt.get("schema") != RECEIPT_SCHEMA:
        fail(f"unsupported CI receipt schema: {receipt.get('schema')!r}")
    if receipt_raw != canonical_bytes(receipt):
        fail("CI receipt is not canonical JSON")
    receipt_sha = sidecar_digest(receipt_path, label="CI receipt")

    mode = receipt.get("mode")
    if mode == "pinned":
        expected_authority = "ReproducibleRepositoryCIEvidence"
        verified_authority = "VerifiedReproducibleRepositoryCIEvidence"
        expected_source_authority = "VerifiedPinnedDependencySourceMaterialization"
    elif mode == "compatibility":
        expected_authority = "CompatibilityOnly"
        verified_authority = "VerifiedCompatibilityOnlyObservation"
        expected_source_authority = "VerifiedCompatibilityObservation"
    else:
        fail(f"unsupported CI receipt mode: {mode!r}")
    if receipt.get("authority") != expected_authority or receipt.get("result") != "pass":
        fail("CI receipt authority/result does not match its mode")
    if receipt.get("repository") != REPOSITORY:
        fail("CI receipt repository identity mismatch")

    subject = receipt.get("subject")
    harness_receipt = receipt.get("harness")
    build_inputs = receipt.get("build_inputs")
    execution = receipt.get("execution")
    source_binding = receipt.get("source_verification")
    for value, label in (
        (subject, "subject"),
        (harness_receipt, "harness"),
        (build_inputs, "build_inputs"),
        (execution, "execution"),
        (source_binding, "source_verification"),
    ):
        if not isinstance(value, dict):
            fail(f"CI receipt {label} object is missing")

    declared_subject = full_sha(subject.get("sha"), label="receipt subject SHA")
    declared_harness = full_sha(harness_receipt.get("sha"), label="receipt harness SHA")
    if declared_subject != full_sha(args.subject_sha, label="expected subject SHA"):
        fail("receipt subject SHA differs from verifier expectation")
    if declared_harness != full_sha(args.harness_sha, label="expected harness SHA"):
        fail("receipt harness SHA differs from verifier expectation")
    if git_head(workspace, label="observed subject") != declared_subject:
        fail("checked-out product HEAD differs from receipt subject")
    if git_head(harness, label="observed harness") != declared_harness:
        fail("checked-out harness HEAD differs from receipt harness")
    require_product_tracked_clean(workspace)
    require_harness_fully_clean(harness)
    if full_sha(subject.get("tree_sha"), label="receipt subject tree") != git_tree(workspace, label="observed subject tree"):
        fail("product tree SHA differs from CI receipt")
    if full_sha(harness_receipt.get("tree_sha"), label="receipt harness tree") != git_tree(harness, label="observed harness tree"):
        fail("harness tree SHA differs from CI receipt")

    workflow_sha = full_sha(required_env("GITHUB_WORKFLOW_SHA"), label="executing workflow SHA")
    if workflow_sha != declared_harness:
        fail("executing workflow SHA differs from receipt harness")
    workflow_path = harness_receipt.get("workflow_path")
    if workflow_path != args.workflow:
        fail("receipt workflow path differs from verifier expectation")
    workflow_ref = required_env("GITHUB_WORKFLOW_REF")
    if harness_receipt.get("workflow_ref") != workflow_ref:
        fail("receipt workflow ref differs from executing workflow ref")
    expected_marker = f"/{args.workflow}@"
    if not workflow_ref.startswith(f"{REPOSITORY}/") or expected_marker not in workflow_ref:
        fail("executing workflow ref is not the expected reviewed workflow")
    workflow = (harness / args.workflow).resolve()
    try:
        workflow.relative_to(harness)
    except ValueError:
        fail("workflow path escapes harness checkout")
    require_digest_field(harness_receipt, "workflow_sha256", sha256_file(workflow), label="harness")

    materializer = harness / "scripts/materialize-parent-mycelix.py"
    source_verifier = harness / "scripts/verify-parent-source-materialization.py"
    receipt_builder = harness / "scripts/build-ci-evidence-receipt.py"
    receipt_verifier = pathlib.Path(__file__).resolve()
    require_digest_field(harness_receipt, "materializer_sha256", sha256_file(materializer), label="harness")
    require_digest_field(harness_receipt, "source_verifier_sha256", sha256_file(source_verifier), label="harness")
    require_digest_field(harness_receipt, "receipt_builder_sha256", sha256_file(receipt_builder), label="harness")
    require_digest_field(harness_receipt, "receipt_verifier_sha256", sha256_file(receipt_verifier), label="harness")

    action_pins = harness_receipt.get("action_pins")
    if not isinstance(action_pins, dict):
        fail("receipt action_pins object is missing")
    workflow_text = workflow.read_text(encoding="utf-8")
    for key, prefix in (
        ("checkout", "actions/checkout@"),
        ("install_nix", "cachix/install-nix-action@"),
        ("upload_artifact", "actions/upload-artifact@"),
    ):
        pin = full_sha(action_pins.get(key), label=f"action pin {key}")
        if f"{prefix}{pin}" not in workflow_text:
            fail(f"receipt action pin {key} is not present in exact workflow bytes")

    cargo_lock = workspace / "Cargo.lock"
    flake_lock = workspace / "flake.lock"
    rust_toolchain = workspace / "rust-toolchain.toml"
    profile = workspace / "release/ci-parent-source-profile-v1.json"
    cargo_metadata = evidence / "cargo-metadata.json"
    flake_metadata = evidence / "flake-metadata.json"
    ci_drv = evidence / "ci-dev-shell.drv"
    for required in (cargo_lock, flake_lock, rust_toolchain, profile, cargo_metadata, flake_metadata, ci_drv):
        if not required.is_file():
            fail(f"required build/evidence input is missing: {required}")
    require_digest_field(build_inputs, "cargo_lock_sha256", sha256_file(cargo_lock), label="build_inputs")
    require_digest_field(build_inputs, "flake_lock_sha256", sha256_file(flake_lock), label="build_inputs")
    require_digest_field(build_inputs, "rust_toolchain_sha256", sha256_file(rust_toolchain), label="build_inputs")
    require_digest_field(build_inputs, "parent_profile_sha256", sha256_file(profile), label="build_inputs")
    require_digest_field(build_inputs, "cargo_metadata_sha256", sha256_file(cargo_metadata), label="build_inputs")
    require_digest_field(build_inputs, "flake_metadata_sha256", sha256_file(flake_metadata), label="build_inputs")
    try:
        ci_drv_value = ci_drv.read_text(encoding="utf-8").strip()
    except OSError as error:
        fail(f"cannot read CI derivation identity: {error}")
    if not ci_drv_value or build_inputs.get("ci_dev_shell_drv") != ci_drv_value:
        fail("CI dev-shell derivation differs from receipt")

    source_report_path = evidence / args.source_verification
    source_report, source_raw = load_object(source_report_path, label="source verification")
    if source_report.get("schema") != SOURCE_VERIFICATION_SCHEMA:
        fail("unexpected source verification schema")
    if source_raw != canonical_bytes(source_report):
        fail("source verification report is not canonical JSON")
    source_sha = sidecar_digest(source_report_path, label="source verification")
    if full_digest(source_binding.get("sha256"), label="receipt source verification digest") != source_sha:
        fail("source verification digest differs from CI receipt")
    if source_report.get("authority") != expected_source_authority:
        fail("source verification authority differs from expected CI mode")
    if source_binding.get("authority") != expected_source_authority:
        fail("CI receipt source authority differs from source verification report")
    if source_report.get("mode") != mode:
        fail("source verification mode differs from CI receipt mode")
    if source_report.get("profile_sha256") != sha256_file(profile):
        fail("source verification profile digest differs from product profile")
    if source_report.get("materializer_sha256") != sha256_file(materializer):
        fail("source verification materializer digest differs from harness")
    if source_report.get("verifier_sha256") != sha256_file(source_verifier):
        fail("source verification verifier digest differs from harness")
    if source_binding.get("observed_source") != source_report.get("observed_source"):
        fail("receipt/source report observed source mismatch")
    if source_binding.get("exports") != source_report.get("exports"):
        fail("receipt/source report export set mismatch")

    expected_execution = {
        "event_name": required_env("GITHUB_EVENT_NAME"),
        "run_id": required_env("GITHUB_RUN_ID"),
        "run_attempt": required_env("GITHUB_RUN_ATTEMPT"),
        "runner_os": required_env("RUNNER_OS"),
        "runner_arch": required_env("RUNNER_ARCH"),
        "runner_image_os": optional_env("ImageOS"),
        "runner_image_version": optional_env("ImageVersion"),
    }
    if execution != expected_execution:
        fail("receipt execution identity differs from current GitHub Actions execution")

    report = {
        "schema": VERIFICATION_SCHEMA,
        "authority": verified_authority,
        "result": "pass",
        "mode": mode,
        "ci_receipt_sha256": receipt_sha,
        "subject": {
            "sha": declared_subject,
            "tree_sha": subject["tree_sha"],
        },
        "harness": {
            "sha": declared_harness,
            "tree_sha": harness_receipt["tree_sha"],
            "workflow_path": args.workflow,
            "workflow_sha256": harness_receipt["workflow_sha256"],
            "receipt_verifier_sha256": sha256_file(receipt_verifier),
        },
        "source_verification_sha256": source_sha,
        "build_inputs": {
            "cargo_lock_sha256": build_inputs["cargo_lock_sha256"],
            "flake_lock_sha256": build_inputs["flake_lock_sha256"],
            "rust_toolchain_sha256": build_inputs["rust_toolchain_sha256"],
            "ci_dev_shell_drv": build_inputs["ci_dev_shell_drv"],
        },
        "execution": expected_execution,
        "non_claims": [
            "Verification of repository CI evidence is not clinical validation, calibration, effectiveness, or safety evidence.",
            "This verification grants no clinician/patient presentation, diagnosis, prescribing, dispensing, administration, or treatment authority.",
            "CompatibilityOnly verification cannot satisfy pinned qualification or promotion prerequisites.",
        ],
    }
    output_sidecar = pathlib.Path(str(output) + ".sha256")
    for path in (output, output_sidecar):
        if path.exists() or path.is_symlink():
            fail(f"refusing to overwrite CI evidence verification artifact: {path}")
    report_bytes = canonical_bytes(report)
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("xb") as handle:
        handle.write(report_bytes)
    with output_sidecar.open("xb") as handle:
        handle.write((sha256_bytes(report_bytes) + "\n").encode())
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--subject-sha", required=True)
    parser.add_argument("--harness-sha", required=True)
    parser.add_argument("--workspace", type=pathlib.Path, required=True)
    parser.add_argument("--harness-root", type=pathlib.Path, required=True)
    parser.add_argument("--evidence-dir", type=pathlib.Path, required=True)
    parser.add_argument("--receipt", type=pathlib.Path, required=True)
    parser.add_argument("--source-verification", default="parent-source-verification.json")
    parser.add_argument("--workflow", required=True)
    parser.add_argument("--verification", type=pathlib.Path, required=True)
    return parser.parse_args()


def main() -> None:
    report = verify(parse_args())
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
