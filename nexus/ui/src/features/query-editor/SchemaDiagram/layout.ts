import type { DatasourceSchema, SchemaTable } from "@/api/types";

// A schema-qualified key — the stable id for a table node and the join key for
// resolving FK endpoints. `schema.name` even for `public` so two tables of the
// same name in different schemas never collide.
export function tableKey(schema: string, name: string): string {
  return `${schema}.${name}`;
}

export type DiagramNode = {
  key: string;
  schema: string;
  name: string;
  table: SchemaTable;
  /** Columns that participate in a FK (either end) — highlighted in the card. */
  fkColumns: Set<string>;
};

export type DiagramEdge = {
  id: string;
  from: string;
  to: string;
  label: string;
};

// Build the node/edge model from a schema. Only FK edges whose *both* endpoints
// are present as nodes are kept — a FK to a filtered-out system table would
// otherwise dangle. Each edge is labelled with the referencing column so the
// diagram reads as "orders.customer_id → customers".
export function buildModel(schema: DatasourceSchema): {
  nodes: DiagramNode[];
  edges: DiagramEdge[];
} {
  const nodes: DiagramNode[] = (schema.tables ?? []).map((t) => ({
    key: tableKey(t.schema, t.name),
    schema: t.schema,
    name: t.name,
    table: t,
    fkColumns: new Set<string>(),
  }));
  const byKey = new Map(nodes.map((n) => [n.key, n]));

  const edges: DiagramEdge[] = [];
  const seen = new Set<string>();
  for (const r of schema.relations ?? []) {
    const from = tableKey(r.from_schema, r.from_table);
    const to = tableKey(r.to_schema, r.to_table);
    const fromNode = byKey.get(from);
    const toNode = byKey.get(to);
    if (!fromNode || !toNode) continue;
    fromNode.fkColumns.add(r.from_column);
    toNode.fkColumns.add(r.to_column);
    // Collapse multiple FK columns between the same pair into one drawn edge,
    // but keep the per-column label readable for the common single-column case.
    const id = `${from}:${r.from_column}->${to}:${r.to_column}`;
    if (seen.has(id)) continue;
    seen.add(id);
    edges.push({
      id,
      from,
      to,
      label: `${r.from_column} → ${r.to_column}`,
    });
  }
  return { nodes, edges };
}

// Deterministic grid layout. The schema carries no positions and we don't pull
// in a layout engine (dagre/elk) for what is usually a few dozen tables, so we
// place nodes left-to-right, top-to-bottom in a grid sized to the node count.
// Tables most connected by FKs sort first so related tables tend to land near
// each other; ties break alphabetically for a stable, reproducible layout.
export function gridPositions(
  nodes: DiagramNode[],
  edges: DiagramEdge[],
): Map<string, { x: number; y: number }> {
  const degree = new Map<string, number>();
  for (const n of nodes) degree.set(n.key, 0);
  for (const e of edges) {
    degree.set(e.from, (degree.get(e.from) ?? 0) + 1);
    degree.set(e.to, (degree.get(e.to) ?? 0) + 1);
  }
  const ordered = [...nodes].sort(
    (a, b) =>
      (degree.get(b.key) ?? 0) - (degree.get(a.key) ?? 0) ||
      a.key.localeCompare(b.key),
  );

  const COL_W = 320;
  const ROW_H = 260;
  const cols = Math.max(1, Math.ceil(Math.sqrt(ordered.length)));
  const pos = new Map<string, { x: number; y: number }>();
  ordered.forEach((n, i) => {
    pos.set(n.key, {
      x: (i % cols) * COL_W,
      y: Math.floor(i / cols) * ROW_H,
    });
  });
  return pos;
}
