import { useMemo } from "react";
import {
  Background,
  Controls,
  Handle,
  Position,
  ReactFlow,
  type Edge,
  type Node,
  type NodeProps,
  type NodeTypes,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import type { NodeCategory } from "@/api/types";
import type { FlowGraph } from "@/features/flows/builder/graph";
import type { NodeDebug } from "@/features/flows/useFlowDebug";

// A read-only twin of the builder `Canvas`: it renders a saved flow's parsed
// graph and overlays live debug counters on each node (rows in/out + batches),
// with the selected node ringed. Clicking a node drives the sample-rows panel.
// No editing — drag/connect/delete are all disabled — so the canvas can never
// drift from the live run it is showing.

type DebugNodeData = {
  label: string;
  kind: string;
  category: NodeCategory;
  nodeIndex: number;
  debug?: NodeDebug;
};

const CATEGORY_TINT: Record<NodeCategory, string> = {
  input: "var(--chart-1)",
  processor: "var(--chart-2)",
  output: "var(--chart-3)",
};

function DebugNodeCard({ data, selected }: NodeProps) {
  const d = data as DebugNodeData;
  const tint = CATEGORY_TINT[d.category];
  const c = d.debug?.counters;
  return (
    <div
      className="glass min-w-40 rounded-lg px-3 py-2"
      style={{
        borderColor: selected ? tint : undefined,
        boxShadow: selected ? `0 0 0 1px ${tint}` : undefined,
      }}
    >
      {d.category !== "input" ? (
        <Handle type="target" position={Position.Left} />
      ) : null}
      <p
        className="text-[10px] font-semibold uppercase tracking-wide"
        style={{ color: tint }}
      >
        {d.category}
      </p>
      <p className="truncate text-sm font-medium text-foreground">{d.label}</p>
      <p className="truncate text-[11px] text-muted-foreground">{d.kind}</p>
      {/* Live counters: rows out (what the node produced) and a small in→out
          delta so a filter/fan-out reads at a glance. */}
      <div className="mt-1.5 flex items-center gap-2 text-[11px]">
        <span
          className="rounded px-1.5 py-0.5 font-mono"
          style={{ backgroundColor: `${tint}26`, color: tint }}
          title="rows out"
        >
          {c ? formatCount(c.rows_out) : "—"} rows
        </span>
        {c && c.rows_in !== c.rows_out ? (
          <span
            className="font-mono text-muted-foreground"
            title="rows in → rows out"
          >
            {formatCount(c.rows_in)}→{formatCount(c.rows_out)}
          </span>
        ) : null}
      </div>
      {d.category !== "output" ? (
        <Handle type="source" position={Position.Right} />
      ) : null}
    </div>
  );
}

const NODE_TYPES: NodeTypes = { debugNode: DebugNodeCard };

export function DebugCanvas({
  graph,
  byNode,
  selectedIndex,
  onSelect,
}: {
  graph: FlowGraph;
  byNode: Map<number, NodeDebug>;
  selectedIndex: number | null;
  onSelect: (nodeIndex: number) => void;
}) {
  // The parsed graph lists nodes in chain order (source, processors…, sink),
  // which is exactly the backend's node-index order, so the node's position in
  // `graph.nodes` is its debug node index.
  const rfNodes = useMemo<Node[]>(
    () =>
      graph.nodes.map((n, i) => ({
        id: n.id,
        type: "debugNode",
        position: n.position ?? { x: i * 220, y: 0 },
        selected: i === selectedIndex,
        draggable: false,
        connectable: false,
        deletable: false,
        data: {
          label: labelFor(n.kind),
          kind: n.kind,
          category: n.category,
          nodeIndex: i,
          debug: byNode.get(i),
        } satisfies DebugNodeData,
      })),
    [graph.nodes, byNode, selectedIndex],
  );

  const rfEdges = useMemo<Edge[]>(
    () =>
      graph.edges.map((e) => ({
        id: `${e.source}->${e.target}`,
        source: e.source,
        target: e.target,
        animated: true,
      })),
    [graph.edges],
  );

  return (
    <div className="size-full">
      <ReactFlow
        nodes={rfNodes}
        edges={rfEdges}
        nodeTypes={NODE_TYPES}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable
        onNodeClick={(_, node) => {
          const i = (node.data as DebugNodeData).nodeIndex;
          onSelect(i);
        }}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Background />
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
  );
}

function labelFor(kind: string): string {
  return kind
    .split("_")
    .map((p) => (p ? p[0].toUpperCase() + p.slice(1) : p))
    .join(" ");
}

// Compact large counts (1234 → 1.2k) so a node badge stays readable.
function formatCount(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}k`;
  return `${(n / 1_000_000).toFixed(1)}M`;
}
