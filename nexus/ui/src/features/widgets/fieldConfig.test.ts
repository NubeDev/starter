import { describe, expect, it } from "vitest";

import type { SeriesField, WidgetConfig } from "@/data/types";
import {
  matchOverride,
  resolveField,
  resolveThresholdSteps,
} from "@/features/widgets/fieldConfig";

// Pure resolution of defaults + overrides + legacy flat fields into the
// effective per-series display (F10).

const base = (overrides: Partial<WidgetConfig> = {}): WidgetConfig => ({
  query: { datasourceId: "ds", sql: "select v from t" },
  fields: { series: [{ value: "temp", label: "Inlet" }] },
  ...overrides,
});

describe("resolveField", () => {
  it("bridges the legacy flat min/max/decimals", () => {
    const r = resolveField({ value: "v" }, base({ min: 0, max: 10, decimals: 2 }));
    expect(r).toMatchObject({ min: 0, max: 10, decimals: 2 });
  });

  it("field defaults win over legacy and seed unit from the series", () => {
    const cfg = base({
      decimals: 0,
      fieldConfig: { defaults: { decimals: 3, unit: "celsius" } },
    });
    const r = resolveField({ value: "temp" }, cfg);
    expect(r.decimals).toBe(3);
    expect(r.unit).toBe("celsius");
  });

  it("a matching override lays its display on top", () => {
    const cfg = base({
      fieldConfig: {
        overrides: [
          {
            matcher: { type: "byRegex", value: "temp" },
            display: { unit: "celsius", color: "0 80% 50%", displayName: "Coolant" },
          },
        ],
      },
    });
    const r = resolveField({ value: "temp_in" }, cfg);
    expect(r.unit).toBe("celsius");
    expect(r.color).toBe("0 80% 50%");
    expect(r.displayName).toBe("Coolant");
  });

  it("an override can hide a series", () => {
    const cfg = base({
      fieldConfig: {
        overrides: [{ matcher: { type: "byName", value: "temp" }, display: { hidden: true } }],
      },
    });
    expect(resolveField({ value: "temp" }, cfg).hidden).toBe(true);
  });
});

describe("matchOverride", () => {
  const overrides = [
    { matcher: { type: "byName" as const, value: "power" }, display: {} },
    { matcher: { type: "byRegex" as const, value: "^temp" }, display: {} },
  ];

  it("matches by exact name against value or label", () => {
    expect(matchOverride({ value: "power" }, overrides)).toBe(overrides[0]);
    expect(matchOverride({ value: "x", label: "power" } as SeriesField, overrides)).toBe(
      overrides[0],
    );
  });

  it("matches by regex and returns undefined when none apply", () => {
    expect(matchOverride({ value: "temp_out" }, overrides)).toBe(overrides[1]);
    expect(matchOverride({ value: "humidity" }, overrides)).toBeUndefined();
  });

  it("a malformed regex matcher does not throw and simply doesn't match", () => {
    const bad = [{ matcher: { type: "byRegex" as const, value: "(" }, display: {} }];
    expect(matchOverride({ value: "anything" }, bad)).toBeUndefined();
  });
});

describe("resolveThresholdSteps", () => {
  it("returns the field-config steps when present", () => {
    const cfg = base({
      fieldConfig: { defaults: { thresholds: [{ value: null, color: "a" }, { value: 80, color: "b" }] } },
    });
    expect(resolveThresholdSteps(cfg)).toHaveLength(2);
  });

  it("returns empty when no steps are configured", () => {
    expect(resolveThresholdSteps(base())).toEqual([]);
  });
});
