import { describe, expect, it } from "vitest";

import type { DashboardExport } from "@/api/types";
import {
  exportPanelToCreate,
  exportVariableToCreate,
  filterExport,
  parseExport,
  readExportLayout,
  selectAll,
  selectionCount,
} from "@/features/portability/model";

function model(): DashboardExport {
  return {
    schema_version: 1,
    slug: "ops",
    name: "Ops",
    icon: "gauge",
    accent: "152 76% 44%",
    panels: [
      { title: "A", sql: "SELECT 1", viz: "line", datasource_id: "ds-1", layout: { x: 0, y: 0, w: 6, h: 4 } },
      { title: "B", sql: "SELECT 2", viz: "table", datasource_id: null, layout: { x: 6, y: 0, w: 6, h: 4 } },
    ],
    variables: [
      { name: "env", label: "Environment", kind: "custom", multi: false },
      { name: "host", label: null, kind: "query" },
    ],
  };
}

describe("portability model", () => {
  it("selectAll picks every panel index and variable name", () => {
    const sel = selectAll(model());
    expect([...sel.panelIndices].sort()).toEqual([0, 1]);
    expect([...sel.variableNames].sort()).toEqual(["env", "host"]);
    expect(selectionCount(sel)).toEqual({ panels: 2, variables: 2, total: 4 });
  });

  it("filterExport keeps only the selected panels and variables", () => {
    const filtered = filterExport(model(), {
      panelIndices: new Set([1]),
      variableNames: new Set(["host"]),
    });
    expect(filtered.panels).toHaveLength(1);
    expect(filtered.panels[0].title).toBe("B");
    expect(filtered.variables).toHaveLength(1);
    expect(filtered.variables?.[0].name).toBe("host");
    // Appearance/schema carry over unchanged so the subset is a valid export.
    expect(filtered.name).toBe("Ops");
    expect(filtered.schema_version).toBe(1);
  });

  it("readExportLayout falls back for a malformed layout instead of hiding the tile", () => {
    expect(readExportLayout({ x: 2, y: 3, w: 4, h: 2 })).toEqual({ x: 2, y: 3, w: 4, h: 2 });
    expect(readExportLayout(null)).toEqual({ x: 0, y: 0, w: 4, h: 4 });
    expect(readExportLayout({ w: 0, h: -1 })).toEqual({ x: 0, y: 0, w: 4, h: 4 });
  });

  it("exportPanelToCreate maps a null datasource to an empty string the create API can take", () => {
    const req = exportPanelToCreate(model().panels[1]);
    expect(req).toMatchObject({ title: "B", sql: "SELECT 2", datasource_id: "", viz: "table" });
  });

  it("exportVariableToCreate carries the kind across verbatim", () => {
    const req = exportVariableToCreate(model().variables![0]);
    expect(req).toMatchObject({ name: "env", label: "Environment", kind: "custom" });
  });

  describe("parseExport", () => {
    it("accepts a well-formed export and normalises missing variables to []", () => {
      const result = parseExport(
        JSON.stringify({ name: "X", slug: "x", panels: [] }),
      );
      expect(result.ok).toBe(true);
      if (result.ok) expect(result.model.variables).toEqual([]);
    });

    it("rejects non-JSON", () => {
      expect(parseExport("not json").ok).toBe(false);
    });

    it("rejects an object with no panels array", () => {
      const result = parseExport(JSON.stringify({ name: "X", slug: "x" }));
      expect(result.ok).toBe(false);
      if (!result.ok) expect(result.error).toMatch(/panels/);
    });

    it("rejects an export missing name/slug", () => {
      expect(parseExport(JSON.stringify({ panels: [] })).ok).toBe(false);
    });
  });
});
