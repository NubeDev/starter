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
        color: "var(--primary)",
      },
      style: {
        stroke: "var(--primary)",
        strokeWidth: 2,
      },
      label: `${rel.from_column} → ${rel.to_column}`,
      labelStyle: {
        fill: "var(--muted-foreground)",
        fontSize: 10,
      },
      labelBgStyle: {
        fill: "var(--background)",
      },
    }));

    return { initialNodes: nodes, initialEdges: edges };
  }, [data]);

  // Run dagre layout for the with-edges case; otherwise fall back
  // to a tidy grid so disconnected tables don't pile up at (0, 0).
  const { nodes: layoutedNodes, edges: layoutedEdges } = useMemo(() => {
    if (initialEdges.length > 0) {
      return layoutWithDagre(initialNodes, initialEdges);
    }
    return { nodes: gridLayoutNodes(initialNodes), edges: initialEdges };
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
          nodeStrokeColor="var(--primary)"
          nodeColor="var(--card)"
          nodeBorderRadius={4}
        />
      </ReactFlow>
    </div>
  );
}

/// Grid-pack nodes into N columns. Used when the dataset has no
/// edges and dagre would otherwise stack everything on x = 0.
function gridLayoutNodes(nodes: Node[]): Node[] {
  const NODE_WIDTH = 280;
  const HEADER_HEIGHT = 44;
  const COLUMN_HEIGHT = 28;
  const GAP_X = 60;
  const GAP_Y = 60;
  const cols = Math.max(1, Math.ceil(Math.sqrt(nodes.length)));
  return nodes.map((node, i) => {
    const col = i % cols;
    const row = Math.floor(i / cols);
    const columnCount = (node.data?.columns as unknown[] | undefined)?.length ?? 0;
    const _h = HEADER_HEIGHT + columnCount * COLUMN_HEIGHT;
    return {
      ...node,
      position: {
        x: col * (NODE_WIDTH + GAP_X),
        y: row * (HEADER_HEIGHT + 10 * COLUMN_HEIGHT + GAP_Y),
      },
    };
  });
}
