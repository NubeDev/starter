// Forked from sql-studio (https://github.com/frectonz/sql-studio) — MIT.
// Upstream commit: 1a0736055a4647c18d0be19347e4325007c7bd52.
// Local edits: re-skinned to rubix tokens; data layer swapped to @nube/rubix-client-react.

import dagre from "dagre";
import { type Node, type Edge, Position } from "@xyflow/react";

const NODE_WIDTH = 280;
const COLUMN_HEIGHT = 28;
const HEADER_HEIGHT = 44;
const GROUP_GAP_X = 80;

function nodeHeight(node: Node): number {
  const columnCount = (node.data?.columns as unknown[] | undefined)?.length ?? 0;
  return HEADER_HEIGHT + columnCount * COLUMN_HEIGHT;
}

/// Extract the "extension" prefix for a table name.
///
/// Convention: extension tables use `<namespace>__<table>` (double
/// underscore), e.g. `com_rubix_example__customers`. Tables without
/// `__` are treated as core; we still bucket them by their first
/// two underscore-separated segments so `starter_auth_users_*` and
/// `starter_changes` end up in different lanes.
function groupKey(name: string): string {
  const dd = name.indexOf("__");
  if (dd > 0) return name.slice(0, dd);
  const parts = name.split("_");
  if (parts.length >= 3) return parts.slice(0, 3).join("_");
  if (parts.length >= 2) return parts.slice(0, 2).join("_");
  return parts[0] ?? name;
}

function dagreLayout(
  nodes: Node[],
  edges: Edge[],
): { nodes: Node[]; width: number; height: number } {
  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  // TB: stack tables vertically inside each extension column.
  g.setGraph({ rankdir: "TB", nodesep: 24, ranksep: 40 });

  nodes.forEach((node) => {
    g.setNode(node.id, { width: NODE_WIDTH, height: nodeHeight(node) });
  });
  edges.forEach((edge) => {
    if (g.hasNode(edge.source) && g.hasNode(edge.target)) {
      g.setEdge(edge.source, edge.target);
    }
  });

  dagre.layout(g);

  let maxX = 0;
  let maxY = 0;
  const positioned = nodes.map((node) => {
    const p = g.node(node.id);
    const h = nodeHeight(node);
    const x = p.x - NODE_WIDTH / 2;
    const y = p.y - h / 2;
    if (x + NODE_WIDTH > maxX) maxX = x + NODE_WIDTH;
    if (y + h > maxY) maxY = y + h;
    return {
      ...node,
      position: { x, y },
      targetPosition: Position.Left,
      sourcePosition: Position.Right,
    };
  });

  // Normalize to (0, 0) origin so groups can be offset cleanly.
  let minX = Infinity;
  let minY = Infinity;
  positioned.forEach((n) => {
    if (n.position.x < minX) minX = n.position.x;
    if (n.position.y < minY) minY = n.position.y;
  });
  if (!isFinite(minX)) minX = 0;
  if (!isFinite(minY)) minY = 0;
  const normalized = positioned.map((n) => ({
    ...n,
    position: { x: n.position.x - minX, y: n.position.y - minY },
  }));

  return {
    nodes: normalized,
    width: Math.max(0, maxX - minX),
    height: Math.max(0, maxY - minY),
  };
}

/// Lay out all tables, grouping by extension prefix so each
/// extension forms its own horizontal swimlane. Within a group
/// we run dagre on edges whose endpoints both fall inside the
/// group; cross-group edges are kept on the canvas but ignored
/// by the layout (they route across lanes).
export function layoutWithDagre(
  nodes: Node[],
  edges: Edge[],
): { nodes: Node[]; edges: Edge[] } {
  // Bucket by group key, preserving first-seen order.
  const order: string[] = [];
  const groups = new Map<string, Node[]>();
  for (const node of nodes) {
    const key = groupKey(node.id);
    if (!groups.has(key)) {
      groups.set(key, []);
      order.push(key);
    }
    groups.get(key)!.push(node);
  }

  // Group edges that live entirely inside one group.
  const groupOf = new Map<string, string>();
  nodes.forEach((n) => groupOf.set(n.id, groupKey(n.id)));
  const internalEdges = new Map<string, Edge[]>();
  for (const edge of edges) {
    const gs = groupOf.get(edge.source);
    const gt = groupOf.get(edge.target);
    if (gs && gs === gt) {
      const list = internalEdges.get(gs) ?? [];
      list.push(edge);
      internalEdges.set(gs, list);
    }
  }

  // Lay each group out, then arrange groups side-by-side —
  // one extension per column — and prepend a heading node above
  // each column so the grouping is visually obvious.
  const laidGroups = order.map((key) => {
    const gNodes = groups.get(key) ?? [];
    const gEdges = internalEdges.get(key) ?? [];
    return { key, ...dagreLayout(gNodes, gEdges) };
  });

  const HEADER_NODE_HEIGHT = 48;
  let xCursor = 0;
  const positioned: Node[] = [];
  laidGroups.forEach((grp) => {
    // Heading node — rendered as `groupHeader` node type.
    positioned.push({
      id: `__group_header_${grp.key}`,
      type: "groupHeader",
      position: { x: xCursor, y: -HEADER_NODE_HEIGHT - 16 },
      data: { label: prettyGroupLabel(grp.key), count: grp.nodes.length },
      draggable: false,
      selectable: false,
      width: Math.max(NODE_WIDTH, grp.width),
      style: { width: Math.max(NODE_WIDTH, grp.width) },
    } as Node);

    grp.nodes.forEach((n) => {
      positioned.push({
        ...n,
        position: { x: n.position.x + xCursor, y: n.position.y },
      });
    });
    xCursor += Math.max(NODE_WIDTH, grp.width) + GROUP_GAP_X;
  });

  return { nodes: positioned, edges };
}

/// Turn a raw group key into a friendly column heading.
///
/// `com_rubix_example` → `com.rubix.example`
/// `starter_auth_users` → `starter.auth.users`
function prettyGroupLabel(key: string): string {
  return key.replace(/_/g, ".");
}
