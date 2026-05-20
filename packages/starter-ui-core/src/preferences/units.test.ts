// units.ts cross-check + conversion sanity.
//
// The fixture under `__fixtures__/units.json` is a snapshot of what
// `GET /v1/units` is expected to return (the closed `StaticRegistry`
// in `starter-spi`). This test asserts the TS static map mirrors it
// exactly — a missed update on either side fails here loudly.

import { describe, expect, it } from "vitest";

import unitsFixture from "./__fixtures__/units.json" with { type: "json" };
import {
  ALLOWED_UNITS,
  CANONICAL_UNIT,
  UNIT_QUANTITY,
  UNIT_SYMBOL,
  UnitConversionError,
  convertUnit,
} from "./units.js";
import type { Quantity, Unit, UnitsResponse } from "./index.js";

const fixture = unitsFixture as UnitsResponse;

describe("static unit map mirrors GET /v1/units", () => {
  it("covers the same quantities", () => {
    const fixtureQuantities = fixture.quantities.map((q) => q.quantity).sort();
    const tsQuantities = Object.keys(CANONICAL_UNIT).sort();
    expect(tsQuantities).toEqual(fixtureQuantities);
  });

  it("canonical units match for every quantity", () => {
    for (const entry of fixture.quantities) {
      expect(CANONICAL_UNIT[entry.quantity as Quantity]).toBe(entry.canonical);
    }
  });

  it("allowed-unit sets match for every quantity", () => {
    for (const entry of fixture.quantities) {
      const q = entry.quantity as Quantity;
      expect([...ALLOWED_UNITS[q]].sort()).toEqual([...entry.allowed].sort());
    }
  });

  it("UNIT_SYMBOL and UNIT_QUANTITY cover every unit in the fixture", () => {
    const allUnits = fixture.quantities.flatMap((q) => q.allowed) as Unit[];
    for (const u of allUnits) {
      expect(UNIT_SYMBOL[u]).toBeTruthy();
      expect(UNIT_QUANTITY[u]).toBeTruthy();
    }
  });
});

describe("convertUnit", () => {
  it("identity converts to the same number", () => {
    expect(convertUnit(42, "celsius", "celsius")).toBe(42);
  });

  it("celsius → fahrenheit + back", () => {
    expect(convertUnit(100, "celsius", "fahrenheit")).toBeCloseTo(212, 6);
    expect(convertUnit(32, "fahrenheit", "celsius")).toBeCloseTo(0, 6);
    expect(convertUnit(72.4, "fahrenheit", "celsius")).toBeCloseTo(22.444444, 5);
  });

  it("psi ↔ kilopascal (matches the Rust integration test)", () => {
    expect(convertUnit(14.5037738, "psi", "kilopascal")).toBeCloseTo(100, 3);
  });

  it("bar → kilopascal", () => {
    expect(convertUnit(1, "bar", "kilopascal")).toBeCloseTo(100, 6);
  });

  it("mph → m/s (matches the Rust integration test)", () => {
    expect(convertUnit(60, "mile_per_hour", "meter_per_second")).toBeCloseTo(26.8224, 4);
  });

  it("km/h → m/s", () => {
    expect(convertUnit(36, "kilometer_per_hour", "meter_per_second")).toBeCloseTo(10, 6);
  });

  it("knot → m/s", () => {
    expect(convertUnit(1, "knot", "meter_per_second")).toBeCloseTo(0.5144444, 6);
  });

  it("foot → meter (matches the Rust integration test)", () => {
    expect(convertUnit(10, "foot", "meter")).toBeCloseTo(3.048, 6);
  });

  it("pound → kilogram (matches the Rust integration test)", () => {
    expect(convertUnit(10, "pound", "kilogram")).toBeCloseTo(4.5359237, 6);
  });

  it("rejects cross-quantity conversion", () => {
    expect(() => convertUnit(1, "celsius", "kilogram")).toThrow(UnitConversionError);
  });
});
