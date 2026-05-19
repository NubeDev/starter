## Done

- Verified `crates/starter-spi/src/lib.rs` already declares `pub mod preferences;`, `pub mod units;`, `pub mod i18n;` in alphabetical order (landed in stages 3–5).
- Verified `crates/starter-spi/Cargo.toml` already lists `uom` and `icu_locale_core` as workspace deps scoped to the new modules.
- Generated `DOCS/user/scope/starter-spi-deps.baseline.txt` via `cargo tree -p starter-spi --edges normal` (115 lines); contains uom + icu_locale_core and their transitive deps; no `time` crate, no new tokio-runtime additions, chrono/tokio entries match pre-existing.
- Verified `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` is unchanged (git diff clean).
- `cargo build -p starter-spi` green.
- Committed as `stage 6 — lib.rs wiring + module visibility + dep-tree baseline`.

## Next

- Stage 7 (final): per the SCOPE workflow, the next session picks it up.

## What you need to know

- The lib.rs wiring and Cargo.toml dep additions were already performed by earlier stages (3–5); this stage only needed the baseline file + verification, which matches the directive's check-and-record posture.
- Workspace emits unrelated `default-features` warnings on starter-flow-* and starter-flow-surfaces — pre-existing, not from this stage.
- Features on the new deps: `uom` and `icu_locale_core` are inherited from `[workspace.dependencies]`; check `Cargo.toml` at workspace root if feature trimming is needed in a later stage.

## Open questions

- (none)
