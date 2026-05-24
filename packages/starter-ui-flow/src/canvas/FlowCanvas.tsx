import { useMemo, type CSSProperties, type ReactNode } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  ReactFlowProvider,
  type ReactFlowProps,
} from "@xyflow/react";
import type { FlowGraph, RunOverlay } from "../types.js";
import type { NodeKindRegistry } from "../nodes/NodeRegistry.js";
import { DEFAULT_EDGE_TYPES } from "../edges/index.js";
import { useFlowGraph } from "../hooks/useFlowGraph.js";
import { useTypedConnect } from "../hooks/useTypedConnect.js";
import { FlowI18nProvider } from "../i18n/context.js";
import type { FlowMessages } from "../i18n/messages.js";

export interface FlowCanvasProps {
  registry: NodeKindRegistry;
  graph: FlowGraph;
  /** Optional live state overlay (running/ok/error per node). */
  overlay?: RunOverlay;
  /** Fires on every graph mutation (node move, connect, delete, …). */
  onChange?: (graph: FlowGraph) => void;
  /** Read-only mode disables editing affordances. */
  readOnly?: boolean;
  showMiniMap?: boolean;
  showControls?: boolean;
  showBackground?: boolean;
  /** Extra @xyflow/react props for full escape-hatch control. */
  reactFlowProps?: Partial<ReactFlowProps>;
  /** Localized strings. Partial override merged on top of
   * `DEFAULT_FLOW_MESSAGES`. Built-in node kind labels and slot
   * labels can be translated via `i18n.kindLabels` /
   * `i18n.slotLabels`. */
  i18n?: Partial<FlowMessages>;
  /** Slot rendered inside the canvas (e.g. palette overlay). */
  children?: ReactNode;
  style?: CSSProperties;
  className?: string;
}

/**
 * Drop-in `<FlowCanvas>` for rendering a `FlowGraph`. Wraps
 * @xyflow/react, plugs in `nodeTypes` from the supplied
 * `NodeKindRegistry`, and validates connections by slot kind.
 *
 * Caller-owned graph state via `onChange`. This component holds
 * UI-only state (selection, viewport) — never persists.
 */
export function FlowCanvas({
  registry,
  graph,
  overlay,
  onChange,
  readOnly = false,
  showMiniMap = true,
  showControls = true,
  showBackground = true,
  reactFlowProps,
  i18n,
  children,
  style,
  className,
}: FlowCanvasProps) {
  const nodeTypes = useMemo(() => registry.toNodeTypes(), [registry]);

  const flow = useFlowGraph({ initial: graph, registry, overlay, onChange });
  const isValidConnection = useTypedConnect({ registry, nodes: flow.graph.nodes });

  return (
    <FlowI18nProvider value={i18n}>
      <ReactFlowProvider>
        <div
          className={className}
          style={{ width: "100%", height: "100%", minHeight: 320, ...style }}
        >
          <ReactFlow
            nodes={flow.rfNodes}
            edges={flow.rfEdges}
            nodeTypes={nodeTypes}
            edgeTypes={DEFAULT_EDGE_TYPES}
            onNodesChange={readOnly ? undefined : flow.onNodesChange}
            onEdgesChange={readOnly ? undefined : flow.onEdgesChange}
            onConnect={readOnly ? undefined : flow.onConnect}
            isValidConnection={isValidConnection}
            nodesDraggable={!readOnly}
            nodesConnectable={!readOnly}
            elementsSelectable
            fitView
            {...reactFlowProps}
          >
            {showBackground ? <Background gap={16} size={1} /> : null}
            {showControls ? <Controls /> : null}
            {showMiniMap ? <MiniMap pannable zoomable /> : null}
            {children}
          </ReactFlow>
        </div>
      </ReactFlowProvider>
    </FlowI18nProvider>
  );
}
