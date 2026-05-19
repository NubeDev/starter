//! Unit tests for the `i18n` module.
//!
//! These pin the directive-locked behaviour for stage 5 of the
//! Phase 0 wire-surface job:
//!
//! - `LanguageTag` accepts `"en"`, `"en-US"`, `"en-AU"`, `"zh-TW"`;
//!   rejects `""`, `"not a tag"`, and `"en_US"` (underscore is wrong);
//! - `MessageKey` rejects empty / leading-dot / trailing-dot /
//!   double-dot inputs;
//! - `Diagnostic` JSON round-trip preserves param **order** (the
//!   `BTreeMap` choice forces deterministic serialisation, so the same
//!   bytes come back out).

use std::collections::BTreeMap;

use super::{Diagnostic, DiagnosticParam, I18nError, LanguageTag, MessageKey};

// -- LanguageTag --------------------------------------------------------

#[test]
fn language_tag_accepts_canonical_bcp47_forms() {
    for ok in ["en", "en-US", "en-AU", "zh-TW"] {
        let tag = LanguageTag::parse(ok).unwrap_or_else(|e| panic!("expected {ok:?} ok: {e}"));
        assert_eq!(tag.as_str(), ok, "round-trip preserves input bytes");
    }
}

#[test]
fn language_tag_rejects_empty() {
    let err = LanguageTag::parse("").unwrap_err();
    assert!(matches!(err, I18nError::InvalidLanguageTag { .. }));
}

#[test]
fn language_tag_rejects_garbage() {
    let err = LanguageTag::parse("not a tag").unwrap_err();
    assert!(matches!(err, I18nError::InvalidLanguageTag { .. }));
}

#[test]
fn language_tag_rejects_underscore_separator() {
    // The whole point of pulling in icu_locale_core: the BCP-47
    // separator is `-`, never `_`. A producer that emits `en_US`
    // (POSIX locale form) must be caught here, not silently passed
    // through.
    let err = LanguageTag::parse("en_US").unwrap_err();
    assert!(matches!(err, I18nError::InvalidLanguageTag { .. }));
}

#[test]
fn language_tag_serde_round_trip() {
    let tag = LanguageTag::parse("en-US").unwrap();
    let json = serde_json::to_string(&tag).unwrap();
    assert_eq!(json, "\"en-US\"");
    let back: LanguageTag = serde_json::from_str(&json).unwrap();
    assert_eq!(back, tag);
}

#[test]
fn language_tag_serde_rejects_invalid_at_deserialise() {
    let err = serde_json::from_str::<LanguageTag>("\"en_US\"").unwrap_err();
    assert!(err.to_string().contains("language tag") || err.to_string().contains("BCP-47"));
}

// -- MessageKey ---------------------------------------------------------

#[test]
fn message_key_accepts_canonical_forms() {
    for ok in [
        "flow.error",
        "auth.token.expired",
        "ui.button.save",
        "a",
        "a.b",
    ] {
        let key = MessageKey::parse(ok).unwrap_or_else(|e| panic!("expected {ok:?} ok: {e}"));
        assert_eq!(key.as_str(), ok);
    }
}

#[test]
fn message_key_rejects_empty() {
    let err = MessageKey::parse("").unwrap_err();
    assert!(matches!(err, I18nError::InvalidMessageKey { .. }));
}

#[test]
fn message_key_rejects_whitespace_only() {
    let err = MessageKey::parse("   ").unwrap_err();
    assert!(matches!(err, I18nError::InvalidMessageKey { .. }));
}

#[test]
fn message_key_rejects_leading_dot() {
    let err = MessageKey::parse(".flow.error").unwrap_err();
    assert!(matches!(err, I18nError::InvalidMessageKey { .. }));
}

#[test]
fn message_key_rejects_trailing_dot() {
    let err = MessageKey::parse("flow.error.").unwrap_err();
    assert!(matches!(err, I18nError::InvalidMessageKey { .. }));
}

#[test]
fn message_key_rejects_double_dot() {
    let err = MessageKey::parse("flow..error").unwrap_err();
    assert!(matches!(err, I18nError::InvalidMessageKey { .. }));
}

