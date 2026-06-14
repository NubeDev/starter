import { describe, expect, it } from "vitest";

import type { QueryResponse } from "@/api/types";
import { toWidgetData } from "@/api/query/toWidgetData";

// The query response's rows are JSON objects keyed by column name, which
// is exactly `WidgetData.points`. The reshape is a thin, total mapping —
// pinned here so a contract change (e.g. a nested rows shape) is caught.
describe("toWidgetData", () => {
  it("passes rows through as points keyed by column", () => {
    const res: QueryResponse = {
      columns: [
        { name: "ts", type: "timestamp" },
        { name: "value", type: "float" },
      ],
      rows: [
        { ts: "2026-06-09T00:00:00Z", value: 21.4 },
        { ts: "2026-06-09T00:01:00Z", value: 22.0 },
      ],
      stats: { row_count: 2, byte_count: 64, elapsed_ms: 3, truncated: false },
    };
    const data = toWidgetData(res);
    expect(data.points).toHaveLength(2);
    expect(data.points[0]).toEqual({ ts: "2026-06-09T00:00:00Z", value: 21.4 });
  });

  it("yields an empty point list for an empty result (no fabricated rows)", () => {
    const res: QueryResponse = {
      columns: [],
      rows: [],
      stats: { row_count: 0, byte_count: 0, elapsed_ms: 1, truncated: false },
    };
    expect(toWidgetData(res).points).toEqual([]);
  });
});
