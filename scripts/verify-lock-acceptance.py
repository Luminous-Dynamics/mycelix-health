#!/usr/bin/env python3
"""Verify a signed dependency-graph acceptance statement with OpenSSH.

The statement is signed externally with:

  ssh-keygen -Y sign -f <private-key> -n mycelix-health-lock-acceptance-v1 <statement.json>

Verification uses an external OpenSSH allowed_signers policy. The policy is not
hard-coded into the clinical subject; its exact bytes are hashed into the
verification receipt.

Terminal authority: VerifiedHumanAcceptanceAttestationOnly.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import subprocess
from typing import Any, NoReturn

SCHEMA = "mycelix-health-lock-acceptance-verification/v1"
STATEMENT_SCHEMA = "mycelix-health-lock-acceptance-statement/v1"
NAMESPACE = "mycelix-health-lock-acceptance-v1"


def fail(message: str) -> NoReturn:
    raise SystemExit(f"lock acceptance verification failed: {message}")


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def load_canonical(path: pathlib.Path, *, label: str) -> tuple[dict[str, Any], bytes]:
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot load {label} {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must contain one JSON object")
    if raw != canonical_bytes(value):
        fail(f"{label} is not canonical JSON")
    return value, raw


def read_sidecar(path: pathlib.Path) -> str:
    sidecar = pathlib.Path(str(path) + ".sha256")
    try:
        value = sidecar.read_text(encoding="utf-8").strip().lower()
    except OSError as error:
        fail(f"cannot read sidecar {sidecar}: {error}")
    if len(value) != 64 or any(ch not in "0123456789abcdef" for ch in value):
        fail(f"invalid SHA-256 sidecar: {sidecar}")
    return value


def write_create_only(path: pathlib.Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() or path.is_symlink():
        fail(f"refusing to overwrite acceptance verification: {path}")
    try:
        with path.open("xb") as handle:
            handle.write(data)
    except FileExistsError:
        fail(f"refusing to overwrite acceptance verification: {path}")


def run_verify(statement: bytes, *, signature: pathlib.Path, allowed_signers: pathlib.Path, principal: str) -> str:
    try:
        result = subprocess.run(
            [
                "ssh-keygen",
                "-Y",
                "verify",
                "-f",
                str(allowed_signers),
                "-I",
                principal,
                "-n",
                NAMESPACE,
                "-s",
                str(signature),
            ],
            input=statement,
            capture_output=True,
            check=False,
        )
    except OSError as error:
        fail(f"cannot execute ssh-keygen: {error}")
    if result.returncode != 0:
        fail(
            "OpenSSH signature verification failed"
            + f"\nstdout:\n{result.stdout.decode(errors='replace')}"
            + f"\nstderr:\n{result.stderr.decode(errors='replace')}"
        )
    return result.stdout.decode(errors="replace").strip()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--statement", type=pathlib.Path, required=True)
    parser.add_argument("--signature", type=pathlib.Path, required=True)
    parser.add_argument("--allowed-signers", type=pathlib.Path, required=True)
    parser.add_argument("--principal", required=True)
    parser.add_argument("--verification", type=pathlib.Path, required=True)
    args = parser.parse_args()

    statement_path = args.statement.resolve()
    signature_path = args.signature.resolve()
    allowed_signers_path = args.allowed_signers.resolve()
    output = args.verification.resolve()
    principal = args.principal.strip()

    if not principal or any(ch.isspace() for ch in principal):
        fail("principal must be one non-empty whitespace-free identifier")
    if not signature_path.is_file():
        fail(f"signature does not exist: {signature_path}")
    if not allowed_signers_path.is_file():
        fail(f"allowed_signers policy does not exist: {allowed_signers_path}")

    statement, statement_raw = load_canonical(statement_path, label="acceptance statement")
    if read_sidecar(statement_path) != sha256_bytes(statement_raw):
        fail("acceptance statement sidecar mismatch")
    if statement.get("schema") != STATEMENT_SCHEMA:
        fail("unsupported acceptance statement schema")
    if statement.get("authority") != "AcceptanceStatementToSignOnly":
        fail("acceptance statement authority must be AcceptanceStatementToSignOnly")
    if statement.get("signature_namespace") != NAMESPACE:
        fail("acceptance statement signature namespace mismatch")
    if statement.get("principal") != principal:
        fail("requested verifier principal differs from signed statement principal")
    if statement.get("decision") != "accept_exact_candidate_lock_for_replacement_subject_preparation":
        fail("acceptance statement decision is not the supported v1 decision")

    signature_bytes = signature_path.read_bytes()
    allowed_signers_bytes = allowed_signers_path.read_bytes()
    if not signature_bytes:
        fail("signature is empty")
    if not allowed_signers_bytes:
        fail("allowed_signers policy is empty")

    verify_stdout = run_verify(
        statement_raw,
        signature=signature_path,
        allowed_signers=allowed_signers_path,
        principal=principal,
    )

    verification = {
        "schema": SCHEMA,
        "authority": "VerifiedHumanAcceptanceAttestationOnly",
        "result": "pass",
        "signature_namespace": NAMESPACE,
        "principal": principal,
        "statement_sha256": sha256_bytes(statement_raw),
        "signature_sha256": sha256_bytes(signature_bytes),
        "allowed_signers_sha256": sha256_bytes(allowed_signers_bytes),
        "subject": statement.get("subject"),
        "harness": statement.get("harness"),
        "locks": statement.get("locks"),
        "evidence": statement.get("evidence"),
        "openssh_verification_output": verify_stdout,
        "non_claims": [
            "VerifiedHumanAcceptanceAttestationOnly authenticates the configured OpenSSH principal under the supplied allowed_signers policy; it does not prove organizational role beyond that policy.",
            "Dependency acceptance is not software qualification, scientific validation, or clinical authority.",
            "The exact lock-bearing replacement commit must still be identity-verified and then qualified with fresh --locked execution.",
        ],
    }
    raw = canonical_bytes(verification)
    write_create_only(output, raw)
    write_create_only(pathlib.Path(str(output) + ".sha256"), (sha256_bytes(raw) + "\n").encode())
    print(json.dumps(verification, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
