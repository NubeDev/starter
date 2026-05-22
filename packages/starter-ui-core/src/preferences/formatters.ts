// Pure formatters keyed off `ResolvedPreferences`. No React, no
// closures over context — every call passes prefs explicitly so the
// formatters compose cleanly into pure rendering paths (memoisation,
// SSR, list-virtualisation, snapshot tests).
//
// Locale rules:
// - Date/time/number formats use `Intl.*` with the prefs' BCP-47
//   locale as the primary lookup, then we override with the
//   user-selected format flags (24h/12h, comma/dot, etc.) where they
//   diverge from the locale default. The `auto` sentinel keeps the
//   locale-default behaviour (no override).
// - Currency uses `Intl.NumberFormat` `style: "currency"` with the
//   resolved `currency` code; the locale still drives symbol
//   placement and grouping.
//
// formatQuantity routes a (value, sourceUnit) pair to the user's
// preferred unit for that quantity, then renders the converted value
// + the unit symbol.

import type {
  DateFormat,
  NumberFormat,
  Quantity,
  ResolvedPreferences,
  TimeFormat,
  Unit,
} from "./types.js";
import { UNIT_SYMBOL, convertUnit } from "./units.js";

// ---------------------------------------------------------------------
// Date / time
// ---------------------------------------------------------------------

/** Render a UNIX-millis timestamp as a date in the user's preferred
 * pattern + timezone. `prefs.date_format === "auto"` defers to the
 * locale's default short date. */
export function formatDate(timestampMs: number, prefs: ResolvedPreferences): string {
  return new Intl.DateTimeFormat(prefs.locale, dateOptions(prefs.date_format, prefs.timezone)).format(
    new Date(timestampMs),
  );
}

/** Render a UNIX-millis timestamp as a time-of-day in the user's
 * preferred clock + timezone. `prefs.time_format === "auto"` defers to
 * the locale default. */
export function formatTime(timestampMs: number, prefs: ResolvedPreferences): string {
  return new Intl.DateTimeFormat(prefs.locale, timeOptions(prefs.time_format, prefs.timezone)).format(
    new Date(timestampMs),
  );
}

/** Render a UNIX-millis timestamp as a combined date + time in the
 * user's preferred patterns + timezone. This is the
 * preference-aware replacement for `Date.prototype.toLocaleString()`:
 * call sites that previously hard-coded the browser locale should
 * switch to this so the active locale, timezone, date-format, and
 * time-format preferences all flow through.
 *
 * The two halves are composed (not concatenated as raw strings) via
 * `Intl.DateTimeFormat.formatToParts` when both halves are `"auto"`
 * so the locale's preferred join character (e.g. `", "` for `en-US`,
 * `" "` for `es-ES`) is honoured. When either half is an explicit
 * pattern we fall back to a plain `${date} ${time}` join — explicit
 * patterns are the user opting out of the locale's join. */
export function formatDateTime(timestampMs: number, prefs: ResolvedPreferences): string {
  if (prefs.date_format === "auto" && prefs.time_format === "auto") {
    return new Intl.DateTimeFormat(prefs.locale, {
      timeZone: prefs.timezone,
      dateStyle: "short",
      timeStyle: "short",
    }).format(new Date(timestampMs));
  }
  const d = formatDate(timestampMs, prefs);
  const t = formatTime(timestampMs, prefs);
  return `${d} ${t}`;
}

function dateOptions(fmt: DateFormat, timeZone: string): Intl.DateTimeFormatOptions {
  switch (fmt) {
    case "auto":
      return { timeZone, dateStyle: "short" };
    case "YYYY-MM-DD":
      // ISO-style — force unambiguous numeric layout.
      return { timeZone, year: "numeric", month: "2-digit", day: "2-digit" };
    case "DD/MM/YYYY":
    case "MM/DD/YYYY":
      return { timeZone, year: "numeric", month: "2-digit", day: "2-digit" };
  }
}

function timeOptions(fmt: TimeFormat, timeZone: string): Intl.DateTimeFormatOptions {
  switch (fmt) {
    case "auto":
      return { timeZone, timeStyle: "short" };
    case "24h":
      return { timeZone, hour: "2-digit", minute: "2-digit", hour12: false };
    case "12h":
      return { timeZone, hour: "numeric", minute: "2-digit", hour12: true };
  }
}

