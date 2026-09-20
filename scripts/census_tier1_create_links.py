#!/usr/bin/env python3
"""Enumerate create_link(...) constructions in coordinators packaged by dna/dna.yaml.

This scanner is intentionally narrower than a Rust parser but handles the syntax
needed for a reproducible construction census:
- derives packaged coordinator zome names from the DNA manifest;
- ignores strings and comments while finding functions/calls/delimiters;
- attributes each create_link call to its enclosing function;
- splits the four top-level create_link arguments;
- extracts the LinkTypes variant;
- emits stable JSON with source line + normalized expressions.

The output is raw characterization evidence. It does not decide whether a link
construction is private/safe and it does not prove DHT replication semantics.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "dna" / "dna.yaml"


def fail(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def packaged_coordinators() -> list[str]:
    text = MANIFEST.read_text(encoding="utf-8")
    if "\ncoordinator:\n" not in text:
        fail("dna manifest has no coordinator section")
    section = text.split("\ncoordinator:\n", 1)[1]
    # Only top-level list items under coordinator.zomes have four spaces before
    # `- name:` in the current manifest. Dependency names are indented deeper.
    names = re.findall(r"^    - name:\s+([A-Za-z0-9_]+)\s*$", section, re.MULTILINE)
    if not names:
        fail("no packaged coordinator zomes found")
    if len(names) != len(set(names)):
        fail("duplicate coordinator names in manifest")
    return names


def source_path(zome: str) -> Path:
    path = ROOT / "zomes" / zome / "coordinator" / "src" / "lib.rs"
    if not path.is_file():
        fail(f"packaged coordinator source not found: {path.relative_to(ROOT)}")
    return path


def mask_noncode(text: str) -> str:
    """Replace strings/chars/comments with spaces while preserving newlines/length."""
    out = list(text)
    i = 0
    n = len(text)
    state = "code"
    block_depth = 0
    while i < n:
        if state == "code":
            if text.startswith("//", i):
                out[i] = out[i + 1] = " "
                i += 2
                state = "line_comment"
                continue
            if text.startswith("/*", i):
                out[i] = out[i + 1] = " "
                i += 2
                block_depth = 1
                state = "block_comment"
                continue
            if text[i] == '"':
                out[i] = " "
                i += 1
                state = "string"
                continue
            if text[i] == "'":
                # Rust lifetimes (`'a`) should remain code. Treat a quote as a
                # char literal only when a closing quote is nearby.
                close = i + 2
                if close < n and text[close] == "'":
                    out[i] = " "
                    i += 1
                    state = "char"
                    continue
                if i + 3 < n and text[i + 1] == "\\" and text[i + 3] == "'":
                    out[i] = " "
                    i += 1
                    state = "char"
                    continue
            i += 1
            continue

        if state == "line_comment":
            if text[i] == "\n":
                state = "code"
            else:
                out[i] = " "
            i += 1
            continue

        if state == "block_comment":
            if text.startswith("/*", i):
                out[i] = out[i + 1] = " "
                block_depth += 1
                i += 2
                continue
            if text.startswith("*/", i):
                out[i] = out[i + 1] = " "
                block_depth -= 1
                i += 2
                if block_depth == 0:
                    state = "code"
                continue
            if text[i] != "\n":
                out[i] = " "
            i += 1
            continue

        if state in {"string", "char"}:
            quote = '"' if state == "string" else "'"
            if text[i] == "\\":
                if text[i] != "\n":
                    out[i] = " "
                if i + 1 < n:
                    if text[i + 1] != "\n":
                        out[i + 1] = " "
                    i += 2
                else:
                    i += 1
                continue
            if text[i] == quote:
                out[i] = " "
                i += 1
                state = "code"
                continue
            if text[i] != "\n":
                out[i] = " "
            i += 1
            continue

    if state in {"string", "char", "block_comment"}:
        fail(f"unterminated {state} while scanning Rust source")
    return "".join(out)


def match_delimiter(masked: str, open_index: int, opener: str, closer: str) -> int:
    if masked[open_index] != opener:
        raise ValueError("open_index does not point to requested delimiter")
    depth = 0
    for i in range(open_index, len(masked)):
        char = masked[i]
        if char == opener:
            depth += 1
        elif char == closer:
            depth -= 1
            if depth == 0:
                return i
    fail(f"unmatched {opener} at byte offset {open_index}")
    raise AssertionError("unreachable")


def function_ranges(masked: str) -> list[tuple[int, int, str]]:
    functions: list[tuple[int, int, str]] = []
    pattern = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(")
    for match in pattern.finditer(masked):
        # Find the function body opening brace after the signature. This skips
        # return types / where clauses without needing to parse them.
        brace = masked.find("{", match.end())
        semicolon = masked.find(";", match.end())
        if brace < 0 or (semicolon >= 0 and semicolon < brace):
            continue
        end = match_delimiter(masked, brace, "{", "}")
        functions.append((brace, end, match.group(1)))
    return functions


def enclosing_function(functions: list[tuple[int, int, str]], offset: int) -> str:
    candidates = [(start, end, name) for start, end, name in functions if start <= offset <= end]
    if not candidates:
        return "<module>"
    # Nested functions are legal; choose the narrowest enclosing body.
    _, _, name = min(candidates, key=lambda item: item[1] - item[0])
    return name


def split_args(original: str, masked: str, open_index: int, close_index: int) -> list[str]:
    args: list[str] = []
    start = open_index + 1
    paren = bracket = brace = 0
    i = start
    while i < close_index:
        char = masked[i]
        if char == "(":
            paren += 1
        elif char == ")":
            paren -= 1
        elif char == "[":
            bracket += 1
        elif char == "]":
            bracket -= 1
        elif char == "{":
            brace += 1
        elif char == "}":
            brace -= 1
        elif char == "," and paren == bracket == brace == 0:
            args.append(original[start:i].strip())
            start = i + 1
        i += 1
    tail = original[start:close_index].strip()
    if tail:
        args.append(tail)
    return args


def normalize_expr(expr: str) -> str:
    return " ".join(expr.split())


def scan_source(zome: str, path: Path) -> list[dict]:
    text = path.read_text(encoding="utf-8")
    masked = mask_noncode(text)
    functions = function_ranges(masked)
    calls: list[dict] = []
    per_function: dict[str, int] = {}

    pattern = re.compile(r"\bcreate_link\s*\(")
    for match in pattern.finditer(masked):
        open_index = masked.find("(", match.start(), match.end())
        close_index = match_delimiter(masked, open_index, "(", ")")
        args = split_args(text, masked, open_index, close_index)
        if len(args) != 4:
            fail(
                f"{path.relative_to(ROOT)}:{text.count(chr(10), 0, match.start()) + 1}: "
                f"create_link has {len(args)} top-level arguments, expected 4"
            )
        link_match = re.search(r"\bLinkTypes::([A-Za-z_][A-Za-z0-9_]*)\b", args[2])
        if not link_match:
            fail(
                f"{path.relative_to(ROOT)}:{text.count(chr(10), 0, match.start()) + 1}: "
                "third create_link argument does not contain LinkTypes::<Variant>"
            )
        function = enclosing_function(functions, match.start())
        ordinal = per_function.get(function, 0) + 1
        per_function[function] = ordinal
        calls.append(
            {
                "callsite_id": f"{zome}:{function}:{ordinal}",
                "zome": zome,
                "function": function,
                "ordinal_in_function": ordinal,
                "line": text.count("\n", 0, match.start()) + 1,
                "link_type": link_match.group(1),
                "base_expr": normalize_expr(args[0]),
                "target_expr": normalize_expr(args[1]),
                "tag_expr": normalize_expr(args[3]),
            }
        )
    return calls


def census() -> dict:
    zomes = packaged_coordinators()
    all_calls: list[dict] = []
    counts: dict[str, int] = {}
    for zome in zomes:
        calls = scan_source(zome, source_path(zome))
        counts[zome] = len(calls)
        all_calls.extend(calls)
    return {
        "schema_version": 1,
        "dna_manifest": str(MANIFEST.relative_to(ROOT)),
        "coordinator_count": len(zomes),
        "create_link_call_count": len(all_calls),
        "counts_by_zome": counts,
        "calls": all_calls,
    }


def self_test() -> None:
    sample = r'''
fn public_directory(provider: ActionHash) {
    let a = anchor_hash(&format!("specialty_{}", "cardiology"))?;
    create_link(a, provider, LinkTypes::AllProviders, ())?;
}

fn protected(did: String, patient: ActionHash, target: ActionHash) {
    // create_link(fake, fake, LinkTypes::Ignored, ());
    let label = "create_link(fake, fake, LinkTypes::IgnoredString, ())";
    create_link(
        patient.clone(),
        target,
        LinkTypes::PatientToDID,
        did.as_bytes().to_vec(),
    )?;
}
'''
    masked = mask_noncode(sample)
    functions = function_ranges(masked)
    calls = []
    per_function: dict[str, int] = {}
    for match in re.finditer(r"\bcreate_link\s*\(", masked):
        open_index = masked.find("(", match.start(), match.end())
        close_index = match_delimiter(masked, open_index, "(", ")")
        args = split_args(sample, masked, open_index, close_index)
        function = enclosing_function(functions, match.start())
        ordinal = per_function.get(function, 0) + 1
        per_function[function] = ordinal
        link_match = re.search(r"LinkTypes::([A-Za-z_][A-Za-z0-9_]*)", args[2])
        assert link_match is not None
        calls.append((function, ordinal, link_match.group(1), normalize_expr(args[3])))

    expected = [
        ("public_directory", 1, "AllProviders", "()"),
        ("protected", 1, "PatientToDID", "did.as_bytes().to_vec()"),
    ]
    if calls != expected:
        fail(f"self-test census mismatch: {calls!r} != {expected!r}")
    print("create_link census parser self-test: PASS")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--pretty", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        return

    result = census()
    json.dump(result, sys.stdout, indent=2 if args.pretty else None, sort_keys=True)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
