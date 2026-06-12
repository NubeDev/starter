import { useMemo } from "react";
import {
  Background,
  Controls,
  MarkerType,
  ReactFlow,
  type Edge,
  type Node,
  type NodeTypes,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import type { DatasourceSchema } from "@/api/types";
import {
  buildModel,
  gridPositions,
} from "@/features/query-editor/SchemaDiagram/layout";
import { TableNode } from "@/features/query-editor/SchemaDiagram/TableNode";

const NODE_TYPES: NodeTypes = { table: TableNode };

// A read-only ER diagram of a schema: one card per table (columns listed, FK
// columns marked), one edge per foreign key. Pan/zoom via React Flow; the
// layout is a deterministic grid (see `gridPositions`) since the schema carries
// no positions and the table count is small enough not to need a layout engine.
//
// Relationships are *real* foreign keys read from `information_schema`, so a
// schema with no FKs (common for telemetry/sim datasources) shows tables with
// no edges — that's accurate, not a failure.
export function SchemaDiagram({ schema }: { schema: DatasourceSchema }) {
  const { nodes, edges } = useMemo(() => {
    const model = buildModel(schema);
    const pos = gridPositions(model.nodes, model.edges);

    const rfNodes: Node[] = model.nodes.map((n) => ({
      id: n.key,
      type: "table",
      position: pos.get(n.key) ?? { x: 0, y: 0 },
      data: n as unknown as Record<string, unknown>,
    }));
    const rfEdges: Edge[] = model.edges.map((e) => ({
      id: e.id,
      source: e.from,
      target: e.to,
      label: e.label,
      labelStyle: { fontSize: 10, fill: "var(--muted-foreground)" },
      labelBgStyle: { fill: "var(--card)", fillOpacity: 0.85 },
      style: { stroke: "var(--primary)", strokeWidth: 1.5 },
      markerEnd: { type: MarkerType.ArrowClosed, color: "var(--primary)" },
    }));
    return { nodes: rfNodes, edges: rfEdges };
  }, [schema]);

  if (nodes.length === 0) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-sm text-muted-foreground">
        No tables to diagram.
      </div>
    );
  }

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={NODE_TYPES}
      fitView
      minZoom={0.1}
      proOptions={{ hideAttribution: true }}
      nodesDraggable
      nodesConnectable={false}
      elementsSelectable
    >
      <Background />
      <Controls showInteractive={false} />
    </ReactFlow>
  );
}
