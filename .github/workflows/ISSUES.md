# CI issues — known broken / paused

Living list of CI workflow problems that are **not** code bugs and
**not** prioritised right now. Add a row when you find one; remove it
when it's fixed.

The point of this file: when a PR's GitHub check turns red, the
reviewer should be able to check here in 30 seconds and tell whether
the failure is (a) something the PR introduced or (b) something
master has been red on for unrelated reasons. Without this list,
every red check chases its own investigation.

If you reproduce a failure locally and it doesn't match anything
here, it's a real bug — open a real issue.

---

## Active

### I1 — `pnpm (build / typecheck)` — `ERR_PNPM_BAD_PM_VERSION`

| | |
|---|---|
| First seen | 2026-05-21, master after PR #16 merge |
| Workflow | `.github/workflows/ci.yml` job `pnpm` |
| Symptom | `Error: Multiple versions of pnpm specified: Remove one of these versions to avoid version mismatch errors like ERR_PNPM_BAD_PM_VERSION` at the `pnpm/action-setup@v4` step |
| Root cause | The workflow pins `with: version: 9` at line 96/237 of `ci.yml`, and `package.json` also declares `"packageManager": "pnpm@9.0.0"`. `pnpm/action-setup@v4` treats the two as conflicting (even though both name 9.x) and refuses to install. |
| Blast radius | Every PR shows `pnpm (build / typecheck)` red. Inherited by every job that depends on the pnpm step (see I2). |
| Fix | Remove the `with: version: 9` block from both `pnpm/action-setup@v4` uses in `ci.yml`, defer to `packageManager`. One-line edit; ~5 min including a verify-on-CI loop. |
| Why deferred | Cosmetic — the workflow has been red on master since before any current PR. Not blocking work. |

### I2 — `openapi / ts drift` — same `ERR_PNPM_BAD_PM_VERSION`

| | |
|---|---|
| First seen | 2026-05-21, same master commits as I1 |
| Workflow | `.github/workflows/ci.yml` job `openapi-ts-drift` |
| Symptom | Identical to I1 — fails at `pnpm/action-setup@v4`. |
| Root cause | Same as I1; this job uses the same setup block. |
| Fix | Lands with I1 (same edit, both jobs). |

### I3 — `rust (check / clippy / test)` — pre-existing clippy errors

| | |
|---|---|
| First seen | 2026-05-21, master after PR #15 merge |
| Workflow | `.github/workflows/ci.yml` job `rust` |
| Symptom | `cargo clippy --workspace --all-targets -- -D warnings` fails on assorted lints (clone-on-Copy, MSRV violation, manual impl Default, etc.). |
| Root cause | Lint debt accumulated across `starter-flow-spi`, `starter-ui-ir`, `starter-ui-bindings`, `starter-skills`, `starter-flow`, and the `starter-extensions` workspace. The largest single contributor was a stale `rust-version = "1.78"` declaration in both `Cargo.toml`s while the code uses `std::sync::LazyLock` (stable in 1.80). |
| Fix | **Resolved on branch `chore/ci-green-master`** — five commits across the affected crates. Workspace clippy is clean locally after that branch lands. PR for review. |
| Why noted here | Tracking the diagnosis trail so the next person who sees this failure knows it's the lint-debt branch, not new breakage. |

### I4 — `cargo test (per-crate sweep)` — `starter-extensions` workflow

| | |
|---|---|
| First seen | 2026-05-21, master |
| Workflow | `.github/workflows/starter-extensions.yml` |
| Symptom | Per-crate test sweep against the `starter-extensions` workspace fails to compile / test cleanly. |
| Root cause | Same lint debt as I3, plus dead-code/unused-method lints emitted from inside `requires!{}` macro expansion in `starter-ext-sdk`. |
| Fix | Resolved on the same `chore/ci-green-master` branch as I3 (12 fixes across 8 starter-extensions crates). |
| Why noted here | The two CI red signals (I3 + I4) share one branch and one PR — don't chase them independently. |

### I5 — `starter-spi dep baseline` — `sha1_smol` drift

| | |
|---|---|
| First seen | 2026-05-21, master |
| Workflow | `.github/workflows/ci.yml` job `spi-dep-baseline` |
| Symptom | `scripts/check-spi-dep-baseline.sh` reports `+sha1_smol v1.0.1` against the committed baseline. |
| Root cause | `uuid v1.23.1` added an internal `sha1_smol` dep. `uuid` is a legitimate direct dep of `starter-spi`, so this is the script's "legitimate change to starter-spi's direct deps" path, not a provider-crate leak. |
| Fix | Baseline regenerated via `scripts/check-spi-dep-baseline.sh --update`. Committed on `chore/ci-green-master` (commit `c842c81`). |

---

## Resolved

(Move I-rows here as they land. Each gets a one-line "fixed in
`<PR # or commit>` on `<date>`" so the diagnostic context is still
findable from a later red CI run on a similar pattern.)

*(none yet)*

---

## Conventions

- **One row per failing CI signal.** If two checks fail for the same
  root cause, file them separately and cross-reference (see I1 / I2)
  — separate rows make it obvious when one fix closes both.
- **Don't speculate.** A row should name the workflow, the failing
  step, the symptom verbatim from logs, and either a known root
  cause or "unknown — investigation deferred." "Probably caused by
  X" without a reproduction is worse than nothing.
- **Add the fix branch / PR.** If a row's resolution is in flight,
  point at the branch / PR so the next reviewer doesn't redo the
  diagnostic work.
- **Move resolved rows down.** Don't delete them — the next time a
  similar failure appears, the past resolution is the fastest
  diagnostic path.
- **This file is not a substitute for tracking real bugs.** If a CI
  failure surfaces an actual code bug, open a GitHub Issue or add a
  TODO in the offending file; this file is for "the workflow itself
  is broken / paused" cases only.
