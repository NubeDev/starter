import { useCallback, useMemo } from "react";
import {
  Background,
  Controls,
  Handle,
  Position,
  ReactFlow,
  type Connection,
  type Edge,
  type Node,
  type NodeProps,
  type NodeTypes,
  type OnConnect,
  type OnEdgesChange,
  type OnNodesChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import type { NodeCategory } from "@/api/types";
import type { BuilderApi } from "@/features/flows/builder/store";

// The graph canvas. React Flow renders nodes/edges derived from the builder
// graph; user edits (drag/connect/select/delete) are applied back through the
// builder api so the graph stays the source of truth. Edges are constrained by
// category in `onConnect` — a v1 flow is input → processor* → output, so an
// edge whose direction breaks that chain is refused.

type NodeData = {
  label: string;
  kind: string;
  category: NodeCategory;
};

// Whether an edge from `source` to `target` keeps the input→processor*→output
// ordering. Inputs only emit; outputs only receive; processors are in between.
function edgeAllowed(source: NodeCategory, target: NodeCategory): boolean {
  if (target === "input") return false; // nothing feeds an input
  if (source === "output") return false; // an output is terminal
  return true;
}

const CATEGORY_TINT: Record<NodeCategory, string> = {
  input: "var(--chart-1)",
  processor: "var(--chart-2)",
  output: "var(--chart-3)",
};

function GraphNodeCard({ data, selected }: NodeProps) {
  const d = data as NodeData;
  const tint = CATEGORY_TINT[d.category];
  return (
    <div
      className="glass min-w-36 rounded-lg px-3 py-2"
      style={{
        borderColor: selected ? tint : undefined,
        boxShadow: selected ? `0 0 0 1px ${tint}` : undefined,
      }}
    >
      {d.category !== "input" ? (
        <Handle type="target" position={Position.Left} />
      ) : null}
      <p className="text-[10px] font-semibold uppercase tracking-wide" style={{ color: tint }}>
        {d.category}
      </p>
      <p className="truncate text-sm font-medium text-foreground">{d.label}</p>
      <p className="truncate text-[11px] text-muted-foreground">{d.kind}</p>
      {d.category !== "output" ? (
        <Handle type="source" position={Position.Right} />
      ) : null}
    </div>
  );
}

const NODE_TYPES: NodeTypes = { graphNode: GraphNodeCard };

export function Canvas({ builder }: { builder: BuilderApi }) {
  const { graph } = builder;

  const categoryById = useMemo(() => {
    const m = new Map<string, NodeCategory>();
    for (const n of graph.nodes) m.set(n.id, n.category);
    return m;
  }, [graph.nodes]);

  const rfNodes = useMemo<Node[]>(
    () =>
      graph.nodes.map((n) => ({
        id: n.id,
        type: "graphNode",
        position: n.position ?? { x: 0, y: 0 },
        selected: n.id === builder.selectedId,
        data: { label: labelFor(n.kind), kind: n.kind, category: n.category },
      })),
    [graph.nodes, builder.selectedId],
  );

  const rfEdges = useMemo<Edge[]>(
    () =>
      graph.edges.map((e) => ({
        id: `${e.source}->${e.target}`,
        source: e.source,
        target: e.target,
      })),
    [graph.edges],
  );

  const onNodesChange = useCallback<OnNodesChange>(
    (changes) => {
      for (const c of changes) {
        if (c.type === "position" && c.position) {
          builder.moveNode(c.id, c.position);
        } else if (c.type === "remove") {
          builder.removeNode(c.id);
        } else if (c.type === "select") {
          if (c.selected) builder.select(c.id);
        }
      }
    },
    [builder],
  );

  const onEdgesChange = useCallback<OnEdgesChange>(
    (changes) => {
      for (const c of changes) {
        if (c.type === "remove") {
          const [source, target] = c.id.split("->");
          if (source && target) builder.removeEdge(source, target);
        }
      }
    },
    [builder],
  );

  const onConnect = useCallback<OnConnect>(
    (conn: Connection) => {
      if (!conn.source || !conn.target) return;
      const s = categoryById.get(conn.source);
      const t = categoryById.get(conn.target);
      if (!s || !t || !edgeAllowed(s, t)) return;
      builder.connect(conn.source, conn.target);
    },
    [builder, categoryById],
  );

  return (
    <div className="size-full">
      <ReactFlow
        nodes={rfNodes}
        edges={rfEdges}
        nodeTypes={NODE_TYPES}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onPaneClick={() => builder.select(null)}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Background />
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
  );
}

// A node's display label is its kind title-cased; the palette's richer label
// is not carried on the graph node (the graph stores only what serialises plus
// canvas layout), so derive a readable label from the kind here.
function labelFor(kind: string): string {
  return kind
    .split("_")
    .map((p) => (p ? p[0].toUpperCase() + p.slice(1) : p))
    .join(" ");
}
