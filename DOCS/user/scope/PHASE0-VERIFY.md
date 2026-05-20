# Phase 0 — stage 7 verification

Stage 7 of the Phase 0 SCOPE (starter-spi wire surface for preferences,
units, i18n) ran the workspace gates and the headless-appliance smoke
described in DOCS/user/scope/SCOPE.md.

## Workspace gates

- `cargo build --workspace --all-features` — green.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — green.
- `cargo fmt --all -- --check` — green (one `cargo fmt --all` was run
  during this stage to absorb a stray multi-line `contains` chain from
  stage 6's units tests; no behavioural diff).

## starter-spi dep posture

`cargo tree -p starter-spi --edges normal` contains neither `axum` nor
`tower` nor `hyper` nor `sqlx`. The new third-party deps `uom` and
`icu_locale_core` land exactly where R4 / the SCOPE Library-choices
table say they should and bring no runtime, HTTP, or DB transitives.

The starter-spi baseline at
`DOCS/user/scope/starter-spi-deps.baseline.txt` is byte-for-byte
identical to a fresh `cargo tree -p starter-spi --edges normal` from
this worktree.

## Headless appliance smoke

A binary linking only `starter-auth-token` + `starter-secrets-file`
(no `starter-prefs`, no `starter-i18n` — neither crate exists yet)
still builds. `starter-spi`'s new modules are gated behind their own
`pub mod` declarations and pull no middleware, routes, or migrations
into the build graph. Confirmed via `cargo build -p starter-auth-token
-p starter-secrets-file` — green.

## Phase 0 module isolation

`rg "starter_spi::(preferences|units|i18n)" --type rust` across the
workspace returns matches only inside `crates/starter-spi/` itself
(unit tests). No other crate imports the new modules. Phase 1 / Phase
3 (`starter-prefs`, `starter-i18n`) is the next consumer; those crates
do not exist yet.

## starter-flow-spi baseline — REGRESSION FLAG

`DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` (landed by the
merged starter-flow-scaffold job) is **NOT** byte-for-byte identical
to the current `cargo tree -p starter-flow-spi --edges normal`. The
diff is two-fold:

1. **Worktree path lines** — the baseline was captured in an earlier
   worktree (`job-01KRZXKWVD5QP0D9FK2SMRK1RG`); current run is
   `job-01KS07FCDQXWZ8QT8NSHMVJ7S3`. Two lines (the two
   `(/home/user/.codeless/worktrees/...)` lines for starter-flow-spi
   and starter-spi) differ only in path. This is cosmetic and not a
   real dep change; `cargo tree` should be re-captured with a
   path-stripping post-process for the baseline to be portable across
   worktrees.

2. **Real transitive deps** — `starter-flow-spi` depends on
   `starter-spi` (Cargo.toml line: `starter-spi = { workspace = true
   }`). Stage 6 added `uom` and `icu_locale_core` to
   `starter-spi`'s `[dependencies]` (no optional feature gating), so
   those crates and their transitives (`displaydoc`, `litemap`,
   `tinystr`, `writeable`, `num-traits`, `typenum`) now show up under
   the `starter-spi` subtree of `starter-flow-spi`'s `--edges normal`
   tree. This is a real, intentional change to the flow-spi
   transitive graph.

The stage-6 commit message claimed the flow-spi baseline was
unchanged. That claim is incorrect — adding any
non-optional dep to `starter-spi` necessarily lights up the
`starter-flow-spi` tree because flow-spi depends on starter-spi. The
two reconciliations available to the next session are:

- (a) **Accept the drift** — re-capture
  `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` from a clean
  tree, document `uom`/`icu_locale_core` as expected Phase-0 drift
  in the flow SCOPE notes, and add a worktree-path-stripping step to
  the baseline-capture procedure so future diffs are stable.
- (b) **Restore byte-for-byte** — move `uom` and `icu_locale_core`
  behind starter-spi cargo features (`units` and `i18n`) that
  default to **off**, so a consumer that doesn't opt in (such as
  `starter-flow-spi`) doesn't pull them. Phase 1's `starter-prefs`
  and Phase 3's `starter-i18n` would then enable those features
  explicitly.

Recommendation: (b). It preserves the "headless appliance pulls
nothing it didn't ask for" posture more strongly than (a) and keeps
the flow-spi baseline genuinely stable for the next downstream
consumer that lands.

Stage 7 itself does NOT execute either fix — the directive says the
new baseline is byte-for-byte-stable, and forcing either fix at this
late stage exceeds Phase 0's "wire surface only, no behaviour" scope.
The next session picks the call.

## F-0.1 / F-0.2 closure (Phases 1–5 rollout, stage 22)

The `starter-prefs-i18n` job (Phases 1–5) revisited both follow-ups.
Resolution recorded honestly:

- **F-0.1 — `starter-flow-spi` baseline drift.** Still **OPEN** at
  HEAD `febb6c3` + stage-22 docs sweep. The intended resolution
  (recommendation (b): feature-gate `uom` + `icu_locale_core` on
  `starter-spi` behind default-off `units` / `i18n` features) was
  *not* executed despite intermediate stage commit messages claiming
  it. `crates/starter-spi/Cargo.toml` still lists both deps
  unconditionally, and
  `cargo test -p starter-flow --test workspace_dep_tree_gates
  starter_flow_spi_baseline_holds` fails with the same uom + icu
  drift. Full details + the two reconciliation paths land in
  [PHASES-1-5-VERIFY.md](./PHASES-1-5-VERIFY.md) §C-1.
- **F-0.2 — Worktree-path-sensitive baseline capture.** **CLOSED.**
  `crates/starter-flow/tests/workspace_dep_tree_gates.rs` normalises
  any `/worktrees/job-XXX/` absolute path to a stable `<WORKTREE>/`
  prefix before diffing the baseline, so the test is reproducible
  across worktrees and CI nodes. The capture helper
  [`capture-baseline.sh`](./capture-baseline.sh) applies the same
  post-process when re-baselining. The two cosmetic worktree-path
  diff lines that originally surfaced in this report no longer
  produce a test failure on their own.
