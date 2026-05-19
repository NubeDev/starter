## Done

- Added `crates/starter-spi/src/i18n/` module with `LanguageTag`, `MessageKey`, `DiagnosticParam`, `Diagnostic`, and `I18nError`, wired into `lib.rs`.
- Added `icu_locale_core = "2"` (default-features=false, features=["alloc"]) to workspace Cargo.toml and starter-spi/Cargo.toml per SCOPE Library-choices table.
- 19 new unit tests cover directive matrix; `cargo test -p starter-spi --lib` 46/46 green; `cargo clippy --all-targets -- -D warnings` green.
- Committed as "stage 5 — starter-spi::i18n module".

## Next

- Stage 6 (next session): per the SCOPE this is the cargo-tree diff / integration check before Phase 1 begins.

## What you need to know

- `LanguageTag` uses `LanguageIdentifier::try_from_str` which lives behind icu_locale_core's `alloc` feature — that's why the workspace dep enables it.
- `Diagnostic.params` is `BTreeMap` (deterministic JSON ordering) and is skipped on serialise when empty.
- `DiagnosticParam` is externally-tagged snake_case: `{"string":"x"}`, `{"i64":-7}`, `{"f64":0.5}`, `{"bool":true}`, `{"timestamp":<i64 epoch ms>}`.
- Newtypes use `#[serde(try_from = "String", into = "String")]` + `#[schema(value_type = String)]` so OpenAPI sees a plain string while validation runs on deserialise.

## Open questions

- (none)
