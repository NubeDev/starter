//! `MessageBundle` — an in-memory snapshot of loaded catalogs,
//! keyed by [`LanguageTag`].
//!
//! See SCOPE.md Phase 3. Built once at startup and shared via
//! `Arc<MessageBundle>`; lookups are infallible read-paths along
//! the R5 fallback chain.
//!
//! # R5 fallback chain
//!
//! For a `(lang, key)` lookup the bundle walks:
//!
//! 1. **Exact match** — the requested [`LanguageTag`].
//! 2. **Language family** — any registered tag whose BCP-47
//!    `language` subtag matches (e.g. `"en-US"` requested falls
//!    through to a registered `"en"` or `"en-GB"`).
//! 3. **Static fallback** — the tag passed to
//!    [`MessageBundle::new`] (typically `"en"`).
//!
//! Each step returns `Some(&str)` the moment the message key is
//! present in the candidate catalog. Missing keys fall all the way
//! through to `None` so the caller can decide what to render — see
//! [`MessageBundle::render_or_key`] for the documented default
//! ("log a debug event and return the key itself").

use std::collections::{BTreeMap, HashMap};

use icu_locale_core::LanguageIdentifier;
#[cfg(feature = "preferences")]
use starter_spi::i18n::Diagnostic;
use starter_spi::i18n::{DiagnosticParam, LanguageTag, MessageKey};
#[cfg(feature = "preferences")]
use starter_spi::preferences::ResolvedPreferences;

use crate::catalog::Catalog;
#[cfg(feature = "preferences")]
use crate::interpolate::interpolate_typed_with_prefs;
use crate::interpolate::interpolate_typed;

/// In-memory snapshot of every language catalog the binary knows
/// about.
///
/// Cheap to clone via `Arc`; mutate-then-swap is the intended
/// reload story (build a fresh `MessageBundle`, swap the `Arc`).
#[derive(Debug, Clone)]
pub struct MessageBundle {
    catalogs: HashMap<LanguageTag, Catalog>,
    fallback: LanguageTag,
}

impl MessageBundle {
    /// Construct an empty bundle with `fallback` as the final
    /// step in the R5 chain. The fallback tag does NOT need to be
    /// pre-registered — if there is no catalog for it,
    /// [`MessageBundle::lookup`] returns `None` for any miss, which
    /// [`MessageBundle::render_or_key`] then renders as the key
    /// itself.
    #[must_use]
    pub fn new(fallback: LanguageTag) -> Self {
        Self {
            catalogs: HashMap::new(),
            fallback,
        }
    }

    /// Register (or replace) the catalog for `lang`.
    pub fn insert(&mut self, lang: LanguageTag, catalog: Catalog) {
        self.catalogs.insert(lang, catalog);
    }

    /// Merge `catalog` into the catalog already registered for
    /// `lang`. Existing keys are overwritten by incoming values
    /// (last-writer-wins); keys that are not yet registered are
    /// added.
    ///
    /// Use this to layer application-specific messages on top of
    /// a pre-built platform bundle (e.g. compose `flow_agent.*`
    /// keys on top of [`crate::platform::starter_bundle`] without
    /// losing the `starter.*` chrome).
    pub fn extend(&mut self, lang: LanguageTag, catalog: Catalog) {
        let entry = self.catalogs.entry(lang).or_default();
        for (key, value) in catalog.messages {
            entry.messages.insert(key, value);
        }
    }

    /// The registered tag the bundle falls back to when neither
    /// the requested tag nor its language family is present.
    #[must_use]
    pub fn fallback(&self) -> &LanguageTag {
        &self.fallback
    }

    /// Languages this bundle knows about. Iteration order is
    /// unspecified — callers that need a deterministic shape
    /// (e.g. the `GET /v1/i18n/manifest` route) should collect
    /// and sort by [`LanguageTag::as_str`].
    pub fn languages(&self) -> impl Iterator<Item = &LanguageTag> {
        self.catalogs.keys()
    }

    /// The catalog registered for `lang`, if any. Useful for
    /// `GET /v1/i18n/catalogs/{lang}` — serve the catalog the
    /// caller explicitly asked for, not the one the fallback walk
    /// would pick.
    #[must_use]
    pub fn catalog(&self, lang: &LanguageTag) -> Option<&Catalog> {
        self.catalogs.get(lang)
    }

