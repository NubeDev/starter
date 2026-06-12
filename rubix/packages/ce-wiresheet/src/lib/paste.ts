import type { Component } from "./engine-types";
import { rawFacet, remapFacetUids, FACET_PROP } from "./facet";

export interface PasteUpdate {
  uid: number;
  position?: { x: number; y: number };
  properties?: Record<string, { value: string }>;
}
export interface PastePlan {
  updates: PasteUpdate[]; // one bulkUpdate payload (positions + facet remap)
  newUids: number[]; // the TOP-LEVEL clones to select after paste
}

// Plan a paste from /copy/nodes output: flatten the (possibly nested) cloned
// subtree, translate the TOP-LEVEL clones so their bounding-box centre lands at
// the cursor, and remap uid references in any copied __facets (the engine copies
// the facet value verbatim, so it still points at the original uids — see
// API_REQUESTS §0a). Only top-level clones (placed directly under the dest) are
// repositioned and selected; descendants are off-canvas inside a pasted folder.
export function planPaste(
  clones: Component[],
  destParentUid: number,
  cursor: { x: number; y: number },
  uidMap?: { components?: Record<string, number>; properties?: Record<string, number> },
): PastePlan {
  const all: Component[] = [];
  const flatten = (c: Component) => {
    all.push(c);
    c.children?.forEach(flatten);
  };
  clones.forEach(flatten);

  const topLevel = all.filter((c) => c.parent === destParentUid);
  const xs = topLevel.map((c) => c.metadata?.position?.x ?? 0);
  const ys = topLevel.map((c) => c.metadata?.position?.y ?? 0);
  const dx = topLevel.length ? cursor.x - (Math.min(...xs) + Math.max(...xs)) / 2 : 0;
  const dy = topLevel.length ? cursor.y - (Math.min(...ys) + Math.max(...ys)) / 2 : 0;

  const compMap = uidMap?.components ?? {};
  const propMap = uidMap?.properties ?? {};
  const topSet = new Set(topLevel.map((c) => c.uid));
  const updates: PasteUpdate[] = [];
  for (const c of all) {
    const entry: PasteUpdate = { uid: c.uid };
    if (topSet.has(c.uid)) {
      entry.position = {
        x: Math.round((c.metadata?.position?.x ?? 0) + dx),
        y: Math.round((c.metadata?.position?.y ?? 0) + dy),
      };
    }
    if (uidMap) {
      const raw = rawFacet(c.properties);
      if (raw) {
        const remapped = remapFacetUids(raw, compMap, propMap);
        if (remapped !== raw) entry.properties = { [FACET_PROP]: { value: remapped } };
      }
    }
    if (entry.position || entry.properties) updates.push(entry);
  }
  return { updates, newUids: topLevel.map((c) => c.uid) };
}
