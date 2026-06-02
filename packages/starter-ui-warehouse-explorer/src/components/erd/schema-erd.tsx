// ERD canvas for the new Schema Explorer page.
//
// Reuses the group-aware dagre layout from `use-layout.ts` (shared with
// the legacy view) but renders the polished `SchemaNode` and a custom
// floating toolbar in place of react-flow's default Controls/MiniMap.
//
// Selection is controlled from the parent so the left tree and the
// canvas stay in sync: clicking a tree row focuses + selects the node,
// clicking a node selects it (and the tree highlights it).

import { useEffect, useImperativeHandle, useMemo } from "react";
import {
  Background,
  BackgroundVariant,
  MarkerType,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type ColorMode,
  type Edge,
  type Node,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Maximize2, Minus, Plus, Scan } from "lucide-react";

import { useTheme } from "../../lib/theme";
import { cn } from "../../lib/utils";
import { layoutWithDagre } from "./use-layout";
import { SchemaNode, type SchemaColumn } from "./schema-node";

type ErdColumn = {
  name: string;
  data_type: string;
  nullable: boolean;
  is_primary_key: boolean;
};

type ErdTable = { name: string; columns: ErdColumn[] };

type ErdRelationship = {
  from_table: string;
  from_column: string;
  to_table: string;
  to_column: string;
};

type ErdData = {
  tables: ErdTable[];
  relationships: ErdRelationship[];
};

const nodeTypes = { schemaNode: SchemaNode };

/** Imperative handle the parent uses to drive the canvas from the tree. */
export type SchemaErdHandle = {
  focusNode: (name: string) => void;
  fitView: () => void;
};

type Props = {
  data: ErdData;
  selectedNode: string | null;
  onSelect: (name: string | null) => void;
  handleRef?: React.Ref<SchemaErdHandle>;
};

export function SchemaErd(props: Props) {
  return (
    <ReactFlowProvider>
      <SchemaErdInner {...props} />
    </ReactFlowProvider>
  );
}

function SchemaErdInner({ data, selectedNode, onSelect, handleRef }: Props) {
  const theme = useTheme();
  const rf = useReactFlow();

  const { nodes, edges } = useMemo(() => {
    // Columns that take part in any relationship → render a handle dot.
    // FK columns (the "from" side) get a link glyph.
    const connected = new Map<string, Set<string>>();
    const foreignKeys = new Map<string, Set<string>>();
    const touch = (map: Map<string, Set<string>>, table: string, col: string) => {
      if (!map.has(table)) map.set(table, new Set());
      map.get(table)!.add(col);
    };
    for (const rel of data.relationships) {
      touch(connected, rel.from_table, rel.from_column);
      touch(connected, rel.to_table, rel.to_column);
      touch(foreignKeys, rel.from_table, rel.from_column);
    }

    const baseNodes: Node[] = data.tables.map((table) => {
      const fk = foreignKeys.get(table.name);
      const columns: SchemaColumn[] = table.columns.map((c) => ({
        ...c,
        is_foreign_key: fk?.has(c.name) ?? false,
      }));
      return {
        id: table.name,
        type: "schemaNode",
        position: { x: 0, y: 0 },
        data: {
          label: table.name,
          columns,
          kind: "table" as const,
          connected: connected.get(table.name) ?? new Set<string>(),
        },
      };
    });

    const baseEdges: Edge[] = data.relationships.map((rel, i) => ({
      id: `edge-${i}`,
      source: rel.from_table,
      target: rel.to_table,
      sourceHandle: rel.from_column,
      targetHandle: rel.to_column,
      type: "smoothstep",
      markerEnd: { type: MarkerType.ArrowClosed, width: 14, height: 14 },
    }));

    const laid = layoutWithDagre(baseNodes, baseEdges);
    return { nodes: laid.nodes, edges: laid.edges };
  }, [data]);

  // Apply selection styling without rerunning layout.
  const styledNodes = useMemo<Node[]>(
    () =>
      nodes.map((n) =>
        n.type === "schemaNode"
          ? { ...n, selected: n.id === selectedNode }
          : n,
      ),
    [nodes, selectedNode],
  );

  const styledEdges = useMemo<Edge[]>(
    () =>
      edges.map((e) => {
        const active =
          selectedNode != null &&
          (e.source === selectedNode || e.target === selectedNode);
        return {
          ...e,
          animated: active,
          style: {
            stroke: active ? "hsl(var(--primary))" : "hsl(var(--border))",
            strokeWidth: active ? 2 : 1.5,
          },
          markerEnd: {
            type: MarkerType.ArrowClosed,
            width: 14,
            height: 14,
            color: active ? "hsl(var(--primary))" : "hsl(var(--border))",
          },
        };
      }),
    [edges, selectedNode],
  );

  useImperativeHandle(
    handleRef,
    () => ({
      focusNode: (name: string) => {
        const node = rf.getNode(name);
        if (node) {
          rf.fitView({ nodes: [{ id: name }], duration: 400, maxZoom: 1.1, padding: 0.4 });
        }
      },
      fitView: () => rf.fitView({ duration: 400, padding: 0.2 }),
    }),
    [rf],
  );

  // Center on the selected node when it changes from the tree.
  useEffect(() => {
    if (selectedNode && rf.getNode(selectedNode)) {
      rf.fitView({ nodes: [{ id: selectedNode }], duration: 400, maxZoom: 1.1, padding: 0.4 });
    }
  }, [selectedNode, rf]);

  return (
    <div className="relative h-full w-full">
      <ReactFlow
        nodes={styledNodes}
        edges={styledEdges}
        nodeTypes={nodeTypes}
        colorMode={theme as ColorMode}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.1}
        maxZoom={2}
        proOptions={{ hideAttribution: true }}
        onNodeClick={(_, node) =>
          node.type === "schemaNode" ? onSelect(node.id) : undefined
        }
        onPaneClick={() => onSelect(null)}
        defaultEdgeOptions={{ type: "smoothstep" }}
      >
        <Background variant={BackgroundVariant.Dots} gap={18} size={1} />
      </ReactFlow>

      <CanvasToolbar />
    </div>
  );
}

/** Floating zoom / fit toolbar, top-right, matching the reference. */
function CanvasToolbar() {
  const rf = useReactFlow();
  const btn =
    "flex h-8 w-8 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground";

  return (
    <div className="absolute right-3 top-3 z-10 flex overflow-hidden rounded-lg border border-border bg-card/95 shadow-sm backdrop-blur">
      <button type="button" className={btn} title="Zoom out" onClick={() => rf.zoomOut({ duration: 200 })}>
        <Minus className="h-4 w-4" />
      </button>
      <button type="button" className={cn(btn, "border-l border-border")} title="Zoom in" onClick={() => rf.zoomIn({ duration: 200 })}>
        <Plus className="h-4 w-4" />
      </button>
      <button type="button" className={cn(btn, "border-l border-border")} title="Fit view" onClick={() => rf.fitView({ duration: 400, padding: 0.2 })}>
        <Scan className="h-4 w-4" />
      </button>
      <button type="button" className={cn(btn, "border-l border-border")} title="Reset zoom" onClick={() => rf.zoomTo(1, { duration: 200 })}>
        <Maximize2 className="h-4 w-4" />
      </button>
    </div>
  );
}
