//! Platform seed catalogs at `catalogs/starter/`. See SCOPE.md
//! Phase 3.
//!
//! Both the English and Spanish starter chrome catalogs are
//! compiled in via [`include_str!`] so a binary that links this
//! crate can serve the starter-owned strings (auth flow labels,
//! error envelope keys, settings page chrome, preferences field
//! labels) with no on-disk catalog directory. Product-owned
//! catalogs supplied at deploy time live alongside these and are
//! loaded via [`crate::catalog::Catalog::from_file`].
//!
//! Both catalogs ship with identical key sets — the
//! `seed_catalog_consistency` integration test asserts this so the
//! Phase 4 React UI's "load every key in both locales" smoke test
//! has a stable baseline.

use starter_spi::i18n::LanguageTag;

use crate::bundle::MessageBundle;
use crate::catalog::{Catalog, CatalogError};

/// Compiled-in English starter chrome catalog. The single source
/// of truth for every starter-owned string the workspace currently
/// emits (auth flow, error envelopes, settings page chrome,
/// preferences field labels).
pub const STARTER_EN_JSON: &str = include_str!("../catalogs/starter/en.json");

/// Compiled-in Spanish translation of [`STARTER_EN_JSON`]. Every
/// key in the English catalog has a Spanish entry — see the
/// `seed_catalog_consistency` integration test.
pub const STARTER_ES_JSON: &str = include_str!("../catalogs/starter/es.json");

/// Parse the embedded English starter chrome catalog.
///
/// # Panics
///
/// Never in practice — the embedded JSON is part of the crate's
/// build artefacts and is validated by the unit + integration
/// tests. The `expect` is there so a malformed seed catalog (which
/// would be a build-time mistake, not a runtime one) surfaces as
/// a clear panic during startup rather than a confusing
/// later-stage miss.
#[must_use]
pub fn starter_en() -> Catalog {
    Catalog::from_json_str(STARTER_EN_JSON).expect("embedded en.json must be valid")
}

/// Parse the embedded Spanish starter chrome catalog. See
/// [`starter_en`] for the panic posture.
#[must_use]
pub fn starter_es() -> Catalog {
    Catalog::from_json_str(STARTER_ES_JSON).expect("embedded es.json must be valid")
}

/// Build a [`MessageBundle`] preloaded with the compiled-in
/// starter chrome catalogs (en + es), with `en` as the R5
/// fallback. Intended for the typical product binary that wants
/// the starter strings out of the box.
///
/// Consumers that need a different fallback or want to add their
/// own catalogs on top can clone the returned bundle and
/// [`MessageBundle::insert`] additional languages.
#[must_use]
pub fn starter_bundle() -> MessageBundle {
    let en_tag = LanguageTag::parse("en").expect("\"en\" is a valid BCP-47 tag");
    let es_tag = LanguageTag::parse("es").expect("\"es\" is a valid BCP-47 tag");
    let mut bundle = MessageBundle::new(en_tag.clone());
    bundle.insert(en_tag, starter_en());
    bundle.insert(es_tag, starter_es());
    bundle
}

/// Build a starter [`MessageBundle`] but surface catalog-parse
/// errors instead of panicking. Mostly useful for tests that want
/// to assert the embedded catalogs parse cleanly.
pub fn try_starter_bundle() -> Result<MessageBundle, CatalogError> {
    let en = Catalog::from_json_str(STARTER_EN_JSON)?;
    let es = Catalog::from_json_str(STARTER_ES_JSON)?;
    let en_tag = LanguageTag::parse("en").expect("\"en\" is a valid BCP-47 tag");
    let es_tag = LanguageTag::parse("es").expect("\"es\" is a valid BCP-47 tag");
    let mut bundle = MessageBundle::new(en_tag.clone());
    bundle.insert(en_tag, en);
    bundle.insert(es_tag, es);
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_spi::i18n::MessageKey;

    fn key(s: &str) -> MessageKey {
        MessageKey::parse(s).expect("test key must parse")
    }

    #[test]
    fn embedded_en_catalog_parses() {
        let cat = starter_en();
        assert!(!cat.is_empty());
    }

    #[test]
    fn embedded_es_catalog_parses() {
        let cat = starter_es();
        assert!(!cat.is_empty());
    }

    #[test]
    fn starter_bundle_resolves_known_key_in_both_languages() {
        let bundle = starter_bundle();
        let en_tag = LanguageTag::parse("en").unwrap();
        let es_tag = LanguageTag::parse("es").unwrap();
        let k = key("starter.auth.login.button.label");
        assert_eq!(bundle.lookup(&en_tag, &k), Some("Sign in"));
        assert_eq!(bundle.lookup(&es_tag, &k), Some("Iniciar sesión"));
    }

    #[test]
    fn try_starter_bundle_is_ok() {
        try_starter_bundle().expect("embedded catalogs parse");
    }
}
