# Portable Cargo.lock Repair Operator Runbook V1

## Purpose

This runbook applies the portable repair-only harness to the current clinical-assurance dependency-graph blocker tracked in P0 #141.

It does **not** qualify the clinical subject and does **not** authorize committing a generated lockfile.

## Exact identities for the current repair

Clinical subject whose dependency graph needs reconciliation:

`97e991a75d0565f1e7769c2a11ac94e0b2f2aded`

Pinned parent Mycelix source declared by the current qualification lineage:

`a85369699099d4c7524e502e531735eed4ab36f4`

Pinned parent tree:

`6f7ff861547fdff31dba9a284f845e7a2a736d7a`

Required clinical package:

`mycelix-symthaea-clinical-distribution-trust-v3`

The portable harness itself is a separate exact Git identity. Use the exact reviewed commit containing this runbook and `scripts/run-portable-lock-candidate.sh`; do not run an edited/dirty harness checkout.

## Preconditions

The operator needs:

- a clean `mycelix-health` Git checkout containing the exact clinical subject object;
- that checkout switched to the exact reviewed portable-harness commit;
- a clean `Luminous-Dynamics/mycelix` checkout at the exact pinned parent commit;
- Nix available;
- Python 3 and Git available;
- enough disk space for disposable detached worktrees and Nix/Cargo evaluation.

The harness checkout itself may be used as `--subject-checkout`; the generator creates a separate detached worktree at the requested clinical subject SHA.

## Prepare the parent checkout

Example:

```bash
PARENT=/srv/luminous-dynamics/mycelix

git -C "$PARENT" fetch --all --prune
git -C "$PARENT" checkout --detach a85369699099d4c7524e502e531735eed4ab36f4

test "$(git -C "$PARENT" rev-parse HEAD)" = \
  a85369699099d4c7524e502e531735eed4ab36f4

test "$(git -C "$PARENT" rev-parse 'HEAD^{tree}')" = \
  6f7ff861547fdff31dba9a284f845e7a2a736d7a

test -z "$(git -C "$PARENT" status --porcelain=v1 --untracked-files=all)"
```

Do not proceed if any identity check fails.

## Run the portable repair harness

From the clean portable-harness checkout:

```bash
HEALTH=/srv/luminous-dynamics/mycelix-health
PARENT=/srv/luminous-dynamics/mycelix
OUT=/tmp/mycelix-health-lock-97e991a7

bash ./scripts/run-portable-lock-candidate.sh \
  --subject-checkout "$HEALTH" \
  --subject-sha 97e991a75d0565f1e7769c2a11ac94e0b2f2aded \
  --parent-checkout "$PARENT" \
  --output-dir "$OUT"
```

The default required-package set already includes:

`mycelix-symthaea-clinical-distribution-trust-v3`

Additional required packages may be bound with repeated `--require-package NAME` arguments.

The adversarial portable-harness tests run first by default. `--skip-self-test` exists only for controlled debugging; evidence intended for review should not skip them.

## Expected generation authorities

Successful generation must produce:

`RepairCandidateOnly`

Successful independent executable verification must produce:

`VerifiedRepairCandidateOnly`

Neither authority means that the dependency graph has been accepted.

## Mandatory candidate review files

Review at minimum:

- `Cargo.lock.candidate`
- `Cargo.lock.diff`
- `lock-audit.json`
- `lock-candidate.json`
- `lock-candidate-verification.json`
- `parent-source-verification.json`
- `lock-movement-review.template.json`
- `REVIEW.md`

The operator must explain every unexpected registry-version or git-source movement before acceptance.

A useful strict default for this repair is:

- new local workspace package entries expected from the clinical-assurance stack: potentially acceptable after inspection;
- unrelated registry version movement: reject unless demonstrated necessary;
- unrelated git dependency movement: reject unless demonstrated necessary;
- package removal unrelated to the clinical stack: reject unless demonstrated necessary.

## Explicit movement disposition

The generated `lock-movement-review.template.json` is editable human input, not final canonical evidence. Every movement begins as `unreviewed`.

For every item, replace `unreviewed` with exactly one of:

- `expected`
- `necessary`
- `accepted`
- `rejected`

and provide a non-empty rationale. `review_reference` may identify an issue/comment/note used during review, but v1 treats it as unverified context rather than authenticated reviewer identity.

After editing, seal the draft:

```bash
python3 ./scripts/seal-lock-movement-review.py \
  --candidate-dir "$OUT" \
  --draft "$OUT/lock-movement-review.template.json" \
  --output "$OUT/lock-movement-review.statement.json"
```

A successful seal emits:

`ReviewStatementOnly`

Then independently reconstruct exact coverage:

```bash
python3 ./scripts/verify-lock-movement-review.py \
  --candidate-dir "$OUT" \
  --review "$OUT/lock-movement-review.statement.json" \
  --verification "$OUT/lock-movement-review-verification.json"
```

A complete statement emits:

`MechanicalReviewCoverageComplete`

If the intended acceptance policy cannot tolerate any explicitly rejected movement, add:

`--require-no-rejections`

The hierarchy is deliberately conservative:

```text
ReviewTemplateOnly
        -> human edits
ReviewStatementOnly
        -> independent reconstruction
MechanicalReviewCoverageComplete
        != authenticated reviewer
        != dependency-graph approval
        != permission to commit
```

See `docs/security/LOCK_MOVEMENT_REVIEW_V1.md` for the complete boundary.

## Acceptance is a separate operation

The portable harness deliberately does **not** copy the candidate into `Cargo.lock`.

Only after the candidate, audit, sealed movement statement, mechanical coverage result, and rationales have been inspected should a human make the separate decision whether the exact candidate bytes are acceptable.

If accepted, create a new branch from the exact clinical subject and copy the exact reviewed candidate bytes into that branch. Then commit the new `Cargo.lock` as a new immutable subject.

Conceptually:

```text
97e991a7... clinical subject
        +
VerifiedRepairCandidateOnly
        +
ReviewStatementOnly
        +
MechanicalReviewCoverageComplete
        +
separate human acceptance decision
        |
        v
NEW branch from 97e991a7...
        |
copy exact Cargo.lock.candidate bytes
        |
commit
        v
NEW immutable lock-bearing subject
        |
        v
fresh --locked exact-head qualification
```

The old `97e991a7...` subject remains unmodified and unqualified.

## Post-acceptance qualification

The replacement subject must rerun the focused exact-head qualification with:

- exact subject checkout;
- exact parent-source identity;
- committed `Cargo.lock`;
- `cargo metadata --locked` as a preflight;
- all focused v3 prerequisite tests;
- postflight proof that `Cargo.lock` remained byte-identical.

Only an actually executed successful qualification may establish software qualification evidence.

## Non-claims

Running this runbook does not establish:

- authenticated human reviewer identity;
- organizational approval;
- a repository qualification PASS;
- scientific validity of a Symthaea prediction or decision rule;
- calibration or OOD detector adequacy;
- clinical effectiveness or safety;
- clinician or patient presentation authority;
- diagnosis, prescribing, dispensing, administration, or treatment authority;
- regulatory or legal compliance, clearance, or approval.

The runbook exists only to create and review an independently verified dependency-repair candidate without mutating the original clinical subject.