import { describe, expect, it } from "vitest";

import type { SeriesPoint, Transform, WidgetData } from "@/data/types";
import {
  applyCalculated,
  applyFilter,
  applyGroupBy,
  applyOrganize,
  applyReduce,
  applyRename,
  applyTransforms,
} from "@/features/canvas/transforms";

// Pure transform pipeline (F10): rows in, rows out, no fetch, no mutation.
// Each transform is pinned in isolation, then the orchestrator's ordering.

const rows: SeriesPoint[] = [
  { site: "A", power: 10, area: 2 },
  { site: "A", power: 30, area: 2 },
  { site: "B", power: 50, area: 5 },
];

describe("applyRename", () => {
  it("renames a field across rows and drops the old key", () => {
    const out = applyRename(rows, "power", "kw");
    expect(out[0]).toEqual({ site: "A", kw: 10, area: 2 });
    expect(out[0]).not.toHaveProperty("power");
  });

  it("does not mutate the input", () => {
    applyRename(rows, "power", "kw");
    expect(rows[0]).toHaveProperty("power");
  });

  it("is a no-op when from equals to or from is absent", () => {
    expect(applyRename(rows, "power", "power")).toEqual(rows);
    expect(applyRename(rows, "ghost", "x")[0]).toEqual(rows[0]);
  });
});

describe("applyCalculated", () => {
  it("adds left <op> right as a new field", () => {
    const out = applyCalculated(rows, "density", "power", "/", "area");
    expect(out[0].density).toBe(5);
    expect(out[2].density).toBe(10);
  });

  it("guards division by zero and non-numeric operands with null", () => {
    const out = applyCalculated(
      [{ a: 1, b: 0 }, { a: "x", b: 2 }],
      "r",
      "a",
      "/",
      "b",
    );
    expect(out[0].r).toBeNull();
    expect(out[1].r).toBeNull();
  });
});

describe("applyFilter", () => {
  it("keeps rows passing a numeric comparison", () => {
    expect(applyFilter(rows, "power", ">=", "30")).toHaveLength(2);
  });

  it("keeps rows passing an equality on text", () => {
    expect(applyFilter(rows, "site", "=", "A")).toHaveLength(2);
    expect(applyFilter(rows, "site", "!=", "A")).toHaveLength(1);
  });
});

describe("applyGroupBy", () => {
  it("aggregates by a key in first-appearance order", () => {
    const out = applyGroupBy(rows, "site", "power", "sum", "total");
    expect(out).toEqual([
      { site: "A", total: 40 },
      { site: "B", total: 50 },
    ]);
  });

  it("avg and count behave", () => {
    expect(applyGroupBy(rows, "site", "power", "avg", "t")[0].t).toBe(20);
    expect(applyGroupBy(rows, "site", "power", "count", "n")[0].n).toBe(2);
  });
});

describe("applyReduce", () => {
  it("reduces to a single row with the chosen calc", () => {
    expect(applyReduce(rows, "power", "sum", "v")).toEqual([{ v: 90 }]);
    expect(applyReduce(rows, "power", "max", "v")).toEqual([{ v: 50 }]);
    expect(applyReduce(rows, "power", "last", "v")).toEqual([{ v: 50 }]);
  });

  it("empty input reduces to a single null", () => {
    expect(applyReduce([], "power", "avg", "v")).toEqual([{ v: null }]);
  });
});

describe("applyOrganize", () => {
  it("reorders listed columns first, keeps the rest", () => {
    const out = applyOrganize(rows, ["area", "site"]);
    expect(Object.keys(out[0])).toEqual(["area", "site", "power"]);
  });
});

describe("applyTransforms", () => {
  const data: WidgetData = { points: rows };

  it("runs the pipeline in order, each feeding the next", () => {
    const pipeline: Transform[] = [
      { kind: "filter", field: "site", op: "=", value: "A" },
      { kind: "reduce", field: "power", calc: "sum", as: "total" },
    ];
    expect(applyTransforms(data, pipeline).points).toEqual([{ total: 40 }]);
  });

  it("returns the data untouched when there are no transforms", () => {
    expect(applyTransforms(data, undefined)).toBe(data);
    expect(applyTransforms(data, [])).toBe(data);
  });
});
