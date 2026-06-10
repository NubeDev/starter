import type { NodeCategory, NodeType } from "@/api/types";

import type { FlowGraph, GraphEdge, GraphNode } from "@/features/flows/builder/graph";

// Parse an existing flow's `{input, pipeline, output}` config back into a graph
// so a saved flow opens in the visual editor. The inverse of
// `serializeGraph` — together they round-trip a flow without changing its
// stored shape.

// Split an ArkFlow `{type, ...config}` component into its kind and config. A
// component with no `type` is rejected; an unknown `type` is kept (the editor
// still shows it, just without a schema-driven form).
function splitComponent(
  value: unknown,
): { kind: string; config: Record<string, unknown> } | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return null;
  }
  const obj = value as Record<string, unknown>;
  const kind = obj.type;
  if (typeof kind !== "string") return null;
  const { type: _type, ...config } = obj;
  return { kind, config };
}

// Look up a node's category from the palette; default by position when a kind
// is not in the registry (input slot → input, output slot → output).
function categoryOf(
  kind: string,
  fallback: NodeCategory,
  palette: NodeType[],
): NodeCategory {
  return palette.find((n) => n.kind === kind)?.category ?? fallback;
}

let counter = 0;
function freshId(prefix: string): string {
  counter += 1;
  return `${prefix}-${counter}`;
}

// The pipeline field is either an array of processors or an object with a
// `processors` array (ArkFlow accepts both); normalise to the array.
function processorsOf(pipeline: unknown): unknown[] {
  if (Array.isArray(pipeline)) return pipeline;
  if (typeof pipeline === "object" && pipeline !== null) {
    const p = (pipeline as Record<string, unknown>).processors;
    if (Array.isArray(p)) return p;
  }
  return [];
}

// Build the graph from a flow's three config blobs. Lays out a single linear
// chain input → processors → output with edges between consecutive nodes.
export function parseGraph(
  input: unknown,
  pipeline: unknown,
  output: unknown,
  palette: NodeType[],
): FlowGraph {
  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];

  const addNode = (
    value: unknown,
    fallback: NodeCategory,
  ): GraphNode | null => {
    const split = splitComponent(value);
    if (!split) return null;
    const node: GraphNode = {
      id: freshId(split.kind),
      kind: split.kind,
      category: categoryOf(split.kind, fallback, palette),
      config: split.config,
    };
    nodes.push(node);
    return node;
  };

  const inputNode = addNode(input, "input");
  const processorNodes = processorsOf(pipeline)
    .map((p) => addNode(p, "processor"))
    .filter((n): n is GraphNode => n !== null);
  const outputNode = addNode(output, "output");

  const chain = [inputNode, ...processorNodes, outputNode].filter(
    (n): n is GraphNode => n !== null,
  );
  for (let i = 0; i < chain.length - 1; i += 1) {
    edges.push({ source: chain[i].id, target: chain[i + 1].id });
  }

  return { nodes, edges };
}
