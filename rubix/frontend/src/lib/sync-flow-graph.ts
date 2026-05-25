// Project a `<FlowCanvas>` `FlowGraph` snapshot back onto the
// authoritative YAML body. Used by `/flows/$flowId` to persist
// canvas-side deletions and new edges via `flowDeploy`.
//
// The YAML body is the source of truth; the canvas owns ephemeral
// UI state (selection, drag positions) plus the right to mutate
// the wire shape. This helper takes the post-mutation `FlowGraph`,
// trims any nodes/links absent from it, appends any edges the
// canvas grew, and returns the new YAML string. Comments, ordering,
// and untouched node configs are preserved via
// `YAML.parseDocument`'s round-trip.

import * as YAML from "yaml";
import type { FlowGraph } from "@nube/starter-ui-flow";

/** Re-encode one canvas edge as a YAML link map (`{ from, to }`). */
function edgeToLink(
  edge: FlowGraph["edges"][number],
): { from: string; to: string } {
  return {
    from: `${edge.source}.${edge.sourceSlot ?? "out"}`,
    to: `${edge.target}.${edge.targetSlot ?? "in"}`,
  };
}

/**
 * Structurally diff `bodyYaml` against `graph` and return the new
 * YAML body. Returns `null` when nothing structural changed (so the
 * caller can skip a noop deploy).
 *
 * Detected structural changes:
 *   - Node removed (node id absent from `graph.nodes`).
 *   - Edge added/removed (the YAML `links` list is fully rebuilt
 *     from `graph.edges`).
 *
 * Node *additions* are NOT handled here — `<NodePalette>`'s
 * `appendFlowNode` is the canonical add path and runs `deploy`
 * itself. If a canvas-internal addition shows up here we still
 * preserve it (re-fetching from the deploy result), but xyflow
 * doesn't generate node-add events from the React side today.
 */
export function syncFlowGraph(bodyYaml: string, graph: FlowGraph): string | null {
  if (!bodyYaml) return null;
  let doc: YAML.Document.Parsed;
  try {
    doc = YAML.parseDocument(bodyYaml);
  } catch {
    return null;
  }

  let mutated = false;

  // --- 1. Trim nodes removed from the canvas, and write back
  //         the canvas-side `position` for survivors. ---
  const keepIds = new Set(graph.nodes.map((n) => n.id));
  const posById = new Map(graph.nodes.map((n) => [n.id, n.position] as const));
  const nodesSeq = doc.get("nodes", true);
  if (nodesSeq && YAML.isSeq(nodesSeq)) {
    for (let i = nodesSeq.items.length - 1; i >= 0; i -= 1) {
      const item = nodesSeq.items[i];
      if (!YAML.isMap(item)) continue;
      const id = item.get("id");
      if (typeof id !== "string") continue;
      if (!keepIds.has(id)) {
        nodesSeq.items.splice(i, 1);
        mutated = true;
        continue;
      }
      const pos = posById.get(id);
      if (!pos) continue;
      // Round to whole pixels — the canvas snaps to ints anyway,
      // and trimming sub-pixel noise keeps the YAML diff readable
      // and avoids spurious deploys from float drift.
      const nextX = Math.round(pos.x);
      const nextY = Math.round(pos.y);
      const existing = item.get("position");
      const prevX = YAML.isMap(existing)
        ? (existing.get("x") as number | undefined)
        : undefined;
      const prevY = YAML.isMap(existing)
        ? (existing.get("y") as number | undefined)
        : undefined;
      if (prevX === nextX && prevY === nextY) continue;
      const posNode = doc.createNode({ x: nextX, y: nextY }) as YAML.YAMLMap;
      posNode.flow = true;
      item.set("position", posNode);
      mutated = true;
    }
  }

  // --- 2. Rebuild the links list from the canvas edge set. ---
  const desiredLinks = graph.edges.map(edgeToLink);
  const existingLinks: Array<{ from: unknown; to: unknown }> = [];
  const linksSeq = doc.get("links", true);
  if (linksSeq && YAML.isSeq(linksSeq)) {
    for (const item of linksSeq.items) {
      if (YAML.isMap(item)) {
        existingLinks.push({ from: item.get("from"), to: item.get("to") });
      }
    }
  }
  const sameLinks =
    existingLinks.length === desiredLinks.length &&
    existingLinks.every(
      (l, i) => l.from === desiredLinks[i]!.from && l.to === desiredLinks[i]!.to,
    );
  if (!sameLinks) {
    // Replace whole `links:` field. Inline { from, to } per the
    // bundled YAML style so the diff stays readable.
    const linksNode = doc.createNode(desiredLinks) as YAML.YAMLSeq;
    for (const item of linksNode.items) {
      if (YAML.isMap(item)) item.flow = true;
    }
    doc.set("links", linksNode);
    mutated = true;
  }

  return mutated ? doc.toString() : null;
}
