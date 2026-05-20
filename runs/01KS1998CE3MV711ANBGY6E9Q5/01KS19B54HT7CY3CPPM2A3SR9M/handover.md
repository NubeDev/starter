## Done

- Added "Phase 1-5 decisions (locked)" block to DOCS/user/scope/SCOPE.md covering D-0.1..D-0.3, D-1.1..D-1.3, D-2.1/D-2.2, D-3.1/D-3.2, D-4.1/D-4.2, D-5.1 — each with deriving rule and revisit trigger.
- Created executable DOCS/user/scope/capture-baseline.sh that runs `cargo tree -p <crate> --edges normal` and sed-strips the worktree prefix on workspace-member lines so output is portable. Verified against `starter-spi` — top line becomes `starter-spi v0.1.0 (/crates/starter-spi)`.
- Updated DOCS/user/scope/TODO.md "Open follow-ups from Phase 0" preamble to point F-0.1 / F-0.2 at the locked decisions and at stage 2 for implementation.
- Committed as `2628f17` on branch `codeless/starter-prefs-i18n`.

## Next

- Stage 2 (per the locked decisions): feature-gate `uom` + `icu_locale_core` in crates/starter-spi/Cargo.toml behind default-off `units` / `i18n` features; cfg-gate `pub mod units` / `pub mod i18n` in crates/starter-spi/src/lib.rs accordingly; re-capture DOCS/user/scope/starter-spi-deps.baseline.txt via `DOCS/user/scope/capture-baseline.sh starter-spi`; re-capture DOCS/flow/scope/starter-flow-spi-deps.baseline.txt via the same script (closes F-0.2 for both files). Verify `cargo build --workspace --all-features` and the headless-appliance smoke still green.

## What you need to know

- Stage directive forbade code changes outside DOCS/ + the capture script, so Cargo.toml / .rs edits and the baseline re-captures explicitly defer to stage 2. SCOPE.md decisions block calls this out under D-0.1 / D-0.2.
- `cargo tree` embeds the worktree path on workspace-member lines only; non-workspace crates print without a path. The sed rule keeps the `(/crates/...)` or `(/examples/...)` tail and is `g`-applied because a single dep line can mention multiple workspace crates via `path = ...` re-exports (rare but possible).
- D-5.1 locks exactly TWO top-level paths for the diagnostics rewriter: `diagnostic` (object) and `diagnostics` (array of the same envelope). Anything beyond those two paths is out of scope and requires a SCOPE update.
- D-3.2 fingerprint algorithm is sha256-hex truncated to 16 chars over canonicalised JSON (sorted keys, no trailing newline) — the canonicalisation is load-bearing so server + any pre-publish tool produce the same hash.

## Open questions

- (none)
