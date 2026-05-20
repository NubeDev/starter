import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  SlotName,
} from "../types.js";
import type { NodeKindRegistry } from "../nodes/NodeRegistry.js";

/** Internal node `data` shape consumed by every kind's renderer. */
export interface RFNodeData extends Record<string, unknown> {
  kindSpec?: NonNullable<ReturnType<NodeKindRegistry["get"]>>["spec"];
  label?: string;
  state?: NodeRunState;
  preview?: string;
  /**
   * Live per-slot values for this node, keyed by output-slot name.
   * Populated from `RunOverlay.slotValues`. Renderers display each
   * value as a small badge next to the matching slot handle.
   */
  slotValues?: Record<SlotName, unknown>;
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

  // RF-shape authoritative state. Holding this here (instead of
  // rebuilding from `graph` on every render) preserves the `measured`
  // dimensions that ReactFlow writes back via `dimensions` NodeChanges.
  // Without it, every parent re-render would strip `measured` and RF
  // would keep nodes at `visibility: hidden` forever.
  const buildRfNodes = useCallback(
    (g: FlowGraph): Node[] =>
      g.nodes.map((n) => {
        const entry = registry.get(n.kind);
        const state = overlay?.nodes[n.id];
        const slotValues = overlay?.slotValues?.[n.id];
        return {
          id: n.id,
          type: n.kind,
          position: n.position,
          data: {
            kindSpec: entry?.spec,
            label: n.label,
            state,
            slotValues,
            raw: n.data,
          } satisfies RFNodeData,
        };
      }),
    [registry, overlay],
  );

  const [rfNodes, setRfNodes] = useState<Node[]>(() => buildRfNodes(initial));

  // Track which node ids we've already seen so we can reconcile when
  // `graph.nodes` changes (add/remove) without nuking measurements on
  // existing nodes.
  const lastGraphRef = useRef<FlowGraph>(initial);
  useEffect(() => {
    if (graph === lastGraphRef.current) return;
    lastGraphRef.current = graph;
    setRfNodes((prev) => {
      const byId = new Map(prev.map((n) => [n.id, n]));
      return graph.nodes.map((n) => {
        const existing = byId.get(n.id);
        const entry = registry.get(n.kind);
        const state = overlay?.nodes[n.id];
        const slotValues = overlay?.slotValues?.[n.id];
        const data: RFNodeData = {
          kindSpec: entry?.spec,
          label: n.label,
          state,
          slotValues,
          raw: n.data,
        };
        if (!existing) {
          return { id: n.id, type: n.kind, position: n.position, data };
        }
        // Preserve `measured`, selection, dragging, etc.
        return { ...existing, type: n.kind, position: n.position, data };
      });
    });
  }, [graph, registry, overlay]);

  // Overlay-only reconciliation: when the run overlay ticks but the
  // graph hasn't changed, push new `state` / `slotValues` into each
  // node's `data` while preserving `measured`, selection, drag state,
  // and every other RF-internal field. Skipping this would leave
  // status colours and slot-value badges stuck on the first frame
  // until the next structural graph change.
  useEffect(() => {
    setRfNodes((prev) =>
      prev.map((rn) => {
        const prevData = (rn.data ?? {}) as RFNodeData;
        const state = overlay?.nodes[rn.id];
        const slotValues = overlay?.slotValues?.[rn.id];
        if (prevData.state === state && prevData.slotValues === slotValues) {
          return rn;
        }
        const data: RFNodeData = { ...prevData, state, slotValues };
        return { ...rn, data };
      }),
    );
  }, [overlay]);

  const rfNodesRef = useRef(rfNodes);
  useEffect(() => {
    rfNodesRef.current = rfNodes;
  }, [rfNodes]);

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
      const next = applyNodeChanges(changes, rfNodesRef.current);
      setRfNodes(next);

      // Only sync mutations that affect the persisted wire graph.
      // Skip `dimensions` / `select` / `dragging` so transient UI
      // state never round-trips through the parent's onChange.
      const persistent = changes.some(
        (c) =>
          c.type === "position" ||
          c.type === "add" ||
          c.type === "remove" ||
          c.type === "replace",
      );
      if (!persistent) return;

      const currentGraph = lastGraphRef.current;
      const updated: FlowGraph = {
        ...currentGraph,
        nodes: next.map((rn) => {
          const orig = currentGraph.nodes.find((n) => n.id === rn.id);
          if (!orig) {
            return {
              id: rn.id,
              kind: rn.type ?? "unknown",
              position: rn.position,
            } satisfies FlowNode;
          }
          return { ...orig, position: rn.position } satisfies FlowNode;
        }),
      };
      lastGraphRef.current = updated;
      setGraph(updated);
      onChange?.(updated);
    },
    [onChange],
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