    /// Walk the R5 fallback chain and return the first message
    /// hit, or `None` if the key is missing everywhere.
    #[must_use]
    pub fn lookup(&self, lang: &LanguageTag, key: &MessageKey) -> Option<&str> {
        // 1. Exact match.
        if let Some(cat) = self.catalogs.get(lang) {
            if let Some(v) = cat.get(key) {
                return Some(v);
            }
        }

        // 2. Language-family match — any registered tag whose
        //    BCP-47 `language` subtag matches the requested tag's.
        if let Ok(req_id) = LanguageIdentifier::try_from_str(lang.as_str()) {
            let req_family = req_id.language;
            for (cand_tag, cand_cat) in &self.catalogs {
                if cand_tag == lang {
                    // Already tried above; skip to avoid double work.
                    continue;
                }
                let Ok(cand_id) = LanguageIdentifier::try_from_str(cand_tag.as_str()) else {
                    continue;
                };
                if cand_id.language == req_family {
                    if let Some(v) = cand_cat.get(key) {
                        return Some(v);
                    }
                }
            }
        }

        // 3. Static fallback.
        if lang != &self.fallback {
            if let Some(cat) = self.catalogs.get(&self.fallback) {
                if let Some(v) = cat.get(key) {
                    return Some(v);
                }
            }
        }

        None
    }

    /// Lookup with the SCOPE-documented "missing key" default: if
    /// the key is missing everywhere, log a `debug` event tagged
    /// `i18n.missing_key` and return the key itself as the rendered
    /// string.
    ///
    /// This is the default helper handlers reach for when they want
    /// best-effort rendering without conditional logic. Callers that
    /// want to make the missing-key path visible to users (e.g. a
    /// dev-mode "[missing: foo.bar]" wrapper) should call
    /// [`MessageBundle::lookup`] directly.
    #[must_use]
    pub fn render_or_key(&self, lang: &LanguageTag, key: &MessageKey) -> String {
        match self.lookup(lang, key) {
            Some(v) => v.to_string(),
            None => {
                tracing::debug!(
                    target: "i18n.missing_key",
                    lang = %lang,
                    key = %key,
                    "missing translation; rendering key as fallback",
                );
                key.as_str().to_string()
            }
        }
    }

    /// Lookup + interpolate the named `{placeholder}` substitutions
    /// in the template with the supplied typed `params`. The R5
    /// fallback chain applies; a missing key renders the key itself
    /// (same posture as [`render_or_key`]).
    ///
    /// This is the param-aware counterpart to [`render_or_key`].
    /// Use it from transports that resolve outside the HTTP
    /// [`crate::diagnostics_layer`] — typically the MCP server-side
    /// render path, the CLI, and any tool returning a
    /// pre-formatted summary string. The interpolation supports
    /// named substitution only (no plural / select); richer
    /// rendering belongs in the client.
    ///
    /// Unknown placeholders are left literal so a template
    /// referencing a missing param surfaces as text rather than
    /// silently dropping.
    #[must_use]
    pub fn render(
        &self,
        lang: &LanguageTag,
        key: &MessageKey,
        params: &BTreeMap<String, DiagnosticParam>,
    ) -> String {
        let template = match self.lookup(lang, key) {
            Some(v) => v,
            None => {
                tracing::debug!(
                    target: "i18n.missing_key",
                    lang = %lang,
                    key = %key,
                    "missing translation; rendering key as fallback",
                );
                return key.as_str().to_string();
            }
        };
        interpolate_typed(template, params)
    }

