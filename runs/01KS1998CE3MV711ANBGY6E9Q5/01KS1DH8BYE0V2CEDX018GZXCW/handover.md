## Done

- Added crates/starter-i18n/catalogs/starter/{en,es}.json with 87 starter-owned keys covering auth flow (login/signup/forgot/reset + every AuthError variant from starter-auth-users session/token/password/linked-providers/user-store), generic error envelopes (validation.*, internal.*, not-found.*, forbidden/unauthorized/rate-limited/conflict), settings page chrome (heading, tabs, save/cancel/reset, toasts, "Auto" inherit sentinel), and one key per ResolvedPreferences field (15 fields).
- Filled crates/starter-i18n/src/platform.rs: `STARTER_EN_JSON` / `STARTER_ES_JSON` `include_str!` consts; `starter_en()` / `starter_es()` parse helpers; `starter_bundle()` returns a `MessageBundle` preloaded with en+es (en = R5 fallback); `try_starter_bundle()` for error-surfacing variant.
- Added crates/starter-i18n/tests/seed_catalog_consistency.rs: identical-key-set assertion, non-empty trimmed values, no UTF-8 BOM, every ResolvedPreferences field has a label key.
- `cargo test -p starter-i18n` green (44 unit + 6 integration).
- Committed as `stage 15 — Phase 3 seed catalogs …` (51d8809).

## Next

- Stage 16 picks up the next item in the Rollout (likely Phase 4 — @nube/starter-ui-core PreferencesProvider/formatters/IntlProvider/Settings page).

## What you need to know

- MessageKey rules (starter-spi): non-empty, no whitespace, ASCII-graphic only, no leading/trailing dot, no `..`. All seed keys use reverse-DNS-ish `starter.<domain>.<…>.label` form.
- Catalog wire shape is flat `{ MessageKey: string }`; loader uses `deny_unknown_fields` semantics via MessageKey validation; fingerprint is the 16-char sha256 prefix of canonical JSON (BTreeMap-ordered).
- en is the R5 fallback; the bundle returned by `platform::starter_bundle()` is the intended entry point for product binaries that want starter chrome out of the box.
- The Phase 4 UI "load every key in both locales" smoke test should diff against `STARTER_EN_JSON`'s key set; `seed_catalog_consistency::en_and_es_have_identical_key_sets` is the Rust-side gate.

## Open questions

- (none)
