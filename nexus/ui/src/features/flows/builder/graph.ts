import type { CreateFlowRequest, NodeCategory } from "@/api/types";

// The flow builder edits a node graph; the backend stores the ArkFlow
// `{input, pipeline:{processors[]}, output}` shape. This module is the bridge —
// it serialises the graph to that shape and parses an existing flow's config
// back into a graph, so the visual editor and the raw-JSON escape hatch
// round-trip without changing the on-the-wire contract.

// A node on the canvas: a chosen node type plus the config the form produced.
// `id` is canvas-local (React Flow needs stable ids); it is not serialised.
// `position` is the canvas layout (also not serialised) — optional so graphs
// built by `parseGraph`/templates without layout still satisfy the type.
export interface GraphNode {
  id: string;
  kind: string;
  category: NodeCategory;
  config: Record<string, unknown>;
  position?: { x: number; y: number };
}

// A directed connection between two canvas nodes.
export interface GraphEdge {
  source: string;
  target: string;
}

export interface FlowGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export type SerializeResult =
  | { ok: true; input: unknown; pipeline: unknown[]; output: unknown }
  | { ok: false; error: string };

// An ArkFlow component config is `{type, ...config}`. The `type` discriminant
// is the node kind; the rest of the object is the form's config.
function componentOf(node: GraphNode): Record<string, unknown> {
  return { type: node.kind, ...node.config };
}

// Order the processors by walking edges from the input to the output. A v1 flow
// is a single linear chain (one input → processors* → one output), so the walk
// is a simple successor follow; a node with no outgoing edge ends the chain.
function orderChain(graph: FlowGraph, startId: string): GraphNode[] {
  const byId = new Map(graph.nodes.map((n) => [n.id, n]));
  const next = new Map(graph.edges.map((e) => [e.source, e.target]));
  const chain: GraphNode[] = [];
  const seen = new Set<string>();
  let cursor: string | undefined = startId;
  while (cursor && !seen.has(cursor)) {
    seen.add(cursor);
    const node = byId.get(cursor);
    if (node) chain.push(node);
    cursor = next.get(cursor);
  }
  return chain;
}

// Serialise the graph to the ArkFlow config. Validates the v1 shape: exactly one
// input and one output, all processors reachable on the input→output chain. The
// returned pieces are the `{input, pipeline, output}` the create/update request
// carries.
export function serializeGraph(graph: FlowGraph): SerializeResult {
  const inputs = graph.nodes.filter((n) => n.category === "input");
  const outputs = graph.nodes.filter((n) => n.category === "output");
  if (inputs.length !== 1) {
    return { ok: false, error: "A flow needs exactly one input node." };
  }
  if (outputs.length !== 1) {
    return { ok: false, error: "A flow needs exactly one output node." };
  }

  const chain = orderChain(graph, inputs[0].id);
  const last = chain[chain.length - 1];
  if (!last || last.id !== outputs[0].id) {
    return { ok: false, error: "The input must connect through to the output." };
  }
  const processors = chain
    .filter((n) => n.category === "processor")
    .map(componentOf);

  return {
    ok: true,
    input: componentOf(inputs[0]),
    pipeline: processors,
    output: componentOf(outputs[0]),
  };
}

// Build the create-request body from a graph plus its name/enabled flags.
export type BuildResult =
  | { ok: true; value: CreateFlowRequest }
  | { ok: false; error: string };

export function toCreateFlow(
  graph: FlowGraph,
  name: string,
  enabled: boolean,
): BuildResult {
  const s = serializeGraph(graph);
  if (!s.ok) return { ok: false, error: s.error };
  return {
    ok: true,
    value: {
      name: name.trim(),
      enabled,
      input: s.input,
      pipeline: s.pipeline,
      output: s.output,
    },
  };
}
