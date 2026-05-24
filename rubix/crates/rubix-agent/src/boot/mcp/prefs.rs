//! BCP-47 [`LanguageTag`] → [`ResolvedPreferences`] mapping.
//!
//! Called from the seed adapter at MCP `tools/call` dispatch time
//! and from sibling code in [`crate::routes::tools`] /
//! [`crate::bin::rubix_admin`] that wants the same locale → prefs
//! mapping outside the flow path. See
//! [docs/design/i18n-prefs/](../../../../docs/design/i18n-prefs/README.md)
//! for the four-axis preferences model (locale, language, timezone,
//! unit system) this helper produces a reasonable default for.

use starter_spi::i18n::LanguageTag;
use starter_spi::preferences::{
    DateFormat, NumberFormat, ResolvedPreferences, Theme, TimeFormat, UnitSystem, WeekStart,
};
use starter_spi::units::Unit;

/// Map a BCP-47 [`LanguageTag`] to a [`ResolvedPreferences`] whose
/// timezone, date format, time format, and language reflect a
/// reasonable default for the tag's region subtag. Unknown tags fall
/// back to UTC + ISO date.
pub fn prefs_from_locale(tag: &LanguageTag) -> ResolvedPreferences {
    let raw = tag.as_str();
    let (timezone, locale, language, date_format, time_format) = match raw {
        "en-US" => (
            "America/New_York",
            "en-US",
            "en",
            DateFormat::MdySlash,
            TimeFormat::H24,
        ),
        "es-AR" => (
            "America/Argentina/Buenos_Aires",
            "es-AR",
            "es",
            DateFormat::DmySlash,
            TimeFormat::H24,
        ),
        _ => {
            // Fall back to the language-only subtag for the i18n
            // catalogue lookup; UTC / ISO date stay neutral so the
            // operator at least sees a parseable timestamp.
            let lang = raw.split('-').next().unwrap_or("en");
            (
                "UTC",
                raw,
                if lang.is_empty() { "en" } else { lang },
                DateFormat::IsoYMD,
                TimeFormat::H24,
            )
        }
    };
    ResolvedPreferences {
        timezone: timezone.to_owned(),
        locale: locale.to_owned(),
        language: language.to_owned(),
        unit_system: UnitSystem::Metric,
        temperature_unit: Unit::Celsius,
        pressure_unit: Unit::Kilopascal,
        speed_unit: Unit::MeterPerSecond,
        length_unit: Unit::Meter,
        mass_unit: Unit::Kilogram,
        date_format,
        time_format,
        week_start: WeekStart::Monday,
        number_format: NumberFormat::SpaceComma,
        currency: "USD".to_owned(),
        theme: Theme::System,
    }
}
