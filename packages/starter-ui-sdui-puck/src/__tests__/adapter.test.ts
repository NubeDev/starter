// Adapter round-trip tests — every IR node we send through the
// builder must come back unchanged after Puck Data → IR conversion,
// so saves are stable for operators who don't touch the canvas.

import { describe, expect, it } from "vitest";

import {
  componentTreeToPuckData,
  puckDataToComponentTree,
  IR_VERSION,
  type ComponentTree,
} from "../adapter.js";

function roundtrip(tree: ComponentTree): ComponentTree {
  return puckDataToComponentTree(componentTreeToPuckData(tree));
}

describe("componentTreeToPuckData / puckDataToComponentTree", () => {
  it("round-trips an empty page", () => {
    const tree: ComponentTree = {
      ir_version: IR_VERSION,
      root: { type: "page" },
    };
    expect(roundtrip(tree)).toEqual({
      ir_version: IR_VERSION,
      root: { type: "page", id: "root", children: [] },
    });
  });

  it("preserves root props (title, tags)", () => {
    const tree: ComponentTree = {
      ir_version: IR_VERSION,
      root: {
        type: "page",
        id: "root",
        title: "Data flow — Site A",
        tags: ["site-a", "ops"],
        children: [],
      },
    };
    expect(roundtrip(tree)).toEqual(tree);
  });

  it("round-trips a row with kpi children (slot recursion)", () => {
    const tree: ComponentTree = {
      ir_version: IR_VERSION,
      root: {
        type: "page",
        id: "root",
        children: [
          {
            type: "row",
            children: [
              { type: "kpi", label: "Throughput", value: 12 },
              { type: "kpi", label: "Errors", value: 0 },
            ],
          },
        ],
      },
    };
    expect(roundtrip(tree)).toEqual(tree);
  });

  it("keeps author-supplied ids and drops synthesised ones", () => {
    const tree: ComponentTree = {
      ir_version: IR_VERSION,
      root: {
        type: "page",
        id: "root",
        children: [
          { type: "kpi", id: "throughput-kpi", label: "Throughput" },
          { type: "kpi", label: "Errors" },
        ],
      },
    };
    const back = roundtrip(tree);
    const children = back.root.children ?? [];
    expect(children[0]?.id).toBe("throughput-kpi");
    expect(children[1]?.id).toBeUndefined();
  });

  it("does not recurse into non-slot arrays (chart.sources stays opaque)", () => {
    const tree: ComponentTree = {
      ir_version: IR_VERSION,
      root: {
        type: "page",
        id: "root",
        children: [
          {
            type: "chart",
            sources: [
              { name: "series-a", points: [{ ts: 0, v: 1 }] },
            ],
          },
        ],
      },
    };
    expect(roundtrip(tree)).toEqual(tree);
  });

  it("emits the current IR_VERSION on the way out", () => {
    const data = componentTreeToPuckData({
      ir_version: 0, // deliberately stale
      root: { type: "page" },
    });
    const back = puckDataToComponentTree(data);
    expect(back.ir_version).toBe(IR_VERSION);
  });

  it("wraps a non-page root in a synthetic page rather than throwing", () => {
    const tree: ComponentTree = {
      ir_version: IR_VERSION,
      root: { type: "kpi", label: "Lonely" } as never,
    };
    const data = componentTreeToPuckData(tree);
    expect(data.content).toHaveLength(1);
    expect((data.content[0] as { type: string }).type).toBe("kpi");
  });

  it("adds a fallback id of 'root' when the stored page has no id", () => {
    const tree: ComponentTree = {
      ir_version: IR_VERSION,
      root: { type: "page", children: [] },
    };
    const back = roundtrip(tree);
    expect(back.root.id).toBe("root");
  });
});
