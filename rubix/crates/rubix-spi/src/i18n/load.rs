//! Builds a [`MessageBundle`] populated with rubix's bundled EN + ES
//! catalogues. Consumers (the agent binary, integration tests, the
//! CLI) call [`rubix_bundle`] at boot and hand the bundle to the
//! transport layer that needs it.
//!
//! See [docs/design/i18n-prefs/](../../../docs/design/i18n-prefs/README.md)
//! for the four-transport translation contract.

use starter_i18n::bundle::MessageBundle;
use starter_i18n::catalog::{Catalog, CatalogError};
use starter_spi::i18n::LanguageTag;

use super::{RUBIX_EN_JSON, RUBIX_ES_JSON};

/// Build a [`MessageBundle`] containing the rubix EN + ES catalogues,
/// with `"en"` as the R5 fallback tag.
///
/// Returns `CatalogError` only if one of the embedded JSON strings
/// fails to parse — which would be a build-time bug, not a runtime
/// condition, because the catalogues are baked in via `include_str!`
/// and the parse semantics are deterministic.
pub fn rubix_bundle() -> Result<MessageBundle, CatalogError> {
    let en_tag = LanguageTag::parse("en").expect("'en' is a valid BCP-47 tag");
    let es_tag = LanguageTag::parse("es").expect("'es' is a valid BCP-47 tag");

    let mut bundle = MessageBundle::new(en_tag.clone());
    bundle.insert(en_tag, Catalog::from_json_str(RUBIX_EN_JSON)?);
    bundle.insert(es_tag, Catalog::from_json_str(RUBIX_ES_JSON)?);
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
    use starter_spi::preferences::{
        DateFormat, NumberFormat, ResolvedPreferences, Theme, TimeFormat, UnitSystem, WeekStart,
    };
    use starter_spi::units::Quantity;
    use starter_spi::units::Unit;

    use super::*;

    fn tag(s: &str) -> LanguageTag {
        LanguageTag::parse(s).expect("test tag must parse")
    }

    fn key(s: &str) -> MessageKey {
        MessageKey::parse(s).expect("test key must parse")
    }

    fn metric_prefs() -> ResolvedPreferences {
        ResolvedPreferences {
            timezone: "UTC".to_owned(),
            locale: "en-AU".to_owned(),
            language: "en".to_owned(),
            unit_system: UnitSystem::Metric,
            temperature_unit: Unit::Celsius,
            pressure_unit: Unit::Kilopascal,
            speed_unit: Unit::MeterPerSecond,
            length_unit: Unit::Meter,
            mass_unit: Unit::Kilogram,
            date_format: DateFormat::IsoYMD,
            time_format: TimeFormat::H24,
            week_start: WeekStart::Monday,
            number_format: NumberFormat::SpaceComma,
            currency: "AUD".to_owned(),
            theme: Theme::Dark,
        }
    }

    #[test]
    fn rubix_bundle_loads_en_and_es() {
        let bundle = rubix_bundle().expect("embedded catalogues parse");
        let langs: Vec<&str> = bundle.languages().map(|tag| tag.as_str()).collect();
        assert!(langs.contains(&"en"), "en catalogue must be registered");
        assert!(langs.contains(&"es"), "es catalogue must be registered");
    }

    #[test]
    fn skill_denied_renders_in_en_and_es() {
        let bundle = rubix_bundle().expect("embedded catalogues parse");
        let mut params = BTreeMap::new();
        params.insert(
            "skill".to_owned(),
            DiagnosticParam::String("system-checker".to_owned()),
        );
        params.insert(
            "tool".to_owned(),
            DiagnosticParam::String("rubix.system.disk".to_owned()),
        );

        let en = bundle.render(&tag("en"), &key("rubix.skill.denied"), &params);
        assert_eq!(
            en,
            "Skill system-checker does not allow tool rubix.system.disk."
        );

        let es = bundle.render(&tag("es"), &key("rubix.skill.denied"), &params);
        assert_eq!(
            es,
            "La habilidad system-checker no permite la herramienta rubix.system.disk."
        );
    }

    #[test]
    fn render_diagnostic_converts_quantity_for_caller_prefs() {
        // The bundle ships keys with no Quantity slot at launch
        // (Target C wires the disk keys). The Quantity round-trip
        // is exercised against a small ad-hoc catalog so the test
        // proves the wiring without coupling to a catalogue entry
        // that does not yet exist.
        let mut bundle = rubix_bundle().expect("embedded catalogues parse");
        let mut ad_hoc = BTreeMap::new();
        ad_hoc.insert(key("rubix.test.length"), "Free space: {free}.".to_owned());
        bundle.extend(tag("en"), Catalog { messages: ad_hoc });

        // 12.5 GB expressed in canonical SI metres-equivalent for
        // Length is just metres; the renderer formats it per
        // length_unit. Metric prefs → metres.
        let mut params = BTreeMap::new();
        params.insert(
            "free".to_owned(),
            DiagnosticParam::Quantity {
                canonical: 1_500.0,
                quantity: Quantity::Length,
            },
        );
        let diag = Diagnostic {
            code: key("rubix.test.length"),
            params,
        };

        let rendered = bundle.render_diagnostic(&tag("en"), &diag, &metric_prefs());
        // Metric prefs select metres; the renderer emits the value
        // in the caller's preferred unit. Exact formatting belongs
        // to starter-prefs; we assert only that the unit hint
        // (metres) appears, not the precise decimal.
        assert!(
            rendered.contains("m"),
            "rendered output {rendered:?} must mention metric unit",
        );
        assert!(rendered.starts_with("Free space:"));
    }

    #[test]
    fn disk_warn_renders_timestamp_in_caller_timezone() {
        // Proves S5 (timezone-aware Timestamp rendering) flows
        // through the rubix-bundled catalogue. Same epoch ms, two
        // callers, two different wall-clock outputs.
        let bundle = rubix_bundle().expect("embedded catalogues parse");

        // 2024-01-15 12:00:00 UTC.
        let epoch_ms = 1_705_320_000_000_i64;

        let mut params = BTreeMap::new();
        params.insert("percent".to_owned(), DiagnosticParam::I64(89));
        params.insert("free".to_owned(), DiagnosticParam::I64(125_000_000_000));
        params.insert("at".to_owned(), DiagnosticParam::Timestamp(epoch_ms));
        let diag = Diagnostic {
            code: key("rubix.system.disk.warn"),
            params,
        };

        let eu_prefs = ResolvedPreferences {
            timezone: "Europe/Paris".to_owned(),
            ..metric_prefs()
        };
        let mut us_prefs = metric_prefs();
        us_prefs.timezone = "America/New_York".to_owned();

        let eu = bundle.render_diagnostic(&tag("en"), &diag, &eu_prefs);
        let us = bundle.render_diagnostic(&tag("en"), &diag, &us_prefs);

        // Paris is UTC+1 in January → 13:00; New York is UTC-5 → 07:00.
        assert!(
            eu.contains("13:00"),
            "EU rendering missing 13:00 — got {eu:?}"
        );
        assert!(
            us.contains("07:00"),
            "US rendering missing 07:00 — got {us:?}"
        );
        assert!(eu.contains("89"));
        assert!(us.starts_with("Disk is nearly full"));
    }
}
