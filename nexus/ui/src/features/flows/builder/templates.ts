import type { FlowGraph, GraphNode } from "@/features/flows/builder/graph";

// Starter flow graphs the editor offers as a one-click starting point. Each is
// a complete, valid v1 chain (input → processor* → output) over the currently
// registered nodes, so a new flow begins from a working shape the user then
// tweaks rather than a blank canvas.

export interface FlowTemplate {
  id: string;
  label: string;
  description: string;
  build: () => FlowGraph;
}

// Connect a list of nodes into a linear chain.
function chain(nodes: GraphNode[]): FlowGraph {
  const edges = nodes
    .slice(0, -1)
    .map((n, i) => ({ source: n.id, target: nodes[i + 1].id }));
  return { nodes, edges };
}

export const FLOW_TEMPLATES: FlowTemplate[] = [
  {
    id: "simulator-postgres",
    label: "Simulator → Postgres",
    description: "Emit synthetic HVAC telemetry, shape it, land it in a Postgres table.",
    build: () =>
      chain([
        {
          id: "sim",
          kind: "simulator",
          category: "input",
          config: { profile: "hvac", interval: "5s", device_id: "sim-1" },
        },
        {
          id: "to-arrow",
          kind: "json_to_arrow",
          category: "processor",
          config: {},
        },
        {
          id: "pg",
          kind: "postgres",
          category: "output",
          config: { uri: "", table: "" },
        },
      ]),
  },
  {
    id: "http-sse",
    label: "HTTP poll → Live (SSE)",
    description: "Poll a JSON endpoint on an interval and fan each response out to live subscribers.",
    build: () =>
      chain([
        {
          id: "http",
          kind: "http_poll",
          category: "input",
          config: { url: "", interval: "15m" },
        },
        {
          id: "sse",
          kind: "sse",
          category: "output",
          config: { run_id: "" },
        },
      ]),
  },
];
