//! Internationalisation — the wire surface for translatable
//! diagnostics and language tags.
//!
//! Phase 0 of `DOCS/user/scope/SCOPE.md` carves the i18n boundary into
//! `starter-spi` so the Phase 3 `starter-i18n` crate has a stable wire
//! shape to bind to:
//!
//! - [`LanguageTag`] — a BCP-47 string validated on construction via
//!   `icu_locale_core` (the SCOPE Library-choices table authorises this
//!   dep on `starter-spi`). Accepts `"en"`, `"en-US"`, `"zh-TW"`; rejects
//!   underscore-separated forms like `"en_US"` and arbitrary free text.
//! - [`MessageKey`] — a reverse-DNS-style identifier like
//!   `"flow.error"` or `"auth.token.expired"`. Same validation friction
//!   as `starter-flow-spi::KindId`: empty / whitespace / non-printable /
//!   leading-or-trailing dot / double-dot are refused at the
//!   constructor.
//! - [`DiagnosticParam`] — the typed values a translation interpolates.
//!   Carries `String`, `i64`, `f64`, `bool`, and `Timestamp` (UTC epoch
//!   ms per R1) variants.
//! - [`Diagnostic`] — `{ code: MessageKey, params: BTreeMap<…> }`. The
//!   map is `BTreeMap` (not `HashMap`) so the JSON wire form is
//!   deterministic — same posture `starter-flow-spi` takes on `SlotMap`.
//!
//! Module layout follows the workspace "one responsibility per file"
//! rule: each public type lives in its own file and is re-exported
//! here.

mod diagnostic;
mod error;
mod language_tag;
mod message_key;

pub use diagnostic::{Diagnostic, DiagnosticParam};
pub use error::I18nError;
pub use language_tag::LanguageTag;
pub use message_key::MessageKey;

#[cfg(test)]
mod tests;
