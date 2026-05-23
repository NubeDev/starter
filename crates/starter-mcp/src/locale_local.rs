//! Tool-dispatch task-local for the caller's preferred [`LanguageTag`].
//!
//! Mirrors [`crate::principal_local`]: the MCP `Tool` trait surface is
//! `invoke(input)` only, so threading the caller's locale through a
//! task-local keeps the trait unchanged while letting tools render
//! diagnostics in the caller's language.
//!
//! The HTTP transport binds the locale per request from
//! `Accept-Language`; the stdio loop binds the locale per session from
//! the `initialize` handshake's `params._meta.acceptLanguage` field.
//! See `docs/design/i18n-prefs/README.md` for the negotiation contract
//! and `docs/design/starter-changes/README.md` (Phase 2b U1) for the
//! upstream gap this closes.

use std::future::Future;

use starter_spi::i18n::LanguageTag;

tokio::task_local! {
    static LOCALE: LanguageTag;
}

/// Run `fut` with `locale` bound on the dispatch task. Both transports
/// wrap every `tools/call` dispatch with this; tool implementations
/// read the binding via [`current_locale`].
pub async fn with_locale<F, T>(locale: LanguageTag, fut: F) -> T
where
    F: Future<Output = T>,
{
    LOCALE.scope(locale, fut).await
}

/// Return the [`LanguageTag`] bound on the current task, if any.
///
/// Returns `None` when the caller did not supply an `Accept-Language`
/// (HTTP) or `params._meta.acceptLanguage` (stdio `initialize`), or
/// when the supplied value did not parse to a BCP-47 tag, or when the
/// task was not entered via [`with_locale`].
pub fn current_locale() -> Option<LanguageTag> {
    LOCALE.try_with(|l| l.clone()).ok()
}

/// Pick the top-quality [`LanguageTag`] from an `Accept-Language`
/// header value. Reuses [`starter_i18n::parse_accept_language`] so the
/// quality-ordering and BCP-47 validation rules match the rest of the
/// stack (`docs/design/i18n-prefs/README.md`). Returns `None` when the
/// header is absent, empty, or contains only entries that fail BCP-47
/// validation — the caller binds nothing and tools see `None` from
/// [`current_locale`].
pub fn locale_from_accept_language(header: &str) -> Option<LanguageTag> {
    starter_i18n::locale::parse_accept_language(header)
        .into_iter()
        .next()
        .map(|(tag, _q)| tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn current_locale_is_none_outside_scope() {
        assert!(current_locale().is_none());
    }

    #[tokio::test]
    async fn current_locale_returns_bound_tag() {
        let tag = LanguageTag::parse("es-AR").unwrap();
        let observed = with_locale(tag.clone(), async { current_locale() }).await;
        assert_eq!(observed, Some(tag));
    }

    #[test]
    fn locale_from_header_picks_highest_quality() {
        // `parse_accept_language` returns a stable q-sorted list; the
        // first element is the caller's top preference.
        let picked = locale_from_accept_language("en;q=0.5, es-AR;q=0.9, fr");
        assert_eq!(picked.as_ref().map(|t| t.as_str()), Some("fr"));
    }

    #[test]
    fn locale_from_header_handles_simple_tag() {
        let picked = locale_from_accept_language("es-AR");
        assert_eq!(picked.as_ref().map(|t| t.as_str()), Some("es-AR"));
    }

    #[test]
    fn locale_from_header_returns_none_on_empty() {
        assert!(locale_from_accept_language("").is_none());
        assert!(locale_from_accept_language("   ").is_none());
    }

    #[test]
    fn locale_from_header_drops_invalid_tags() {
        // `en_US` is not BCP-47 (underscore separator) — drop silently.
        let picked = locale_from_accept_language("en_US, !!!!");
        assert!(picked.is_none());
    }
}
