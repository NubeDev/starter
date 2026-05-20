## Done

- Implemented `Catalog` in `crates/starter-i18n/src/catalog.rs`: flat `{ MessageKey: string }` JSON shape; `from_json_str` (compiled-in const &str) and `from_file` (disk) loaders; `MessageKey::parse` is the "deny unknown top-level key" gate; `fingerprint()` returns first 16 hex chars of sha256 over canonical JSON; `CatalogError { Io, Parse }`.
- Implemented `MessageBundle` in `crates/starter-i18n/src/bundle.rs`: `HashMap<LanguageTag, Catalog>` keyed by tag, `new(fallback)`, `insert`, `catalog`, `languages`, `lookup` walking R5 chain (exact → language-family in either direction → static fallback), `render_or_key` logging `i18n.missing_key` debug event and returning the key as the rendered string.
- Added `thiserror` workspace dep to `crates/starter-i18n/Cargo.toml`.
- 40 unit tests pass via `cargo test -p starter-i18n`.
- Committed as `3ef01dd`.

## Next

- Stage 14 per SCOPE rollout (likely `platform.rs` seed catalogs at `catalogs/starter/` en.json + es.json embedded via `include_str!`, then `translate.rs` Translate trait, then routes/middleware).

## What you need to know

- `LanguageTag` is not `Ord`, so the bundle uses `HashMap` (not `BTreeMap`); `languages()` iteration order is unspecified — callers needing determinism (manifest route) must collect+sort by `as_str()`.
- `MessageKey` IS `Ord`, so `Catalog.messages: BTreeMap<MessageKey, String>` gives canonical serialisation for the fingerprint.
- `Catalog` is `#[serde(transparent)]` over the inner map; serialises as the flat JSON object on the wire.
- `render_or_key` uses `tracing::debug!(target: "i18n.missing_key", …)`; integrators wanting alerts on missing keys can filter on that target.

## Open questions

- (none)
