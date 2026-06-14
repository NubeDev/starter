import { describe, expect, it } from "vitest";

import { serializeGraph, toCreateFlow, type FlowGraph } from "@/features/flows/builder/graph";
import { parseGraph } from "@/features/flows/builder/parse";
import { FLOW_TEMPLATES } from "@/features/flows/builder/templates";
import type { NodeType } from "@/api/types";

const palette: NodeType[] = [
  { kind: "simulator", category: "input", label: "Sim", description: "", config_schema: {} },
  { kind: "http_poll", category: "input", label: "HTTP", description: "", config_schema: {} },
  { kind: "json_to_arrow", category: "processor", label: "J2A", description: "", config_schema: {} },
  { kind: "sql", category: "processor", label: "SQL", description: "", config_schema: {} },
  { kind: "postgres", category: "output", label: "PG", description: "", config_schema: {} },
  { kind: "sse", category: "output", label: "SSE", description: "", config_schema: {} },
];

function linear(): FlowGraph {
  return {
    nodes: [
      { id: "i", kind: "simulator", category: "input", config: { profile: "hvac" } },
      { id: "p", kind: "json_to_arrow", category: "processor", config: {} },
      { id: "o", kind: "postgres", category: "output", config: { uri: "u", table: "t" } },
    ],
    edges: [
      { source: "i", target: "p" },
      { source: "p", target: "o" },
    ],
  };
}

describe("serializeGraph", () => {
  it("serialises a linear chain to {input, pipeline, output}", () => {
    const r = serializeGraph(linear());
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.input).toEqual({ type: "simulator", profile: "hvac" });
    expect(r.pipeline).toEqual([{ type: "json_to_arrow" }]);
    expect(r.output).toEqual({ type: "postgres", uri: "u", table: "t" });
  });

  it("rejects a graph with no input", () => {
    const g = linear();
    g.nodes = g.nodes.filter((n) => n.category !== "input");
    const r = serializeGraph(g);
    expect(r.ok).toBe(false);
  });

  it("rejects two outputs", () => {
    const g = linear();
    g.nodes.push({ id: "o2", kind: "sse", category: "output", config: {} });
    const r = serializeGraph(g);
    expect(r.ok).toBe(false);
  });

  it("rejects an input not connected through to the output", () => {
    const g = linear();
    g.edges = [{ source: "i", target: "p" }]; // p never reaches o
    const r = serializeGraph(g);
    expect(r.ok).toBe(false);
  });
});

describe("toCreateFlow", () => {
  it("builds a create request, trimming the name", () => {
    const r = toCreateFlow(linear(), "  my flow  ", true);
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.value.name).toBe("my flow");
    expect(r.value.enabled).toBe(true);
  });
});

describe("graph round-trip", () => {
  it("serialise → parse → serialise is stable", () => {
    const first = serializeGraph(linear());
    expect(first.ok).toBe(true);
    if (!first.ok) return;
    const graph = parseGraph(first.input, first.pipeline, first.output, palette);
    const second = serializeGraph(graph);
    expect(second.ok).toBe(true);
    if (!second.ok) return;
    expect(second.input).toEqual(first.input);
    expect(second.pipeline).toEqual(first.pipeline);
    expect(second.output).toEqual(first.output);
  });

  it("parses a pipeline given as {processors:[...]}", () => {
    const graph = parseGraph(
      { type: "http_poll", url: "x", interval: "1m" },
      { processors: [{ type: "sql", query: "select 1" }] },
      { type: "sse", run_id: "" },
      palette,
    );
    expect(graph.nodes.map((n) => n.category)).toEqual(["input", "processor", "output"]);
    expect(graph.edges).toHaveLength(2);
  });
});

describe("templates", () => {
  it("every template serialises to a valid flow", () => {
    for (const t of FLOW_TEMPLATES) {
      const r = serializeGraph(t.build());
      expect(r.ok, `${t.id} should be a valid flow`).toBe(true);
    }
  });
});
