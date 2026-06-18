import type { Component, Edge } from "./engine-types";
import { FACET_PROP, exposedPorts, facetFor, rawFacet } from "./facet";

// One in-group property to expose as a port on a new group folder.
export interface GroupBoundaryPort {
  childComponent: number;
  side: "input" | "output";
  label: string;
  facetProp?: number;
}

// A CHAINED boundary port: when a grouped member is itself a folder that already
// exposes a port, and an edge through that port crosses the NEW group's boundary,
// the new folder re-projects the inner folder's port (chain) rather than
// re-exposing the deep child (which would double-expose it). Keyed by the same
// deep prop uid so the per-view routing index resolves it.
export interface ChainedBoundaryPort {
  prop: number; // the deep prop uid (key on the new folder's facet)
  innerFolder: number; // the grouped folder that currently exposes `prop`
  side: "input" | "output";
  label: string;
  facetProp?: number; // the inner folder's __facets prop uid (live label for the chained port)
}

// Detect chained boundary ports when grouping a set that includes folders with
// exposed ports. For each grouped folder's exposed port, an edge through it
// crosses the new boundary unless its OTHER end is also inside the group (a
// selected member, or another grouped folder's exposed port → internal).
export function groupChainedBoundary(
  group: Set<number>,
  groupedFolders: Component[],
  edges: Iterable<Edge>,
): Map<number, ChainedBoundaryPort> {
  // deep prop uid → the grouped folder exposing it (+ its record)
  const exposedByGroup = new Map<number, { folder: Component; side: "input" | "output"; label: string }>();
  for (const f of groupedFolders) {
    for (const ep of exposedPorts(facetFor(f.uid, rawFacet(f.properties)))) {
      exposedByGroup.set(ep.childUid, { folder: f, side: ep.side, label: ep.facet.label ?? "" });
    }
  }
  const groupedExposedProps = new Set(exposedByGroup.keys());

  const out = new Map<number, ChainedBoundaryPort>();
  for (const e of edges) {
    const sP = e.sourcePropertyUid;
    const tP = e.targetPropertyUid;
    let prop: number | undefined;
    let otherProp: number | undefined;
    let otherUid: number | undefined;
    if (sP != null && groupedExposedProps.has(sP)) { prop = sP; otherProp = tP; otherUid = e.targetUid; }
    else if (tP != null && groupedExposedProps.has(tP)) { prop = tP; otherProp = sP; otherUid = e.sourceUid; }
    if (prop == null) continue;
    // internal → no port: the other end is a selected member, or itself routes
    // through another grouped folder's exposed port.
    const otherInGroup =
      (otherUid != null && group.has(otherUid)) || (otherProp != null && groupedExposedProps.has(otherProp));
    if (otherInGroup) continue;
    const g = exposedByGroup.get(prop)!;
    out.set(prop, {
      prop,
      innerFolder: g.folder.uid,
      side: g.side,
      label: g.label,
      facetProp: g.folder.properties[FACET_PROP]?.uid,
    });
  }
  return out;
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
