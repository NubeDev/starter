//! Merge a raw `PATCH` JSON object into a `starter-prefs` user-layer row.
//!
//! LAYER: transport helper for [`super::preferences`]. The PATCH body is parsed
//! as a `serde_json::Map`, not the typed [`PreferencesPatch`], because the typed
//! shape collapses "missing key" and "explicit null" into the same `None`. The
//! route must keep them apart: a missing key leaves the field unchanged, an
//! explicit `null` reverts it to inherit (SQL `NULL`). Same semantics as
//! `starter-prefs`' own route layer — kept here only so nexus can pin the
//! workspace to the caller's tenant rather than honour a spoofable `?org=`.

use serde::Deserialize;
use serde_json::{Map, Value as JsonValue};
use starter_prefs::resolver::{StringPref, UnitPref, UserPrefsRow};
use starter_spi::preferences::{
    DateFormat, NumberFormat, Theme, TimeFormat, UnitSystem, WeekStart,
};
use starter_spi::units::Unit;

/// Apply a parsed PATCH map to a user-layer row in place. `null` reverts a
/// field to inherit (`None`), an absent key leaves it unchanged, any other
/// value sets it. Returns the offending field's message on a bad value.
pub fn apply_user_patch(
    row: &mut UserPrefsRow,
    patch: &Map<String, JsonValue>,
) -> Result<(), String> {
    for (key, value) in patch {
        match key.as_str() {
            "timezone" => row.timezone = opt_string_pref(value, key)?,
            "locale" => row.locale = opt_string(value, key)?,
            "language" => row.language = opt_string(value, key)?,
            "unit_system" => row.unit_system = opt_enum::<UnitSystem>(value, key)?,
            "temperature_unit" => row.temperature_unit = opt_unit(value, key)?,
            "pressure_unit" => row.pressure_unit = opt_unit(value, key)?,
            "speed_unit" => row.speed_unit = opt_unit(value, key)?,
            "length_unit" => row.length_unit = opt_unit(value, key)?,
            "mass_unit" => row.mass_unit = opt_unit(value, key)?,
            "date_format" => row.date_format = opt_enum::<DateFormat>(value, key)?,
            "time_format" => row.time_format = opt_enum::<TimeFormat>(value, key)?,
            "week_start" => row.week_start = opt_enum::<WeekStart>(value, key)?,
            "number_format" => row.number_format = opt_enum::<NumberFormat>(value, key)?,
            "currency" => row.currency = opt_string_pref(value, key)?,
            "theme" => row.theme = opt_enum::<Theme>(value, key)?,
            other => return Err(format!("unknown field {other:?}")),
        }
    }
    Ok(())
}

fn opt_string(value: &JsonValue, key: &str) -> Result<Option<String>, String> {
    match value {
        JsonValue::Null => Ok(None),
        JsonValue::String(s) => Ok(Some(s.clone())),
        _ => Err(format!("{key:?} must be a string or null")),
    }
}

fn opt_string_pref(value: &JsonValue, key: &str) -> Result<Option<StringPref>, String> {
    match value {
        JsonValue::Null => Ok(None),
        JsonValue::String(s) => Ok(Some(StringPref::parse(s))),
        _ => Err(format!("{key:?} must be a string or null")),
    }
}

fn opt_enum<T: for<'de> Deserialize<'de>>(
    value: &JsonValue,
    key: &str,
) -> Result<Option<T>, String> {
    match value {
        JsonValue::Null => Ok(None),
        other => serde_json::from_value::<T>(other.clone())
            .map(Some)
            .map_err(|e| format!("{key:?}: {e}")),
    }
}

fn opt_unit(value: &JsonValue, key: &str) -> Result<Option<UnitPref>, String> {
    match value {
        JsonValue::Null => Ok(None),
        JsonValue::String(s) if s == "auto" => Ok(Some(UnitPref::Auto)),
        JsonValue::String(_) => serde_json::from_value::<Unit>(value.clone())
            .map(|u| Some(UnitPref::Explicit(u)))
            .map_err(|e| format!("{key:?}: {e}")),
        _ => Err(format!("{key:?} must be a string or null")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_key_leaves_field_unchanged() {
        let mut row = UserPrefsRow {
            locale: Some("en-GB".into()),
            ..Default::default()
        };
        let patch: Map<String, JsonValue> = serde_json::from_str(r#"{"language":"fr"}"#).unwrap();
        apply_user_patch(&mut row, &patch).unwrap();
        assert_eq!(row.locale.as_deref(), Some("en-GB"));
        assert_eq!(row.language.as_deref(), Some("fr"));
    }

    #[test]
    fn explicit_null_reverts_to_inherit() {
        let mut row = UserPrefsRow {
            locale: Some("en-GB".into()),
            ..Default::default()
        };
        let patch: Map<String, JsonValue> = serde_json::from_str(r#"{"locale":null}"#).unwrap();
        apply_user_patch(&mut row, &patch).unwrap();
        assert!(row.locale.is_none());
    }

    #[test]
    fn unit_auto_sentinel_parses() {
        let mut row = UserPrefsRow::default();
        let patch: Map<String, JsonValue> =
            serde_json::from_str(r#"{"temperature_unit":"auto"}"#).unwrap();
        apply_user_patch(&mut row, &patch).unwrap();
        assert_eq!(row.temperature_unit, Some(UnitPref::Auto));
    }

    #[test]
    fn explicit_unit_parses() {
        let mut row = UserPrefsRow::default();
        let patch: Map<String, JsonValue> =
            serde_json::from_str(r#"{"temperature_unit":"fahrenheit"}"#).unwrap();
        apply_user_patch(&mut row, &patch).unwrap();
        assert_eq!(
            row.temperature_unit,
            Some(UnitPref::Explicit(Unit::Fahrenheit))
        );
    }

    #[test]
    fn unknown_field_rejected() {
        let mut row = UserPrefsRow::default();
        let patch: Map<String, JsonValue> = serde_json::from_str(r#"{"bogus":1}"#).unwrap();
        assert!(apply_user_patch(&mut row, &patch).is_err());
    }

    #[test]
    fn bad_value_type_rejected() {
        let mut row = UserPrefsRow::default();
        let patch: Map<String, JsonValue> = serde_json::from_str(r#"{"locale":42}"#).unwrap();
        assert!(apply_user_patch(&mut row, &patch).is_err());
    }
}