#[test]
fn message_key_rejects_embedded_whitespace() {
    let err = MessageKey::parse("flow .error").unwrap_err();
    assert!(matches!(err, I18nError::InvalidMessageKey { .. }));
}

#[test]
fn message_key_rejects_non_printable() {
    let err = MessageKey::parse("flow.\u{0007}error").unwrap_err();
    assert!(matches!(err, I18nError::InvalidMessageKey { .. }));
}

#[test]
fn message_key_serde_round_trip() {
    let key = MessageKey::parse("auth.token.expired").unwrap();
    let json = serde_json::to_string(&key).unwrap();
    assert_eq!(json, "\"auth.token.expired\"");
    let back: MessageKey = serde_json::from_str(&json).unwrap();
    assert_eq!(back, key);
}

// -- Diagnostic / DiagnosticParam ---------------------------------------

#[test]
fn diagnostic_param_serde_each_variant() {
    let cases: &[(DiagnosticParam, &str)] = &[
        (DiagnosticParam::String("hi".into()), r#"{"string":"hi"}"#),
        (DiagnosticParam::I64(-7), r#"{"i64":-7}"#),
        (DiagnosticParam::F64(0.5), r#"{"f64":0.5}"#),
        (DiagnosticParam::Bool(true), r#"{"bool":true}"#),
        (
            DiagnosticParam::Timestamp(1_700_000_000_000),
            r#"{"timestamp":1700000000000}"#,
        ),
    ];
    for (value, wire) in cases {
        let s = serde_json::to_string(value).unwrap();
        assert_eq!(&s, wire, "wire form for {value:?}");
        let back: DiagnosticParam = serde_json::from_str(&s).unwrap();
        assert_eq!(&back, value);
    }
}

#[test]
fn diagnostic_round_trip_preserves_param_order() {
    // Insert in non-sorted insertion order. BTreeMap orders by key, so
    // the JSON should come out with keys in lexicographic order, and a
    // re-serialise of the parsed value should produce the exact same
    // bytes — that's what "preserves order" means for a deterministic
    // wire form.
    let mut params: BTreeMap<String, DiagnosticParam> = BTreeMap::new();
    params.insert("zulu".into(), DiagnosticParam::Bool(false));
    params.insert("alpha".into(), DiagnosticParam::I64(1));
    params.insert("mike".into(), DiagnosticParam::String("ok".into()));

    let diag = Diagnostic {
        code: MessageKey::parse("auth.token.expired").unwrap(),
        params,
    };

    let json = serde_json::to_string(&diag).unwrap();
    // Keys come out alpha < mike < zulu regardless of insertion order.
    let alpha = json.find("\"alpha\"").unwrap();
    let mike = json.find("\"mike\"").unwrap();
    let zulu = json.find("\"zulu\"").unwrap();
    assert!(alpha < mike && mike < zulu, "btreemap ordering: {json}");

    // Round-trip is identity at the JSON-byte level.
    let back: Diagnostic = serde_json::from_str(&json).unwrap();
    assert_eq!(back, diag);
    let again = serde_json::to_string(&back).unwrap();
    assert_eq!(again, json, "second-serialise yields identical bytes");
}

#[test]
fn diagnostic_with_no_params_skips_the_field() {
    let diag = Diagnostic::new(MessageKey::parse("ui.ready").unwrap());
    let json = serde_json::to_string(&diag).unwrap();
    assert_eq!(json, r#"{"code":"ui.ready"}"#);
    let back: Diagnostic = serde_json::from_str(&json).unwrap();
    assert_eq!(back, diag);
}

#[test]
fn diagnostic_builder_attaches_params() {
    let diag = Diagnostic::new(MessageKey::parse("flow.error").unwrap())
        .with_param("node", DiagnosticParam::String("transform".into()))
        .with_param("at", DiagnosticParam::Timestamp(1_700_000_000_000));
    assert_eq!(diag.params.len(), 2);
    assert!(diag.params.contains_key("node"));
    assert!(diag.params.contains_key("at"));
}
