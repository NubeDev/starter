//! Shared helper: extract the session locale from an `initialize`
//! frame's `params._meta.acceptLanguage` (MCP `_meta` convention).
//! Used by both the Content-Length stdio loop and the ndjson stdio loop.

use starter_spi::i18n::LanguageTag;

pub(super) fn locale_from_initialize_frame(raw: &str) -> Option<LanguageTag> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    if value.get("method")?.as_str()? != "initialize" {
        return None;
    }
    let header = value
        .get("params")?
        .get("_meta")?
        .get("acceptLanguage")?
        .as_str()?;
    crate::locale_local::locale_from_accept_language(header)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_meta_accept_language() {
        let frame = r#"{"jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"_meta":{"acceptLanguage":"es-AR"}}}"#;
        assert_eq!(locale_from_initialize_frame(frame).unwrap().as_str(), "es-AR");
    }

    #[test]
    fn ignores_non_initialize() {
        let frame = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list",
            "params":{"_meta":{"acceptLanguage":"es-AR"}}}"#;
        assert!(locale_from_initialize_frame(frame).is_none());
    }

    #[test]
    fn none_when_meta_absent() {
        assert!(locale_from_initialize_frame(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#).is_none());
    }

    #[test]
    fn none_for_malformed_json() {
        assert!(locale_from_initialize_frame("not json").is_none());
    }
}
