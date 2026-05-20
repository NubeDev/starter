// Local mirror of `@nube/starter-ui-core/preferences/formatters.ts`
// + the minimal slice of `units.ts` (canonical unit symbol + affine
// conversion table) the formatters need. Pure functions, no React,
// no closures over context — they compose into pure rendering paths
// the same way the ui-core versions do.
//
// The SDK keeps its own copy so the dep arrow holds: an extension
// author imports `@nube/starter-ext-sdk-ts`, never `…ui-core`. The
// two files mirror the Rust DTOs in `crates/starter-spi/src/units`
// and the resolver in `crates/starter-prefs` — both copies update
// in the same PR whenever those DTOs change, so realistic drift is
// not a risk.

import type {
  DateFormat,
  NumberFormat,
  Quantity,
  ResolvedPreferences,
  TimeFormat,
  Unit,
} from "./prefs-types.js";

// ---------------------------------------------------------------------
// Unit conversion table (mirror of ui-core/preferences/units.ts).
// ---------------------------------------------------------------------

const UNIT_SYMBOL: Readonly<Record<Unit, string>> = {
  celsius: "°C",
  fahrenheit: "°F",
  kilopascal: "kPa",
  psi: "psi",
  bar: "bar",
  meter_per_second: "m/s",
  kilometer_per_hour: "km/h",
  mile_per_hour: "mph",
  knot: "kn",
  meter: "m",
  foot: "ft",
  kilogram: "kg",
  pound: "lb",
};

const UNIT_QUANTITY: Readonly<Record<Unit, Quantity>> = {
  celsius: "temperature",
  fahrenheit: "temperature",
  kilopascal: "pressure",
  psi: "pressure",
  bar: "pressure",
  meter_per_second: "speed",
  kilometer_per_hour: "speed",
  mile_per_hour: "speed",
  knot: "speed",
  meter: "length",
  foot: "length",
  kilogram: "mass",
  pound: "mass",
};

interface AffineFactor {
  scale: number;
  offset: number;
}

const TO_CANONICAL: Readonly<Record<Unit, AffineFactor>> = {
  celsius: { scale: 1, offset: 0 },
  fahrenheit: { scale: 5 / 9, offset: -32 * (5 / 9) },
  kilopascal: { scale: 1, offset: 0 },
  psi: { scale: 6.894757293168361, offset: 0 },
  bar: { scale: 100, offset: 0 },
  meter_per_second: { scale: 1, offset: 0 },
  kilometer_per_hour: { scale: 1 / 3.6, offset: 0 },
  mile_per_hour: { scale: 0.44704, offset: 0 },
  knot: { scale: 0.5144444444444445, offset: 0 },
  meter: { scale: 1, offset: 0 },
  foot: { scale: 0.3048, offset: 0 },
  kilogram: { scale: 1, offset: 0 },
  pound: { scale: 0.45359237, offset: 0 },
};

function convertUnit(value: number, sourceUnit: Unit, targetUnit: Unit): number {
  if (sourceUnit === targetUnit) return value;
  if (UNIT_QUANTITY[sourceUnit] !== UNIT_QUANTITY[targetUnit]) {
    throw new Error(
      `cannot convert ${sourceUnit} → ${targetUnit}: different quantities`,
    );
  }
  const src = TO_CANONICAL[sourceUnit];
  const dst = TO_CANONICAL[targetUnit];
  const canonical = value * src.scale + src.offset;
  return (canonical - dst.offset) / dst.scale;
}

function preferredUnitFor(quantity: Quantity, prefs: ResolvedPreferences): Unit {
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
// Date / time
// ---------------------------------------------------------------------

export function formatDate(timestampMs: number, prefs: ResolvedPreferences): string {
  return new Intl.DateTimeFormat(
    prefs.locale,
    dateOptions(prefs.date_format, prefs.timezone),
  ).format(new Date(timestampMs));
}

export function formatTime(timestampMs: number, prefs: ResolvedPreferences): string {
  return new Intl.DateTimeFormat(
    prefs.locale,
    timeOptions(prefs.time_format, prefs.timezone),
  ).format(new Date(timestampMs));
}

function dateOptions(fmt: DateFormat, timeZone: string): Intl.DateTimeFormatOptions {
  switch (fmt) {
    case "auto":
      return { timeZone, dateStyle: "short" };
    case "YYYY-MM-DD":
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

export function formatNumber(
  value: number,
  prefs: ResolvedPreferences,
  options: Intl.NumberFormatOptions = {},
): string {
  return new Intl.NumberFormat(numberLocaleChain(prefs), options).format(value);
}

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

function numberLocaleChain(prefs: ResolvedPreferences): string[] {
  const fmt: NumberFormat = prefs.number_format;
  if (fmt === "auto") return [prefs.locale];
  const forced =
    fmt === "1,234.56" ? "en-US" : fmt === "1.234,56" ? "de-DE" : /* "1 234,56" */ "fr-FR";
  return localeMatchesPattern(prefs.locale, fmt) ? [prefs.locale] : [forced, prefs.locale];
}

function localeMatchesPattern(locale: string, fmt: NumberFormat): boolean {
  const sample = new Intl.NumberFormat(locale).format(1234.56);
  switch (fmt) {
    case "1,234.56":
      return sample === "1,234.56";
    case "1.234,56":
      return sample === "1.234,56";
    case "1 234,56":
      return sample === "1 234,56" || sample === "1 234,56";
    case "auto":
      return true;
  }
}
