import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { NodeType } from "@/api/types";
import { useBuilderGraph } from "@/features/flows/builder/store";

const sim: NodeType = {
  kind: "simulator",
  category: "input",
  label: "Sim",
  description: "",
  config_schema: {},
};
const proc: NodeType = {
  kind: "json_to_arrow",
  category: "processor",
  label: "J2A",
  description: "",
  config_schema: {},
};

describe("useBuilderGraph", () => {
  it("adds a node and selects it", () => {
    const { result } = renderHook(() => useBuilderGraph());
    act(() => result.current.addNode(sim, { x: 1, y: 2 }));
    expect(result.current.graph.nodes).toHaveLength(1);
    const node = result.current.graph.nodes[0];
    expect(node.kind).toBe("simulator");
    expect(node.position).toEqual({ x: 1, y: 2 });
    expect(result.current.selectedId).toBe(node.id);
  });

  it("connects two nodes and ignores a duplicate or self-edge", () => {
    const { result } = renderHook(() => useBuilderGraph());
    act(() => result.current.addNode(sim, { x: 0, y: 0 }));
    act(() => result.current.addNode(proc, { x: 0, y: 0 }));
    const [a, b] = result.current.graph.nodes;
    act(() => result.current.connect(a.id, b.id));
    act(() => result.current.connect(a.id, b.id)); // duplicate
    act(() => result.current.connect(a.id, a.id)); // self
    expect(result.current.graph.edges).toEqual([{ source: a.id, target: b.id }]);
  });

  it("removing a node drops its edges and clears selection", () => {
    const { result } = renderHook(() => useBuilderGraph());
    act(() => result.current.addNode(sim, { x: 0, y: 0 }));
    act(() => result.current.addNode(proc, { x: 0, y: 0 }));
    const [a, b] = result.current.graph.nodes;
    act(() => result.current.connect(a.id, b.id));
    act(() => result.current.select(a.id));
    act(() => result.current.removeNode(a.id));
    expect(result.current.graph.nodes.map((n) => n.id)).toEqual([b.id]);
    expect(result.current.graph.edges).toHaveLength(0);
    expect(result.current.selectedId).toBeNull();
  });

  it("setGraph lays out imported nodes that lack a position", () => {
    const { result } = renderHook(() => useBuilderGraph());
    act(() =>
      result.current.setGraph({
        nodes: [
          { id: "x", kind: "simulator", category: "input", config: {} },
          { id: "y", kind: "json_to_arrow", category: "processor", config: {} },
        ],
        edges: [{ source: "x", target: "y" }],
      }),
    );
    const positions = result.current.graph.nodes.map((n) => n.position);
    expect(positions.every((p) => p !== undefined)).toBe(true);
    expect(positions[0]).not.toEqual(positions[1]);
  });

  it("setConfig replaces only the target node's config", () => {
    const { result } = renderHook(() => useBuilderGraph());
    act(() => result.current.addNode(sim, { x: 0, y: 0 }));
    const id = result.current.graph.nodes[0].id;
    act(() => result.current.setConfig(id, { profile: "hvac" }));
    expect(result.current.graph.nodes[0].config).toEqual({ profile: "hvac" });
  });
});
