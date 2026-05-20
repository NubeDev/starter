## Done

- Added DOCS/user/scope/PHASES-1-5-VERIFY.md recording every SCOPE smoke-test (Headless appliance, Resolver layer precedence, auto derivation, Australian operator, MCP raw mode, Add a language, Canonical-only logs, Custom quantity) with pass/fail + the exact proving command, R1–R8 structural confirmations (grep / cargo-tree / per-rule one-liner), per-crate test results, workspace-policy R5 confirmation, and caveats C-1..C-4.
- Updated DOCS/user/scope/SCOPE.md to mark Phases 1–5 as DONE with per-phase commit-range pointers and a "Landed in starter-prefs-i18n job" footer referencing PHASES-1-5-VERIFY.md.
- Updated DOCS/user/scope/PHASE0-VERIFY.md with F-0.1 / F-0.2 closure notes: F-0.2 CLOSED via the worktree-path-stripping helper in starter-flow's dep-tree gate; F-0.1 honestly recorded as still OPEN.
- Drive-by clippy fix in crates/starter-i18n/src/locale.rs (6-space doc-list indent → 3 spaces) so the lib + lib-test pass `-D warnings`.
- Committed as `a8ebc53 stage 22 — final cleanup + docs sweep`.

## Next

- (none) — job complete.

## What you need to know

- F-0.1 (starter-flow-spi baseline drift) is **still open**. `crates/starter-spi/Cargo.toml` still lists `uom` + `icu_locale_core` unconditionally; the intended feature-gating per PHASE0-VERIFY.md recommendation (b) never landed despite intermediate stage commits claiming it. `cargo test -p starter-flow --test workspace_dep_tree_gates starter_flow_spi_baseline_holds` fails. Fix requires either (a) re-capturing the flow-spi baseline or (b) actually moving uom + icu_locale_core behind default-off starter-spi features.
- `cargo fmt --all -- --check` produces a 645-line diff across ~64 files in starter-extensions/** and examples/** — pre-existing, unrelated to Phases 1–5, recorded as C-2.
- `cargo test --workspace` requires `--exclude starter-grpc`; starter-grpc's tools_service test references a feature-gated `testing` module — pre-existing (C-3).
- Per-crate suites are all green: starter-prefs (with sqlite + routes), starter-i18n (with routes), starter-server, pnpm -C packages/starter-ui-core test (75/75) + typecheck.

## Open questions

- (none) — see C-1 for the one operator-visible follow-up; resolution path is documented in PHASES-1-5-VERIFY.md §C-1.
