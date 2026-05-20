// Wire types for the preferences module. Mirrors the Rust DTOs in
// `crates/starter-spi/src/preferences/*` and `starter-prefs`. Hand-
// maintained: there is no codegen pass for these structs yet (the
// REST endpoints live in starter-prefs, not in the openapi.json the
// generated client consumes). When the codegen catches up, this file
// goes away and we re-export from `@nube/starter-client-ts`.
//
// Wire spellings come straight from the Rust serde tags — see e.g.
// `DateFormat::IsoYMD #[serde(rename = "YYYY-MM-DD")]`.

/** ISO-4217 currency code carried as a free string on the wire. */
export type CurrencyCode = string;

/** `"metric" | "imperial"` — informational once resolved. */
export type UnitSystem = "metric" | "imperial";

/** Closed set of temperature, pressure, speed, length, mass units.
 * Mirrors `crates/starter-spi/src/units/unit.rs::Unit`. */
export type Unit =
  | "celsius"
  | "fahrenheit"
  | "kilopascal"
  | "psi"
  | "bar"
  | "meter_per_second"
  | "kilometer_per_hour"
  | "mile_per_hour"
  | "knot"
  | "meter"
  | "foot"
  | "kilogram"
  | "pound";

/** Quantity kind — drives unit conversion routing. Mirrors
 * `starter-spi::units::Quantity`. */
export type Quantity = "temperature" | "pressure" | "speed" | "length" | "mass";

/** Closed enum of date-format choices. Wire spellings from
 * `starter-spi::preferences::DateFormat`. */
export type DateFormat = "auto" | "YYYY-MM-DD" | "DD/MM/YYYY" | "MM/DD/YYYY";

/** Closed enum of time-format choices. */
export type TimeFormat = "auto" | "24h" | "12h";

/** Closed enum of number-format choices. */
export type NumberFormat = "auto" | "1,234.56" | "1.234,56" | "1 234,56";

/** `monday | sunday | auto`. */
export type WeekStart = "auto" | "monday" | "sunday";

/** UI theme — user-only. */
export type Theme = "light" | "dark" | "system";

/** Fully-resolved per-principal preferences. Mirrors
 * `starter-spi::preferences::ResolvedPreferences` — every field is
 * concrete (no `auto`/`null` once the server's resolver has run). */
export interface ResolvedPreferences {
  timezone: string;
  locale: string;
  language: string;
  unit_system: UnitSystem;
  temperature_unit: Unit;
  pressure_unit: Unit;
  speed_unit: Unit;
  length_unit: Unit;
  mass_unit: Unit;
  date_format: DateFormat;
  time_format: TimeFormat;
  week_start: WeekStart;
  number_format: NumberFormat;
  currency: CurrencyCode;
  theme: Theme;
}

/** `PATCH` body shape — mirror of [`ResolvedPreferences`] with every
 * field optional. A JSON `null` value means "revert to inherit"; an
 * omitted key means "leave alone". */
export interface PreferencesPatch {
  timezone?: string | null;
  locale?: string | null;
  language?: string | null;
  unit_system?: UnitSystem | null;
  temperature_unit?: Unit | null;
  pressure_unit?: Unit | null;
  speed_unit?: Unit | null;
  length_unit?: Unit | null;
  mass_unit?: Unit | null;
  date_format?: DateFormat | null;
  time_format?: TimeFormat | null;
  week_start?: WeekStart | null;
  number_format?: NumberFormat | null;
  currency?: string | null;
  theme?: Theme | null;
}
