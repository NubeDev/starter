// Local mirror of `@nube/starter-ui-core/preferences` wire types.
//
// The SDK never imports from `@nube/starter-ui-core` (SCOPE: TS dep
// arrow — extension authors depend on `starter-ext-sdk-ts` +
// `starter-ui-kit` + `starter-client-ts`, never on the consumer's
// brain). The types are stable — they mirror the Rust DTOs in
// `crates/starter-spi/src/preferences/*` exactly as ui-core does —
// so a local copy here keeps the dep arrow honest without any
// realistic drift risk: any time those Rust DTOs change, both the
// ui-core mirror and this mirror update in the same PR.
//
// Wire spellings come straight from the Rust serde tags.

export type CurrencyCode = string;

export type UnitSystem = "metric" | "imperial";

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

export type Quantity = "temperature" | "pressure" | "speed" | "length" | "mass";

export type DateFormat = "auto" | "YYYY-MM-DD" | "DD/MM/YYYY" | "MM/DD/YYYY";
export type TimeFormat = "auto" | "24h" | "12h";
export type NumberFormat = "auto" | "1,234.56" | "1.234,56" | "1 234,56";
export type WeekStart = "auto" | "monday" | "sunday";
export type Theme = "light" | "dark" | "system";

/** Fully-resolved per-principal preferences. Mirrors
 * `@nube/starter-ui-core/preferences#ResolvedPreferences` and the
 * Rust `starter_spi::preferences::ResolvedPreferences` DTO. */
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

/** `PATCH` body shape. Every field optional; `null` means revert to
 * inherit; an omitted key means leave alone. */
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

/**
 * The shape carried by the host's `PreferencesContext` singleton.
 * Mirrors `@nube/starter-ui-core/preferences#PreferencesContextValue`.
 * The SDK reads this off the singleton handle rather than importing
 * the real type.
 */
export interface HostPreferencesContextValue {
  preferences: ResolvedPreferences | null;
  isLoading: boolean;
  error: unknown;
  setPreferences: (patch: PreferencesPatch) => Promise<void>;
}

/**
 * Duck-typed slice of react-intl's `IntlShape` we depend on. Keeping
 * this minimal avoids pulling `react-intl` into the SDK's type
 * graph (it would otherwise become a hard peer dep on the
 * extension's bundle).
 */
export interface HostIntlShape {
  formatMessage(
    descriptor: { id: string; defaultMessage?: string },
    values?: Record<string, string | number | boolean | Date | null | undefined>,
  ): string;
}

/**
 * The shape carried by the host's `IntlContext` singleton. Mirrors
 * `@nube/starter-ui-core/i18n#IntlContextValue`.
 */
export interface HostIntlContextValue {
  language: string;
  manifest: Readonly<Record<string, string>> | null;
  isLoading: boolean;
  error: unknown;
  intl: HostIntlShape;
}