    /// Render a [`Diagnostic`] for the supplied caller, converting
    /// any `Quantity`-typed params into the caller's preferred unit
    /// system before substituting. This is the one-call helper for
    /// transports that resolve server-side — MCP (the documented R5
    /// exception), the CLI, and any other consumer holding a
    /// [`ResolvedPreferences`].
    ///
    /// Gated on the `preferences` feature so the i18n core stays
    /// usable for consumers that don't ship the preferences crate.
    #[cfg(feature = "preferences")]
    #[must_use]
    pub fn render_diagnostic(
        &self,
        lang: &LanguageTag,
        diag: &Diagnostic,
        prefs: &ResolvedPreferences,
    ) -> String {
        let template = match self.lookup(lang, &diag.code) {
            Some(v) => v,
            None => {
                tracing::debug!(
                    target: "i18n.missing_key",
                    lang = %lang,
                    key = %diag.code,
                    "missing translation; rendering key as fallback",
                );
                return diag.code.as_str().to_string();
            }
        };
        interpolate_typed_with_prefs(template, &diag.params, prefs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(s: &str) -> LanguageTag {
        LanguageTag::parse(s).expect("test tag must parse")
    }

    fn key(s: &str) -> MessageKey {
        MessageKey::parse(s).expect("test key must parse")
    }

    fn cat_with(pairs: &[(&str, &str)]) -> Catalog {
        let mut map = std::collections::BTreeMap::new();
        for (k, v) in pairs {
            map.insert(key(k), (*v).to_string());
        }
        Catalog { messages: map }
    }

    #[test]
    fn lookup_exact_match_wins() {
        let mut b = MessageBundle::new(tag("en"));
        b.insert(tag("en"), cat_with(&[("a.b", "english")]));
        b.insert(tag("es"), cat_with(&[("a.b", "spanish")]));

        assert_eq!(b.lookup(&tag("es"), &key("a.b")), Some("spanish"));
        assert_eq!(b.lookup(&tag("en"), &key("a.b")), Some("english"));
    }

    #[test]
    fn lookup_falls_through_to_language_family() {
        // Request en-US; only `en` is registered — family fallback hits.
        let mut b = MessageBundle::new(tag("en"));
        b.insert(tag("en"), cat_with(&[("a.b", "english")]));

        assert_eq!(b.lookup(&tag("en-US"), &key("a.b")), Some("english"));
    }

    #[test]
    fn lookup_family_match_other_direction() {
        // Request `en`; only `en-GB` is registered — family fallback
        // hits the other way.
        let mut b = MessageBundle::new(tag("en"));
        b.insert(tag("en-GB"), cat_with(&[("a.b", "british")]));

        assert_eq!(b.lookup(&tag("en"), &key("a.b")), Some("british"));
    }

    #[test]
    fn lookup_falls_through_to_static_fallback() {
        // Request `fr`; nothing French registered; fallback `en` carries it.
        let mut b = MessageBundle::new(tag("en"));
        b.insert(tag("en"), cat_with(&[("a.b", "english")]));

        assert_eq!(b.lookup(&tag("fr"), &key("a.b")), Some("english"));
    }

    #[test]
    fn lookup_missing_key_returns_none() {
        // Per R5 the bundle returns None for a true miss; the caller
        // (typically render_or_key) decides what to render.
        let mut b = MessageBundle::new(tag("en"));
        b.insert(tag("en"), cat_with(&[("a.b", "english")]));

        assert_eq!(b.lookup(&tag("en"), &key("not.present")), None);
        assert_eq!(b.lookup(&tag("fr"), &key("not.present")), None);
    }

    #[test]
    fn lookup_prefers_exact_over_family() {
        // `en` registered with one message, `en-GB` with a different
        // value for the same key — exact wins when requested.
        let mut b = MessageBundle::new(tag("en"));
        b.insert(tag("en"), cat_with(&[("a.b", "english")]));
        b.insert(tag("en-GB"), cat_with(&[("a.b", "british")]));

        assert_eq!(b.lookup(&tag("en-GB"), &key("a.b")), Some("british"));
        assert_eq!(b.lookup(&tag("en"), &key("a.b")), Some("english"));
    }

    #[test]
    fn render_or_key_returns_key_for_missing() {
        let b = MessageBundle::new(tag("en"));
        // Empty bundle — nothing to find.
        assert_eq!(b.render_or_key(&tag("en"), &key("a.b")), "a.b");
    }

    #[test]
    fn render_or_key_returns_translation_when_present() {
        let mut b = MessageBundle::new(tag("en"));
        b.insert(tag("en"), cat_with(&[("a.b", "english")]));
        assert_eq!(b.render_or_key(&tag("en"), &key("a.b")), "english");
    }

    #[test]
    fn empty_bundle_lookup_is_none_not_panic() {
        let b = MessageBundle::new(tag("en"));
        assert_eq!(b.lookup(&tag("fr"), &key("a.b")), None);
    }

    #[test]
    fn fallback_is_skipped_when_requested_lang_equals_fallback() {
        // A subtle correctness check: when the requested lang IS
        // the fallback, we must not double-search the same catalog.
        // Easiest observable: a key present only in a non-fallback
        // catalog must NOT be returned for an en request.
        let mut b = MessageBundle::new(tag("en"));
        b.insert(tag("es"), cat_with(&[("a.b", "spanish-only")]));
        assert_eq!(b.lookup(&tag("en"), &key("a.b")), None);
    }

    #[test]
    fn extend_layers_keys_without_dropping_existing() {
        // The intended consumer pattern: start from a pre-built
        // platform bundle (e.g. `starter_bundle()`) and layer
        // application-specific keys on top.
        let mut b = MessageBundle::new(tag("en"));
        b.insert(tag("en"), cat_with(&[("starter.a", "platform")]));
        b.extend(tag("en"), cat_with(&[("app.b", "application")]));

        assert_eq!(b.lookup(&tag("en"), &key("starter.a")), Some("platform"));
        assert_eq!(b.lookup(&tag("en"), &key("app.b")), Some("application"));
    }

    #[test]
    fn extend_overwrites_overlapping_keys() {
        // Last-writer-wins: extending replaces values for keys that
        // already exist. Lets an application override a platform string.
        let mut b = MessageBundle::new(tag("en"));
        b.insert(tag("en"), cat_with(&[("starter.a", "platform")]));
        b.extend(tag("en"), cat_with(&[("starter.a", "overridden")]));

        assert_eq!(b.lookup(&tag("en"), &key("starter.a")), Some("overridden"));
    }

    #[test]
    fn extend_registers_new_language_when_absent() {
        // No catalog yet for `es` — `extend` should still install
        // the keys, behaving like `insert` for the new tag.
        let mut b = MessageBundle::new(tag("en"));
        b.extend(tag("es"), cat_with(&[("app.b", "aplicacion")]));

        assert_eq!(b.lookup(&tag("es"), &key("app.b")), Some("aplicacion"));
    }
}
