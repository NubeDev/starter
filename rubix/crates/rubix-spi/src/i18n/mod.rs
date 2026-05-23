//! Rubix bundled message catalogues.
//!
//! `en.json` is the canonical source; `es.json` is the initial
//! Spanish translation. Both files live next to this module in
//! [`crates/rubix-spi/catalogues/`](../../catalogues/) and are
//! embedded at compile time via `include_str!`.
//!
//! Adding a `MessageKey` without a matching entry in **both**
//! catalogues fails review. See
//! [docs/design/i18n-prefs/](../../../docs/design/i18n-prefs/README.md).

/// EN catalogue JSON, embedded at compile time.
pub const RUBIX_EN_JSON: &str = include_str!("../../catalogues/en.json");

/// ES catalogue JSON, embedded at compile time.
pub const RUBIX_ES_JSON: &str = include_str!("../../catalogues/es.json");

mod load;

pub use load::rubix_bundle;
