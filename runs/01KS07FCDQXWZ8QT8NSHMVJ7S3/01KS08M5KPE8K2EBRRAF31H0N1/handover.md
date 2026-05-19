## Done

- ran `cargo build --workspace --all-features` — green
- ran `cargo clippy --workspace --all-targets --all-features -- -D warnings` — green
- ran `cargo fmt --all` (absorbed stage-6 fmt drift in 4 files: starter-grpc/build.rs, starter-spi units/i18n/preferences tests) and confirmed `cargo fmt --all -- --check` green
- verified `cargo tree -p starter-spi --edges normal` contains no axum / tower / hyper / sqlx
- verified the Phase-0 smoke binary (`cargo build -p starter-auth-token -p starter-secrets-file`) still builds with no starter-prefs / starter-i18n dependency
- verified no crate outside starter-spi imports `starter_spi::{preferences,units,i18n}` (only starter-spi's own unit tests do)
- confirmed `DOCS/user/scope/starter-spi-deps.baseline.txt` is byte-for-byte identical to a fresh `cargo tree -p starter-spi --edges normal` in this worktree
- wrote `DOCS/user/scope/PHASE0-VERIFY.md` capturing the stage-7 verification results and the flow-spi baseline regression analysis
- committed everything as `stage 7 — workspace verify + Phase 0 SCOPE smoke`

## Next

- (none) — Phase 0 stage 7 is the last stage. Phase 1 (starter-prefs crate) is a fresh job.

## What you need to know

- starter-spi gained two non-optional deps in stage 6: `uom` and `icu_locale_core`. They land cleanly (no HTTP / DB / runtime transitives).
- starter-flow-spi has `starter-spi = { workspace = true }` in `[dependencies]`, so any non-optional starter-spi dep necessarily lights up the flow-spi `--edges normal` tree. This contradicts stage 6's claim that the flow-spi baseline was unchanged.
- The stage-7 directive said the flow-spi baseline diff "must be byte-for-byte" — it isn't. I deliberately did NOT modify `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` (the diff is the truthful signal); the full diff + two reconciliation options (re-capture vs. feature-gate) are documented in `DOCS/user/scope/PHASE0-VERIFY.md`. Recommendation in that doc: feature-gate uom + icu_locale_core behind off-by-default starter-spi cargo features.
- The flow-spi baseline also has a worktree-path mismatch (it was captured in a different worktree); a portable baseline-capture procedure should strip the local path lines.
- One unrelated formatting drift was absorbed in `crates/starter-grpc/build.rs` — pre-existing, not Phase-0 work; the commit message calls it out.

## Open questions

- Should `uom` and `icu_locale_core` move behind off-by-default `units` / `i18n` features on starter-spi so the flow-spi baseline genuinely stays stable, or should the flow-spi baseline be re-captured and the drift documented as expected? Recommendation: feature-gate.
- Should the baseline-capture script be updated to strip absolute worktree paths so baselines compare cleanly across worktrees / CI runners?
