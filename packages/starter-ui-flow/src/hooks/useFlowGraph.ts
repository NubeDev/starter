import { useCallback, useMemo, useState } from "react";
import type {
  Edge,
  Node,
  NodeChange,
  EdgeChange,
  Connection,
} from "@xyflow/react";
import { applyNodeChanges, applyEdgeChanges, addEdge } from "@xyflow/react";
import type {
  FlowEdge,
  FlowGraph,
  FlowNode,
  NodeRunState,
  RunOverlay,
} from "../types.js";
import type { NodeKindRegistry } from "../nodes/NodeRegistry.js";

/** Internal node `data` shape consumed by every kind's renderer. */
export interface RFNodeData extends Record<string, unknown> {
  kindSpec: ReturnType<NodeKindRegistry["get"]> extends infer T
    ? T extends { spec: infer S }
      ? S
      : never
    : never;
  label?: string;
  state?: NodeRunState;
  preview?: string;
  /** Original `data` bag from the wire `FlowNode`. */
  raw?: Record<string, unknown>;
}

interface UseFlowGraphArgs {
  /** Initial graph. Treated as the source of truth on first render. */
  initial: FlowGraph;
  registry: NodeKindRegistry;
  /** Optional live run state overlay. */
  overlay?: RunOverlay;
  /** Called whenever the graph changes. */
  onChange?: (graph: FlowGraph) => void;
}

/**
 * Bridges the wire shape (`FlowGraph`) to the @xyflow/react shape.
 * Owns local state, exposes change handlers, and merges in the live
 * `RunOverlay` so node renderers see the right `state` on each tick.
 */
export function useFlowGraph({
  initial,
  registry,
  overlay,
  onChange,
}: UseFlowGraphArgs) {
  const [graph, setGraph] = useState<FlowGraph>(initial);

  const update = useCallback(
    (next: FlowGraph) => {
      setGraph(next);
      onChange?.(next);
    },
    [onChange],
  );

  const rfNodes = useMemo<Node[]>(
    () =>
      graph.nodes.map((n) => {
        const entry = registry.get(n.kind);
        const state = overlay?.nodes[n.id];
        return {
          id: n.id,
          type: n.kind,
          position: n.position,
          data: {
            kindSpec: entry?.spec,
            label: n.label,
            state,
            raw: n.data,
          } satisfies RFNodeData,
        };
      }),
    [graph.nodes, registry, overlay],
  );

  const activeEdgeSet = useMemo(
    () => new Set(overlay?.activeEdges ?? []),
    [overlay],
  );

  const rfEdges = useMemo<Edge[]>(
    () =>
      graph.edges.map((e) => {
        const src = graph.nodes.find((n) => n.id === e.source);
        const slotKind = src
          ? registry
              .get(src.kind)
              ?.spec.outputs.find((s) => s.name === e.sourceSlot)?.kind
          : undefined;
        return {
          id: e.id,
          source: e.source,
          sourceHandle: e.sourceSlot,
          target: e.target,
          targetHandle: e.targetSlot,
          type: "typed",
          data: { slotKind: slotKind ?? "any", active: activeEdgeSet.has(e.id) },
        };
      }),
    [graph.edges, graph.nodes, registry, activeEdgeSet],
  );

  const onNodesChange = useCallback(
    (changes: NodeChange[]) => {
      const next = applyNodeChanges(changes, rfNodes);
      update({
        ...graph,
        nodes: next.map((rn) => {
          const orig = graph.nodes.find((n) => n.id === rn.id);
          if (!orig) {
            // Newly added — preserve type as kind, blank data.
            return {
              id: rn.id,
              kind: rn.type ?? "unknown",
              position: rn.position,
            } satisfies FlowNode;
          }
          return { ...orig, position: rn.position } satisfies FlowNode;
        }),
      });
    },
    [graph, rfNodes, update],
  );

  const onEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      const next = applyEdgeChanges(changes, rfEdges);
      update({
        ...graph,
        edges: next.map((re) => {
          const orig = graph.edges.find((e) => e.id === re.id);
          if (!orig) {
            return {
              id: re.id,
              source: re.source,
              sourceSlot: re.sourceHandle ?? "out",
              target: re.target,
              targetSlot: re.targetHandle ?? "in",
            } satisfies FlowEdge;
          }
          return orig;
        }),
      });
    },
    [graph, rfEdges, update],
  );

  const onConnect = useCallback(
    (connection: Connection) => {
      const merged = addEdge({ ...connection, type: "typed" }, rfEdges);
      const newOnes = merged.filter((m) => !rfEdges.find((e) => e.id === m.id));
      const additions: FlowEdge[] = newOnes.map((re) => ({
        id: re.id,
        source: re.source,
        sourceSlot: re.sourceHandle ?? "out",
        target: re.target,
        targetSlot: re.targetHandle ?? "in",
      }));
      update({ ...graph, edges: [...graph.edges, ...additions] });
    },
    [graph, rfEdges, update],
  );

  const addNode = useCallback(
    (node: FlowNode) => {
      update({ ...graph, nodes: [...graph.nodes, node] });
    },
    [graph, update],
  );

  const removeNode = useCallback(
    (id: string) => {
      update({
        nodes: graph.nodes.filter((n) => n.id !== id),
        edges: graph.edges.filter((e) => e.source !== id && e.target !== id),
      });
    },
    [graph, update],
  );

  return {
    graph,
    rfNodes,
    rfEdges,
    onNodesChange,
    onEdgesChange,
    onConnect,
    addNode,
    removeNode,
    setGraph: update,
  };
}
