//! Stage 15 — Phase 3 seed catalog consistency.
//!
//! Asserts that the compiled-in English and Spanish starter
//! chrome catalogs have **identical key sets**, every value is a
//! non-empty trimmed string, and every key is a valid
//! [`MessageKey`]. The Phase 4 React UI's "load every starter
//! key in every locale" smoke test depends on this invariant —
//! adding a key to `en.json` without translating it in `es.json`
//! (or vice-versa) is caught here, before the workspace builds.

use std::collections::BTreeSet;

use starter_i18n::catalog::Catalog;
use starter_i18n::platform::{STARTER_EN_JSON, STARTER_ES_JSON, starter_en, starter_es};
use starter_spi::i18n::MessageKey;

fn keys(cat: &Catalog) -> BTreeSet<MessageKey> {
    cat.messages.keys().cloned().collect()
}

#[test]
fn en_and_es_have_identical_key_sets() {
    let en = starter_en();
    let es = starter_es();
    let en_keys = keys(&en);
    let es_keys = keys(&es);

    let only_in_en: Vec<_> = en_keys.difference(&es_keys).collect();
    let only_in_es: Vec<_> = es_keys.difference(&en_keys).collect();

    assert!(
        only_in_en.is_empty() && only_in_es.is_empty(),
        "starter en.json and es.json must have identical key sets; \
         missing-from-es: {only_in_en:?}; missing-from-en: {only_in_es:?}",
    );
}

#[test]
fn both_catalogs_are_non_empty() {
    assert!(!starter_en().is_empty(), "en.json must carry keys");
    assert!(!starter_es().is_empty(), "es.json must carry keys");
}

#[test]
fn every_value_is_a_non_empty_trimmed_string() {
    for (label, cat) in [("en", starter_en()), ("es", starter_es())] {
        for (key, value) in &cat.messages {
            assert!(
                !value.trim().is_empty(),
                "{label}.json: key {key:?} has an empty / whitespace-only value",
            );
        }
    }
}

#[test]
fn every_top_level_key_is_a_valid_message_key() {
    // The catalog loader already enforces this via
    // `MessageKey`'s deserializer, so the fact that
    // `Catalog::from_json_str` succeeded above is the proof.
    // This test exists as an explicit, fail-loud guard so the
    // intent shows up in `cargo test` output.
    starter_en();
    starter_es();
}

#[test]
fn no_byte_order_mark_or_extraneous_leading_whitespace() {
    // JSON-parse tolerates a UTF-8 BOM in some implementations,
    // but a BOM at the head of an `include_str!`'d catalog
    // changes the bytes that go through `Catalog::fingerprint`
    // for no semantic reason. Lock the seeds to "no BOM".
    for (label, src) in [("en", STARTER_EN_JSON), ("es", STARTER_ES_JSON)] {
        assert!(
            !src.starts_with('\u{FEFF}'),
            "{label}.json must not start with a UTF-8 BOM",
        );
    }
}

#[test]
fn covers_every_resolved_preferences_field() {
    // One key per ResolvedPreferences field, per the stage 15
    // brief. If a new preferences field lands, the loader test
    // here is the place that catches the missing label.
    let required = [
        "starter.settings.preferences.timezone.label",
        "starter.settings.preferences.locale.label",
        "starter.settings.preferences.language.label",
        "starter.settings.preferences.unit_system.label",
        "starter.settings.preferences.temperature_unit.label",
        "starter.settings.preferences.pressure_unit.label",
        "starter.settings.preferences.speed_unit.label",
        "starter.settings.preferences.length_unit.label",
        "starter.settings.preferences.mass_unit.label",
        "starter.settings.preferences.date_format.label",
        "starter.settings.preferences.time_format.label",
        "starter.settings.preferences.week_start.label",
        "starter.settings.preferences.number_format.label",
        "starter.settings.preferences.currency.label",
        "starter.settings.preferences.theme.label",
    ];

    for cat in [starter_en(), starter_es()] {
        for raw in required {
            let key = MessageKey::parse(raw).expect("hardcoded key parses");
            assert!(
                cat.get(&key).is_some(),
                "starter catalogs must define preferences field label {raw:?}",
            );
        }
    }
}
