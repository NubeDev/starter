// Forked from sql-studio (https://github.com/frectonz/sql-studio) — MIT.
// Upstream commit: 1a0736055a4647c18d0be19347e4325007c7bd52.
// Local edits: re-skinned to rubix tokens; data layer swapped to @nube/rubix-client-react.

import { useMemo } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  type Node,
  type Edge,
  type ColorMode,
  MarkerType,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { useTheme } from "../../lib/theme";
import { TableNode } from "./table-node";
import { GroupHeaderNode } from "./group-header-node";
import { layoutWithDagre } from "./use-layout";

type Column = {
  name: string;
  data_type: string;
  nullable: boolean;
  is_primary_key: boolean;
};

type Table = {
  name: string;
  columns: Column[];
};

type Relationship = {
  from_table: string;
  from_column: string;
  to_table: string;
  to_column: string;
};

type ErdData = {
  tables: Table[];
  relationships: Relationship[];
};

const nodeTypes = {
  tableNode: TableNode,
  groupHeader: GroupHeaderNode,
};

type Props = {
  data: ErdData;
};

export function ErdDiagram({ data }: Props) {
  const theme = useTheme();

  const { initialNodes, initialEdges } = useMemo(() => {
    const nodes: Node[] = data.tables.map((table) => ({
      id: table.name,
      type: "tableNode",
      position: { x: 0, y: 0 },
      data: {
        label: table.name,
        columns: table.columns,
      },
    }));

    const edges: Edge[] = data.relationships.map((rel, index) => ({
      id: `edge-${index}`,
      source: rel.from_table,
      target: rel.to_table,
      sourceHandle: rel.from_column,
      targetHandle: rel.to_column,
      type: "smoothstep",
      animated: true,
      markerEnd: {
        type: MarkerType.ArrowClosed,
        color: "hsl(var(--primary))",
      },
      style: {
        stroke: "hsl(var(--primary))",
        strokeWidth: 2,
      },
      label: `${rel.from_column} → ${rel.to_column}`,
      labelStyle: {
        fill: "hsl(var(--muted-foreground))",
        fontSize: 10,
      },
      labelBgStyle: {
        fill: "hsl(var(--background))",
      },
      labelBgPadding: [4, 2],
      labelBgBorderRadius: 2,
    }));

    return { initialNodes: nodes, initialEdges: edges };
  }, [data]);

  // Group-aware dagre layout. `layoutWithDagre` buckets tables by
  // extension prefix and lays each group out on its own swimlane,
  // so disconnected tables no longer pile into a single column.
  const { nodes: layoutedNodes, edges: layoutedEdges } = useMemo(() => {
    return layoutWithDagre(initialNodes, initialEdges);
  }, [initialNodes, initialEdges]);

  return (
    <div className="w-full h-[calc(100vh-8rem)] min-h-[480px] rounded-lg border border-border overflow-hidden">
      <ReactFlow
        nodes={layoutedNodes}
        edges={layoutedEdges}
        nodeTypes={nodeTypes}
        colorMode={theme as ColorMode}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.1}
        maxZoom={2}
        defaultEdgeOptions={{
          type: "smoothstep",
        }}
      >
        <Background gap={16} size={1} />
        <Controls />
        <MiniMap
          nodeStrokeColor="hsl(var(--primary))"
          nodeColor="hsl(var(--card))"
          nodeBorderRadius={4}
        />
      </ReactFlow>
    </div>
  );
}
