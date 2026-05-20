// Static unit registry + conversion table. Mirrors
// `crates/starter-spi/src/units/registry.rs::StaticRegistry` +
// `crates/starter-spi/src/units/convert.rs::normalize_for_storage`.
//
// Keep in sync rule: a unit added to the Rust `Unit` enum MUST land
// here at the same time. `units.test.ts` cross-checks the closed set
// against a JSON snapshot of `/v1/units` (committed under
// `__fixtures__/units.json`); the test fails the moment the wire
// drifts from the TS map, which is how we catch a missed update.

import type { Quantity, Unit } from "./types.js";

/** Canonical SI unit for each quantity — matches the Rust registry. */
export const CANONICAL_UNIT: Readonly<Record<Quantity, Unit>> = {
  temperature: "celsius",
  pressure: "kilopascal",
  speed: "meter_per_second",
  length: "meter",
  mass: "kilogram",
};

/** Allowed units per quantity — matches `Quantity::allowed_units`. */
export const ALLOWED_UNITS: Readonly<Record<Quantity, readonly Unit[]>> = {
  temperature: ["celsius", "fahrenheit"],
  pressure: ["kilopascal", "psi", "bar"],
  speed: ["meter_per_second", "kilometer_per_hour", "mile_per_hour", "knot"],
  length: ["meter", "foot"],
  mass: ["kilogram", "pound"],
};

/** Display symbol for each unit. Pure presentation; not on the wire. */
export const UNIT_SYMBOL: Readonly<Record<Unit, string>> = {
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

/** Quantity each unit belongs to. Reverse index of `ALLOWED_UNITS`. */
export const UNIT_QUANTITY: Readonly<Record<Unit, Quantity>> = {
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

// Affine conversion: canonical = value * scale + offset.
// Inverse: value = (canonical - offset) / scale.
interface AffineFactor {
  scale: number;
  offset: number;
}

/** Per-unit factor that maps `unit -> canonical`. Identity for canonical
 * units. Factors transcribed from `uom` — see `convert.rs`. */
export const TO_CANONICAL: Readonly<Record<Unit, AffineFactor>> = {
  // Temperature: canonical = celsius.
  celsius: { scale: 1, offset: 0 },
  fahrenheit: { scale: 5 / 9, offset: -32 * (5 / 9) },
  // Pressure: canonical = kilopascal.
  kilopascal: { scale: 1, offset: 0 },
  psi: { scale: 6.894757293168361, offset: 0 },
  bar: { scale: 100, offset: 0 },
  // Speed: canonical = meter_per_second.
  meter_per_second: { scale: 1, offset: 0 },
  kilometer_per_hour: { scale: 1 / 3.6, offset: 0 },
  mile_per_hour: { scale: 0.44704, offset: 0 },
  knot: { scale: 0.5144444444444445, offset: 0 },
  // Length: canonical = meter.
  meter: { scale: 1, offset: 0 },
  foot: { scale: 0.3048, offset: 0 },
  // Mass: canonical = kilogram.
  kilogram: { scale: 1, offset: 0 },
  pound: { scale: 0.45359237, offset: 0 },
};

/** Raised by `convertUnit` when the `(sourceUnit, targetUnit)` pair
 * does not share a quantity. */
export class UnitConversionError extends Error {
  constructor(
    public readonly sourceUnit: Unit,
    public readonly targetUnit: Unit,
  ) {
    super(`cannot convert ${sourceUnit} → ${targetUnit}: different quantities`);
    this.name = "UnitConversionError";
  }
}

/** Convert `value` from `sourceUnit` to `targetUnit`. Routes via the
 * canonical unit (so we only need N factors, not N²).
 *
 * Throws `UnitConversionError` if the units belong to different
 * quantities. */
export function convertUnit(value: number, sourceUnit: Unit, targetUnit: Unit): number {
  if (sourceUnit === targetUnit) return value;
  if (UNIT_QUANTITY[sourceUnit] !== UNIT_QUANTITY[targetUnit]) {
    throw new UnitConversionError(sourceUnit, targetUnit);
  }
  const src = TO_CANONICAL[sourceUnit];
  const dst = TO_CANONICAL[targetUnit];
  const canonical = value * src.scale + src.offset;
  return (canonical - dst.offset) / dst.scale;
}

/** The wire-mirror of `GET /v1/units` — same shape as
 * `crates/starter-client-rs::endpoints::prefs::UnitsResponse`. Used by
 * `units.test.ts` to assert the static map stays in sync. */
export interface UnitsResponse {
  quantities: ReadonlyArray<{
    quantity: string;
    canonical: string;
    allowed: readonly string[];
  }>;
}
