import type { Component, Edge } from "./engine-types";
import { FACET_PROP } from "./facet";

// One in-group property to expose as a port on a new group folder.
export interface GroupBoundaryPort {
  childComponent: number;
  side: "input" | "output";
  label: string;
  facetProp?: number;
}

// Detect the boundary props when grouping `group` (a set of component uids):
// every edge with EXACTLY ONE endpoint inside the group crosses the new folder
// boundary, so the in-group end's prop must be exposed as a port (output if the
// source is inside, input if the target is). Keyed by the in-group prop uid.
//
// The prop uid falls back to a name lookup when the stored edge lacks
// source/targetPropertyUid (some POST responses omit it) — without that fallback
// a boundary edge would be dropped and render as a ghost instead of a port.
export function groupBoundary(
  group: Set<number>,
  edges: Iterable<Edge>,
  comps: Map<number, Component>,
): Map<number, GroupBoundaryPort> {
  const boundary = new Map<number, GroupBoundaryPort>();
  for (const e of edges) {
    const srcIn = group.has(e.sourceUid);
    const dstIn = group.has(e.targetUid);
    if (srcIn === dstIn) continue; // internal or fully-external
    if (srcIn) {
      const child = comps.get(e.sourceUid);
      const propUid = e.sourcePropertyUid ?? child?.properties[e.sourceProperty]?.uid;
      if (propUid != null) {
        boundary.set(propUid, {
          childComponent: e.sourceUid,
          side: "output",
          label: e.sourceProperty,
          facetProp: child?.properties[FACET_PROP]?.uid,
        });
      }
    } else {
      const child = comps.get(e.targetUid);
      const propUid = e.targetPropertyUid ?? child?.properties[e.targetProperty]?.uid;
      if (propUid != null) {
        boundary.set(propUid, {
          childComponent: e.targetUid,
          side: "input",
          label: e.targetProperty,
          facetProp: child?.properties[FACET_PROP]?.uid,
        });
      }
    }
  }
  return boundary;
}
