# Workflow — starter-prefs-spi

How to drive this job. Shape: lock the closed-enum membership at
the entry gate (it costs a major bump to change later), then
land three small modules + their dep landings inside
`starter-spi`, then bring up the cargo-tree baseline as the CI
gate every future starter-spi PR diffs against.

## Sequencing

- **Stage 1 is prose-only.** Lock D-U0.1 through D-U0.4 in
  [SCOPE.md](./SCOPE.md), record under "Decisions". Commit; no
  code.
- **Stage 2 is the entry-gate REVIEW.** Do not advance until the
  user signs off on the closed-enum membership. R4 says renaming
  / removing variants costs a major SemVer bump plus a
  deprecation alias for at least one major. The REVIEW exists
  because the rest of the job binds to whatever set ships out
  of this gate.
- **Stages 3 → 5 land the three modules.** Order is `units` →
  `preferences` → `i18n`. Preferences depends on `units::Unit`
  (its per-unit fields use the enum); i18n is independent but
  lands last so it doesn't dilute the focus during the
  enum-stabilisation work.
- **Stage 6 wires lib.rs, adds the Cargo.toml deps, and commits
  the cargo-tree baseline.** This is the CI seam every future
  starter-spi-touching PR diffs against.
- **Stage 7 is workspace-wide verify + the Phase 0 SCOPE smoke.**
  Cargo build / clippy / fmt across the workspace. The Phase 0
  smoke from the user SCOPE Smoke-tests block ("Headless
  appliance keeps working") confirms `starter-spi`'s new
  modules did not pull middleware / routes / migrations into
  the headless build. The starter-flow-spi baseline diff
  confirms the merged Phase-1-of-flow baseline did not move
  under Phase-0-of-user's landing.

## Per-stage discipline

- **Before any code change in a stage:**
  - `git log -20 --oneline` for the surrounding history.
  - Re-read the rule numbers in [SCOPE.md](./SCOPE.md) the
    stage touches. R1 (canonical-only storage), R3 (no Option
    in ResolvedPreferences), R4 (closed enums + uom for
    conversion), R7 (Accept-Units custom header — informational
    only at Phase 0), R8 (per-series metadata wire shape) are
    the load-bearing rules.
  - Re-read the user SCOPE Crate-layout block §"starter-spi"
    sub-block. Doc comments at the head of every new module
    reference the SCOPE section by name so a future reader does
    not re-derive the design from code.
- **Touch only what the stage names.** No drive-by refactors.
  `starter-spi` already has 11 modules; touching any of them
  except via the lib.rs `pub mod` declaration in stage 6 is
  out of scope.
- **Verify before commit:**
  - **Rust per-stage:** `cargo check -p starter-spi`, then
    `cargo test -p starter-spi`, then `cargo clippy --workspace
    --all-targets -- -D warnings`, then `cargo fmt --check`.
  - **Dep-tree per stage:** re-run `cargo tree -p starter-spi
    --edges normal` and visually confirm only `uom`, `icu_locale`,
    and their transitive deps are new since the previous stage.
    A surprise dep is a stage-fail; revert and find what pulled
    it in.
  - **Sibling baseline per stage:** re-run `cargo tree -p
    starter-flow-spi --edges normal` and `diff` against
    `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`. A diff
    means starter-flow-spi accidentally gained a transitive dep
    through starter-spi's new modules; revert.
- **One logical batch per commit.** The closing trio at the end
  of every stage is the heartbeat the UI watches.

## REVIEW gates

One:

- **After stage 1 (Phase 0 entry gate).** Four small
  decisions — D-U0.1 (Quantity variants), D-U0.2 (Unit
  variants), D-U0.3 (currency wire form), D-U0.4 (Diagnostic
  param map). The REVIEW exists because R4 makes the enum
  membership load-bearing; locking them down first is cheap;
  getting it wrong costs a major SemVer bump.

Stage 7 is itself a verification stage — the smoke pass + dep-
tree gates are the merge gate, not a second REVIEW.

Write a one-line summary into `handover.md` at the gate. Do
not proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | SCOPE.md "Decisions" section filled for D-U0.1 through D-U0.4 plus the four stage-1 sub-decisions from the SCOPE Open-questions block; no code changed; commit references each decision by ID. |
| 3 | `crates/starter-spi/src/units/` (or `units.rs`) populated; `Quantity`, `Unit`, `QuantityDef`, `UnitRegistry`, `StaticRegistry`, `normalize_for_storage` all public; `Quantity` and `Unit` are closed (no `#[non_exhaustive]`); `StaticRegistry` returns the canonical SI mapping from R1 for every Quantity; `normalize_for_storage` round-trips the SCOPE Smoke-tests block's "`auto` derivation" examples (72.4 F → 22.444… C, psi → kPa, etc.) within `f64::EPSILON`-scale tolerance; cargo test green; clippy + fmt green. |
| 4 | `crates/starter-spi/src/preferences/` populated; `ResolvedPreferences`, `PreferencesPatch`, and the six display enums all public; serde rename round-trips against the SCOPE column-comment strings byte-for-byte; `ToSchema` derives present on every public type; cargo test green; clippy + fmt green. |
| 5 | `crates/starter-spi/src/i18n/` populated; `LanguageTag` accepts the SCOPE fallback-chain examples (`en`, `en-US`, `en-AU`, `zh-TW`) and rejects malformed input; `MessageKey` validation rejects empty / leading-dot / trailing-dot / double-dot / whitespace; `Diagnostic` JSON round-trip preserves param order via `BTreeMap`; cargo test green; clippy + fmt green. |
| 6 | `crates/starter-spi/src/lib.rs` declares `pub mod i18n; pub mod preferences; pub mod units;` (alphabetical, matching the existing mod block style); `crates/starter-spi/Cargo.toml` adds `uom = { workspace = true, default-features = false, features = ["si"] }` and `icu_locale = { workspace = true }` under `[dependencies]` (workspace deps land in `Cargo.toml` workspace.dependencies table in the same commit); `DOCS/user/scope/starter-spi-deps.baseline.txt` committed and matches `cargo tree -p starter-spi --edges normal` byte-for-byte. |
| 7 | `cargo build --workspace --all-features` green; `cargo clippy --workspace --all-targets -- -D warnings` green; `cargo fmt --check` green; `cargo tree -p starter-spi --edges normal` matches `DOCS/user/scope/starter-spi-deps.baseline.txt` byte-for-byte; `cargo tree -p starter-flow-spi --edges normal` matches `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` byte-for-byte (unchanged); `cargo tree -p starter-spi --edges normal | grep -E '(axum\|tower\|hyper\|chrono\|time )'` returns empty (no middleware deps, no chrono / time); the "Headless appliance keeps working" smoke premise holds — `starter-spi` does not pull middleware-shaped deps into a headless build. |

## Anti-patterns

- **`#[non_exhaustive]` on the v1 enums.** R4 says closed.
  `#[non_exhaustive]` would silently weaken the guarantee and
  force every downstream `match` to add a `_ => …` arm. The
  closed-enum cost is intentional friction.
- **Hand-written conversion factors.** R4: "Conversion factors
  are delegated to `uom` internally." `0.5556 * (f - 32.0)` in
  `normalize_for_storage` is wrong; route through `uom`'s
  typed conversions. A stage that finds itself wanting a
  hand-written constant has misread R4.
- **Adding `axum` / `tower` / `hyper` to `starter-spi`.** The
  SPI crate is the trait-seam crate. Middleware lives in
  `starter-server`. Phase 0 must not bleed HTTP-layer deps
  into `starter-spi`; stage 7's grep-test enforces this.
- **Adding `chrono` or `time`.** SCOPE Library-choices block
  explicitly picks `jiff`. Phase 0 doesn't need datetime
  parsing — `Timestamp(i64)` epoch-ms is enough — and adding
  one would either conflict with the workspace's eventual
  `jiff` adoption or force everyone to deal with three
  datetime crates.
- **Adding `iso_currency`.** D-U0.3 keeps it out of `starter-spi`.
  Validation lives in `starter-prefs` (Phase 1). A stage that
  thinks `starter-spi` needs ISO 4217 validation has misread
  D-U0.3.
- **`Option<…>` on `ResolvedPreferences` fields.** R3: the
  resolver collapses NULLs and `"auto"` before constructing
  this struct. Optional fields belong on `PreferencesPatch`,
  not `ResolvedPreferences`.
- **`HashMap` on `Diagnostic::params`.** D-U0.4: `BTreeMap` for
  deterministic wire output. Matches the existing
  `starter-flow-spi::SlotMap` posture.
- **`Ratio` quantity, `Percent` unit, money in `Unit`.** All
  out per the SCOPE Decisions-made block and D-U0.1 / D-U0.2.
  A stage that wants these has misread the user SCOPE; revisit
  triggers are documented.
- **Touching `starter-flow-spi`.** The merged sibling froze its
  surface in Phase 1. Phase 0 of user does not touch it; stage
  7's baseline diff catches accidental drift.
- **Per-locale defaults inside `starter-spi`.** ICU / locale-
  derived defaults live in the resolver (Phase 1). `starter-spi`
  ships the types; not the table-lookup logic.
- **`utoipa::ToSchema` missing on a public type.** Workspace R7
  says every DTO appears in `openapi.json`. Stage 4 and stage 5
  tests fail if any newly added public struct or enum lacks the
  derive.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's verify list (per the "Done when"
   table above). Every step must pass. On failure: stop, fix,
   re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the
   active session doc, in the same worktree, so the fresh agent
   that opens the next stage has the context it needs.
3. `git` — stage the changes, commit with the message
   `stage N: <one-line title from template.yaml>`, and push to
   the job's branch (`codeless/starter-prefs-spi`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook fails,
fix the cause.