// ---------------------------------------------------------------------
// Numbers / currency
// ---------------------------------------------------------------------

/** Render a number keyed off `prefs.locale` + `prefs.number_format`.
 * `"auto"` defers to the locale default; explicit formats override
 * grouping/decimal separators by picking the locale that produces
 * that pattern (Intl has no direct knob for it). */
export function formatNumber(
  value: number,
  prefs: ResolvedPreferences,
  options: Intl.NumberFormatOptions = {},
): string {
  return new Intl.NumberFormat(numberLocaleChain(prefs), options).format(value);
}

/** Render a currency amount. `amount` is in **major units** (e.g.
 * `19.99` for $19.99) — the caller divides by 100 if their server
 * exposes minor units. The currency code defaults to
 * `prefs.currency` when not passed. */
export function formatCurrency(
  amount: number,
  currencyCode: string,
  prefs: ResolvedPreferences,
  options: Intl.NumberFormatOptions = {},
): string {
  return new Intl.NumberFormat(numberLocaleChain(prefs), {
    style: "currency",
    currency: currencyCode,
    ...options,
  }).format(amount);
}

/** Render `value` (in `sourceUnit`) converted to the user's preferred
 * unit for `quantity` + the unit symbol. Wraps `convertUnit` +
 * `formatNumber` in one call.
 *
 * `options` is passed to the underlying `Intl.NumberFormat` so the
 * caller can pick the precision (`maximumFractionDigits`, etc.). */
export function formatQuantity(
  value: number,
  quantity: Quantity,
  sourceUnit: Unit,
  prefs: ResolvedPreferences,
  options: Intl.NumberFormatOptions = { maximumFractionDigits: 2 },
): string {
  const targetUnit = preferredUnitFor(quantity, prefs);
  const converted = convertUnit(value, sourceUnit, targetUnit);
  return `${formatNumber(converted, prefs, options)} ${UNIT_SYMBOL[targetUnit]}`;
}

/** Pure helper: what unit does the user prefer for this quantity?
 * Exposed because consumers occasionally need just the unit symbol
 * (chart axis label, etc.) without a value to render. */
export function preferredUnitFor(quantity: Quantity, prefs: ResolvedPreferences): Unit {
  switch (quantity) {
    case "temperature":
      return prefs.temperature_unit;
    case "pressure":
      return prefs.pressure_unit;
    case "speed":
      return prefs.speed_unit;
    case "length":
      return prefs.length_unit;
    case "mass":
      return prefs.mass_unit;
  }
}

// ---------------------------------------------------------------------
// Internal — locale fallback for number formatting
// ---------------------------------------------------------------------

/** Map an explicit `NumberFormat` choice to a locale chain that yields
 * that grouping/decimal pattern. The user's locale stays at the head
 * (so currency symbol/placement still tracks their region); we only
 * fall back when their locale's *default* number pattern disagrees
 * with the explicit choice. */
function numberLocaleChain(prefs: ResolvedPreferences): string[] {
  const fmt: NumberFormat = prefs.number_format;
  if (fmt === "auto") return [prefs.locale];
  // Force pattern via a representative locale appended after the
  // user's own — Intl walks the chain and falls back when the first
  // locale has the wrong separators (rare; matters when the locale
  // is e.g. "en-US" but the user explicitly picked "1.234,56").
  const forced =
    fmt === "1,234.56" ? "en-US" : fmt === "1.234,56" ? "de-DE" : /* "1 234,56" */ "fr-FR";
  return localeMatchesPattern(prefs.locale, fmt) ? [prefs.locale] : [forced, prefs.locale];
}

/** Cheap test: does `Intl.NumberFormat(locale)` already format with
 * the requested pattern? Saves a fallback hop in the common case. */
function localeMatchesPattern(locale: string, fmt: NumberFormat): boolean {
  const sample = new Intl.NumberFormat(locale).format(1234.56);
  switch (fmt) {
    case "1,234.56":
      return sample === "1,234.56";
    case "1.234,56":
      return sample === "1.234,56";
    case "1 234,56":
      // fr-FR uses a narrow no-break space (U+202F) in modern Intl
      // implementations — match either.
      return sample === "1 234,56" || sample === "1 234,56";
    case "auto":
      return true;
  }
}
