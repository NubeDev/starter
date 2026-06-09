import { useCallback, useMemo, useState } from "react";

import type { NodeType } from "@/api/types";
import type { FlowGraph, GraphNode } from "@/features/flows/builder/graph";

// The builder's editable graph state, kept outside React Flow so the graph is
// the source of truth (React Flow's nodes/edges are derived for rendering). A
// hook rather than a zustand store: a single builder instance is mounted at a
// time, so component-local state is enough and keeps the round-trip with the
// raw-JSON tab simple.

let idSeq = 0;
function freshId(kind: string): string {
  idSeq += 1;
  return `n${idSeq}-${kind}`;
}

export interface BuilderApi {
  graph: FlowGraph;
  selectedId: string | null;
  select: (id: string | null) => void;
  // Add a node from the palette at a canvas position.
  addNode: (type: NodeType, position: { x: number; y: number }) => void;
  // Replace the whole graph (template load, raw-JSON apply, open-for-edit).
  setGraph: (graph: FlowGraph) => void;
  removeNode: (id: string) => void;
  // Move a node (React Flow drag); persisted so layout survives a re-render.
  moveNode: (id: string, position: { x: number; y: number }) => void;
  // Connect two nodes; a self-edge or a duplicate is ignored.
  connect: (source: string, target: string) => void;
  removeEdge: (source: string, target: string) => void;
  // Update one node's config (the schema-driven form's onChange).
  setConfig: (id: string, config: Record<string, unknown>) => void;
}

const DEFAULT_POSITION = { x: 80, y: 80 };

export function useBuilderGraph(initial?: FlowGraph): BuilderApi {
  const [graph, setGraph] = useState<FlowGraph>(
    () => initial ?? { nodes: [], edges: [] },
  );
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const select = useCallback((id: string | null) => setSelectedId(id), []);

  const addNode = useCallback(
    (type: NodeType, position: { x: number; y: number }) => {
      const node: GraphNode = {
        id: freshId(type.kind),
        kind: type.kind,
        category: type.category,
        config: {},
        position,
      };
      setGraph((g) => ({ ...g, nodes: [...g.nodes, node] }));
      setSelectedId(node.id);
    },
    [],
  );

  const removeNode = useCallback((id: string) => {
    setGraph((g) => ({
      nodes: g.nodes.filter((n) => n.id !== id),
      edges: g.edges.filter((e) => e.source !== id && e.target !== id),
    }));
    setSelectedId((cur) => (cur === id ? null : cur));
  }, []);

  const moveNode = useCallback(
    (id: string, position: { x: number; y: number }) => {
      setGraph((g) => ({
        ...g,
        nodes: g.nodes.map((n) => (n.id === id ? { ...n, position } : n)),
      }));
    },
    [],
  );

  const connect = useCallback((source: string, target: string) => {
    if (source === target) return;
    setGraph((g) => {
      if (g.edges.some((e) => e.source === source && e.target === target)) {
        return g;
      }
      return { ...g, edges: [...g.edges, { source, target }] };
    });
  }, []);

  const removeEdge = useCallback((source: string, target: string) => {
    setGraph((g) => ({
      ...g,
      edges: g.edges.filter(
        (e) => !(e.source === source && e.target === target),
      ),
    }));
  }, []);

  const setConfig = useCallback(
    (id: string, config: Record<string, unknown>) => {
      setGraph((g) => ({
        ...g,
        nodes: g.nodes.map((n) => (n.id === id ? { ...n, config } : n)),
      }));
    },
    [],
  );

  const replace = useCallback((next: FlowGraph) => {
    // Give imported nodes a default position when none was provided so the
    // canvas can lay them out (parse builds a linear chain without layout).
    let i = 0;
    const nodes = next.nodes.map((n) => {
      const position = n.position ?? {
        x: DEFAULT_POSITION.x + i * 220,
        y: DEFAULT_POSITION.y,
      };
      i += 1;
      return { ...n, position };
    });
    setGraph({ nodes, edges: next.edges });
    setSelectedId(null);
  }, []);

  return useMemo(
    () => ({
      graph,
      selectedId,
      select,
      addNode,
      setGraph: replace,
      removeNode,
      moveNode,
      connect,
      removeEdge,
      setConfig,
    }),
    [
      graph,
      selectedId,
      select,
      addNode,
      replace,
      removeNode,
      moveNode,
      connect,
      removeEdge,
      setConfig,
    ],
  );
}
