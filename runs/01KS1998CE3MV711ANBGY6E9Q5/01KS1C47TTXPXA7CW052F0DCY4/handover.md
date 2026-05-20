## Done

- Added `crates/starter-i18n/` with `default-features = []`, depending on starter-spi, serde, serde_json, utoipa, tracing, icu_locale_core, sha2. `routes` feature (default-off) reserves axum/http/tower; `diagnostics` feature reserved for Phase 5.
- `lib.rs` declares modules: `locale`, `catalog`, `bundle`, `translate`, `platform`, plus `routes` and `middleware` behind `cfg(feature = "routes")`. All non-locale modules are empty placeholders with SCOPE-ownership comments.
- `locale.rs` implements `parse_accept_language` and `pick_language` per R5 fallback chain (exact → language family → wildcard → static fallback). 21 unit tests pass; `cargo check -p starter-i18n --features routes` also green.
- Workspace Cargo.toml updated: member entry + workspace dep alias.
- Committed: `stage 12 — Phase 3 starter-i18n crate scaffold + LanguageTag-based locale module.`

## Next

- Stage 13 (next session): fill catalog loader (`deny_unknown_fields` ICU JSON, sha256-hex-16 fingerprint), `TranslateBundle`, `Translate` trait, platform seed catalogs (`catalogs/starter/en.json`, `es.json`), and the `routes` feature's REST surface + Accept-Language tower middleware.

## What you need to know

- The wildcard `*` is intentionally NOT represented in the `Vec<(LanguageTag, f32)>` returned by `parse_accept_language`, because `LanguageTag` (from starter-spi) validates via `icu_locale_core` which rejects `*`. Instead `pick_language` scans the raw header for `*` after the explicit walk. There's a test (`wildcard_is_dropped_from_parsed_list`) pinning this choice; if a future stage needs the wildcard in the parsed list, the cleanest fix is to extend `LanguageTag` with a wildcard constructor on starter-spi.
- The stage prompt claimed `icu_locale_core` is already feature-gated on starter-spi, but inspecting `crates/starter-spi/Cargo.toml` shows it is still an unconditional dep. Stage 1's decision-lock recorded the intent but did not perform the gating. I added `icu_locale_core` directly to starter-i18n's deps (same workspace entry), so the locale module works regardless; if a later stage actually feature-gates starter-spi's icu, no change here is needed.

## Open questions

- (none)
