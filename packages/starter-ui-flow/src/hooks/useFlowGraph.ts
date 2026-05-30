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
   * Live per-slot values for this node, keyed by slot name. Output
   * slots come straight from `RunOverlay.slotValues`; input slots are
   * derived by carrying the connected upstream output along its edge.
   * Renderers display each value as a small badge next to its handle.
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

  // Derive an "effective" per-node slot-value map that augments the
  // engine's overlay (which only emits *output* slot values) with
  // *input* slot values carried along each edge from the upstream
  // output it is wired to. A node's input slot has no value of its
  // own — it is, by definition, whatever the connected source output
  // last emitted — so we project `source.sourceSlot → target.targetSlot`.
  //
  // Output values always win on the source node; inputs and outputs use
  // disjoint slot names within a kind, so the carry never clobbers a
  // real emitted output. Recomputed only when the overlay ticks or the
  // edge set changes.
  const effectiveSlotValues = useMemo<Record<string, Record<SlotName, unknown>>>(() => {
    const base = overlay?.slotValues;
    const result: Record<string, Record<SlotName, unknown>> = {};
    if (base) {
      for (const [nodeId, slots] of Object.entries(base)) {
        result[nodeId] = { ...slots };
      }
    }
    for (const e of graph.edges) {
      const upstream = base?.[e.source]?.[e.sourceSlot];
      if (upstream === undefined) continue;
      (result[e.target] ??= {})[e.targetSlot] = upstream;
    }
    return result;
  }, [overlay, graph.edges]);

  // Sync `initial` prop changes into local state. Parents that own
  // the graph in a query cache (and recompute it via `useMemo` from
  // a server-side YAML body) need this — otherwise the first render
  // wins forever and the canvas drifts away from the source of
  // truth after every deploy. The check is referential because
  // callers should memoize the prop; if they don't, the cost is a
  // redundant `setGraph` that React batches away.
  useEffect(() => {
    setGraph((prev) => (prev === initial ? prev : initial));
  }, [initial]);

  const update = useCallback(
    (next: FlowGraph) => {
      // Keep the ref in sync inside the same event batch so other
      // handlers firing in the same tick (notably xyflow's cascading
      // edge removal that pairs with a node remove) operate on the
      // post-mutation state rather than the stale closure.
      lastGraphRef.current = next;
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
        const slotValues = effectiveSlotValues[n.id];
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
    [registry, overlay, effectiveSlotValues],
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
        const slotValues = effectiveSlotValues[n.id];
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
  }, [graph, registry, overlay, effectiveSlotValues]);

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
        const slotValues = effectiveSlotValues[rn.id];
        if (prevData.state === state && prevData.slotValues === slotValues) {
          return rn;
        }
        const data: RFNodeData = { ...prevData, state, slotValues };
        return { ...rn, data };
      }),
    );
  }, [overlay, effectiveSlotValues]);

  const rfNodesRef = useRef(rfNodes);
  useEffect(() => {
    rfNodesRef.current = rfNodes;
  }, [rfNodes]);

  const activeEdgeSet = useMemo(
    () => new Set(overlay?.activeEdges ?? []),
    [overlay],
  );

  const buildRfEdges = useCallback(
    (g: FlowGraph): Edge[] =>
      g.edges.map((e) => {
        const src = g.nodes.find((n) => n.id === e.source);
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
    [registry, activeEdgeSet],
  );

  // RF-shape authoritative edge state. Held in state (not `useMemo`)
  // so xyflow's selection — applied via `EdgeChange { type: "select",
  // selected: true }` and persisted on the edge object itself — is
  // preserved across renders. Without this, every change rebuilds
  // `rfEdges` from the wire graph and strips `selected`, which means
  // Backspace has nothing to delete because nothing is ever truly
  // selected from xyflow's point of view.
  const [rfEdges, setRfEdges] = useState<Edge[]>(() => buildRfEdges(initial));

  // Reconcile rfEdges with `graph.edges` (structural changes) while
  // preserving `selected` and other RF-internal flags on edges that
  // survive. Mirrors the rfNodes reconciliation.
  useEffect(() => {
    setRfEdges((prev) => {
      const byId = new Map(prev.map((e) => [e.id, e]));
      const fresh = buildRfEdges(graph);
      return fresh.map((re) => {
        const existing = byId.get(re.id);
        if (!existing) return re;
        return { ...existing, ...re, selected: existing.selected };
      });
    });
  }, [graph, buildRfEdges]);

  // Active-edge overlay updates: refresh `data.active` without
  // disturbing selection / structure.
  useEffect(() => {
    setRfEdges((prev) =>
      prev.map((re) => {
        const prevData = (re.data ?? {}) as { slotKind?: string; active?: boolean };
        const nextActive = activeEdgeSet.has(re.id);
        if (prevData.active === nextActive) return re;
        return { ...re, data: { ...prevData, active: nextActive } };
      }),
    );
  }, [activeEdgeSet]);

  const rfEdgesRef = useRef(rfEdges);
  useEffect(() => {
    rfEdgesRef.current = rfEdges;
  }, [rfEdges]);

  const onNodesChange = useCallback(
    (changes: NodeChange[]) => {
      const next = applyNodeChanges(changes, rfNodesRef.current);
      setRfNodes(next);

      // Only sync mutations that affect the persisted wire graph.
      // Skip `dimensions` / `select` so transient UI state never
      // round-trips through the parent's onChange. Position changes
      // are fired continuously while dragging (`dragging: true`)
      // and once more at drag-end (`dragging: false`); only the
      // latter is persisted — otherwise every drag would spam the
      // backend with a deploy per frame.
      const persistent = changes.some(
        (c) =>
          (c.type === "position" && c.dragging === false) ||
          c.type === "add" ||
          c.type === "remove" ||
          c.type === "replace",
      );
      if (!persistent) return;

      const currentGraph = lastGraphRef.current;
      const keepIds = new Set(next.map((rn) => rn.id));
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
        // Cascade-prune edges that reference a deleted node. Without
        // this, xyflow's separate `onEdgesChange` cascade can race
        // with this handler and the deploy round-trip ends up with
        // dangling links the backend validator will reject.
        edges: currentGraph.edges.filter(
          (e) => keepIds.has(e.source) && keepIds.has(e.target),
        ),
      };
      update(updated);
    },
    [update],
  );

  const onEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      const next = applyEdgeChanges(changes, rfEdgesRef.current);
      setRfEdges(next);

      // Mirror onNodesChange's filter: `select` / `dimensions` are
      // transient UI state and must not round-trip through `onChange`
      // (which would spam the backend with a deploy every time the
      // operator clicks an edge). Only structural mutations get
      // propagated to the wire graph.
      const persistent = changes.some(
        (c) => c.type === "add" || c.type === "remove" || c.type === "replace",
      );
      if (!persistent) return;

      const currentGraph = lastGraphRef.current;
      update({
        ...currentGraph,
        edges: next.map((re) => {
          const orig = currentGraph.edges.find((e) => e.id === re.id);
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
    [update],
  );

  const onConnect = useCallback(
    (connection: Connection) => {
      const currentGraph = lastGraphRef.current;
      const merged = addEdge({ ...connection, type: "typed" }, rfEdgesRef.current);
      const newOnes = merged.filter(
        (m) => !rfEdgesRef.current.find((e) => e.id === m.id),
      );
      if (newOnes.length === 0) return;
      setRfEdges(merged);
      const additions: FlowEdge[] = newOnes.map((re) => ({
        id: re.id,
        source: re.source,
        sourceSlot: re.sourceHandle ?? "out",
        target: re.target,
        targetSlot: re.targetHandle ?? "in",
      }));
      update({ ...currentGraph, edges: [...currentGraph.edges, ...additions] });
    },
    [update],
  );

  const addNode = useCallback(
    (node: FlowNode) => {
      const currentGraph = lastGraphRef.current;
      update({ ...currentGraph, nodes: [...currentGraph.nodes, node] });
    },
    [update],
  );

  const removeNode = useCallback(
    (id: string) => {
      const currentGraph = lastGraphRef.current;
      update({
        nodes: currentGraph.nodes.filter((n) => n.id !== id),
        edges: currentGraph.edges.filter((e) => e.source !== id && e.target !== id),
      });
    },
    [update],
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
