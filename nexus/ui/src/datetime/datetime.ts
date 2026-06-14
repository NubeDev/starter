// Centralised date/time display for nexus/ui.
//
// One import, region-aware, preference-ready. Call sites never touch
// `Intl` or `toLocaleString` directly — they go through `useDateTime()`
// (React) or `formatWith*` (pure). Display conventions come from, in
// priority order:
//
//   1. A mounted `<PreferencesProvider>` (real org/user prefs) — when
//      present, its `ResolvedPreferences` wins.
//   2. The active `Region` preset (USA / EU / China) from
//      `datetime/store.ts` — the zero-backend default.
//
// To later "plug in" org/user preferences, a developer just mounts
// `PreferencesProvider` (already in `@nube/starter-ui-core/preferences`)
// above the app — no call-site changes. Until then the region preset
// drives everything.

import type { ResolvedPreferences } from "@nube/starter-ui-core/preferences";
import {
  formatDate,
  formatDateTime,
  formatTime,
} from "@nube/starter-ui-core/preferences";

import { REGION_PREFS, type Region, type RegionPreferences } from "@/datetime/regions";
import type { DateTimeSettings } from "@/datetime/store";

/** Anything we can format: epoch millis, an ISO/parseable string, or a
 * `Date`. Normalised to epoch millis before hitting the formatters. */
export type DateInput = number | string | Date;

/** Non-display defaults that fill out a full `ResolvedPreferences` when
 * we only have a region's display slice. Units/currency/theme/language
 * are irrelevant to date/time formatting but the formatter type wants a
 * complete object; these are inert placeholders, never rendered by the
 * date/time paths. */
const BASE_PREFS: ResolvedPreferences = {
  timezone: "UTC",
  locale: "en-US",
  language: "en",
  unit_system: "metric",
  temperature_unit: "celsius",
  pressure_unit: "kilopascal",
  speed_unit: "meter_per_second",
  length_unit: "meter",
  mass_unit: "kilogram",
  date_format: "auto",
  time_format: "auto",
  week_start: "auto",
  number_format: "auto",
  currency: "USD",
  theme: "system",
};

/** Promote a region's display slice to a full `ResolvedPreferences` the
 * platform formatters accept. */
export function prefsForRegion(region: Region): ResolvedPreferences {
  return mergePrefs(REGION_PREFS[region]);
}

/** Merge a partial display preference over the inert base. */
export function mergePrefs(partial: RegionPreferences): ResolvedPreferences {
  return { ...BASE_PREFS, ...partial };
}

/** The device's BCP-47 locale (e.g. "en-GB"), or "en-US" off-DOM. Drives
 * "Automatic" date/time/number conventions. */
export function deviceLocale(): string {
  if (typeof navigator !== "undefined" && navigator.language) {
    return navigator.language;
  }
  return BASE_PREFS.locale;
}

/** The device's IANA time zone (e.g. "Europe/Paris"), or "UTC" if the
 * runtime can't report one. Drives "Automatic" time zone. */
export function deviceTimeZone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || BASE_PREFS.timezone;
  } catch {
    return BASE_PREFS.timezone;
  }
}

/**
 * Build a full `ResolvedPreferences` from the user's explicit local
 * display settings (`DateTimeSettings`). Each setting's "automatic"
 * value defers to the device: empty timezone → device zone, and an
 * `auto` date/time format keeps the device locale's default — so the
 * locale is always the device's (the user only overrides the *shape*,
 * never the language). This is the industry-standard model: explicit,
 * independent knobs over a sensible automatic baseline.
 */
export function prefsForSettings(settings: DateTimeSettings): ResolvedPreferences {
  return {
    ...BASE_PREFS,
    locale: deviceLocale(),
    timezone: settings.timezone || deviceTimeZone(),
    date_format: settings.dateFormat,
    time_format: settings.timeFormat,
  };
}

/** Coerce any accepted input to epoch millis. Throws on an unparseable
 * string so a bad timestamp surfaces at the call site rather than
 * silently rendering "Invalid Date". */
export function toEpochMs(input: DateInput): number {
  if (typeof input === "number") return input;
  const ms = input instanceof Date ? input.getTime() : Date.parse(input);
  if (Number.isNaN(ms)) {
    throw new RangeError(`Unparseable date input: ${String(input)}`);
  }
  return ms;
}

/* -------------------------------------------------------------------------
 * Pure formatters — take explicit prefs. Use these off the React tree
 * (chart option builders, tests, workers). In components prefer the
 * `useDateTime()` hook, which binds the active prefs for you.
 * ----------------------------------------------------------------------- */

export function formatDateWith(input: DateInput, prefs: ResolvedPreferences): string {
  return formatDate(toEpochMs(input), prefs);
}

export function formatTimeWith(input: DateInput, prefs: ResolvedPreferences): string {
  return formatTime(toEpochMs(input), prefs);
}

export function formatDateTimeWith(input: DateInput, prefs: ResolvedPreferences): string {
  return formatDateTime(toEpochMs(input), prefs);
}

/** A from→to range, e.g. "12/01/2026 – 12/08/2026". Same date on both
 * ends collapses to a single value. */
export function formatRangeWith(
  from: DateInput,
  to: DateInput,
  prefs: ResolvedPreferences,
): string {
  const a = formatDateWith(from, prefs);
  const b = formatDateWith(to, prefs);
  return a === b ? a : `${a} – ${b}`;
}

/** The bound formatter set returned by `useDateTime()`. */
export interface DateTimeFormatters {
  /** The prefs these formatters are bound to (region or org/user). */
  prefs: ResolvedPreferences;
  date: (input: DateInput) => string;
  time: (input: DateInput) => string;
  dateTime: (input: DateInput) => string;
  range: (from: DateInput, to: DateInput) => string;
}

/** Build a bound formatter set from concrete prefs. Shared by the hook
 * and any non-React caller that already has prefs in hand. */
export function makeFormatters(prefs: ResolvedPreferences): DateTimeFormatters {
  return {
    prefs,
    date: (i) => formatDateWith(i, prefs),
    time: (i) => formatTimeWith(i, prefs),
    dateTime: (i) => formatDateTimeWith(i, prefs),
    range: (a, b) => formatRangeWith(a, b, prefs),
  };
}
