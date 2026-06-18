import type { Component, Edge } from "./engine-types";
import { exposedPorts, facetFor, rawFacet } from "./facet";

// Cross-folder edge routing — the pure decisions behind reload()'s edge handling.
// (Ghost geometry / RF node+edge construction stay in the editor; this module
// only decides what connects to what.)

export interface EdgePartition {
  inEdges: Edge[]; // both endpoints visible in this view → drawn normally
  crossEdges: Edge[]; // exactly one endpoint visible → ghost or exposed port
}

// Split a view's edges. Edges with BOTH endpoints off-canvas are dropped (not in
// either bucket) — neither end is in this view.
export function partitionEdges(edges: Iterable<Edge>, childUids: Set<number>): EdgePartition {
  const inEdges: Edge[] = [];
  const crossEdges: Edge[] = [];
  for (const e of edges) {
    const src = childUids.has(e.sourceUid);
    const dst = childUids.has(e.targetUid);
    if (src && dst) inEdges.push(e);
    else if (src !== dst) crossEdges.push(e);
  }
  return { inEdges, crossEdges };
}

export interface ExposedPortIndex {
  // child-prop uid → the visible component (folder) projecting it as a port
  index: Map<number, { parentUid: number }>;
  // child-prop uid → the child COMPONENT that actually owns the prop (for
  // retargeting edges/overrides; the port is only drawn on the folder)
  remap: Map<number, number>;
  // prop uids to property-subscribe so off-canvas ports stream: the port's own
  // value AND the child's __facets (live label/unit/aliases)
  subProps: Set<number>;
}

// Index the exposed ports of the components visible in this view.
export function exposedPortIndex(children: Component[]): ExposedPortIndex {
  const index = new Map<number, { parentUid: number }>();
  const remap = new Map<number, number>();
  const subProps = new Set<number>();
  for (const child of children) {
    for (const ep of exposedPorts(facetFor(child.uid, rawFacet(child.properties)))) {
      index.set(ep.childUid, { parentUid: child.uid });
      // CHAINED port: childComponent is the inner folder (next chain link), not
      // the real prop owner — don't retarget new edges through it here (resolving
      // the chain needs the inner folder loaded; follow-up). Index + subscriptions
      // still apply, so EXISTING edges render as a port and value/label stream.
      if (ep.facet.childComponent != null && !ep.facet.chain) {
        remap.set(ep.childUid, ep.facet.childComponent);
      }
      subProps.add(ep.childUid);
      if (ep.facet.facetProp != null) subProps.add(ep.facet.facetProp);
    }
  }
  return { index, remap, subProps };
}

// How a single cross-folder edge should render.
export type CrossEdgeRoute =
  | {
      kind: "port"; // the off-canvas end is an exposed port → draw a normal edge to its handle
      edgeUid: number;
      loopBack: boolean;
      externalIsTarget: boolean; // the VISIBLE end is the edge's source
      visibleUid: number;
      visiblePropUid: number;
      portParentUid: number; // the folder the port is drawn on
      portHandle: number; // the child-prop uid (handle id on the folder)
    }
  | {
      kind: "ghost"; // the off-canvas end is a plain component → draw a ghost stub
      edgeUid: number;
      loopBack: boolean;
      externalIsTarget: boolean;
      side: "input" | "output"; // ghost sits on the input or output side of the visible node
      visibleUid: number;
      visiblePropName: string;
      externalUid: number;
      externalPropName: string;
      externalPath: string;
    };

// Decide how a cross edge routes. Precondition: exactly one endpoint is visible
// (i.e. `e` came from partitionEdges().crossEdges). It routes to an exposed PORT
// when the off-canvas end's prop is one this view projects AND the visible end's
// prop uid is known; otherwise it's a GHOST.
export function classifyCrossEdge(
  e: Edge,
  childUids: Set<number>,
  index: Map<number, { parentUid: number }>,
): CrossEdgeRoute {
  const externalIsTarget = childUids.has(e.sourceUid); // visible end is the source
  const externalPropUid = externalIsTarget ? e.targetPropertyUid : e.sourcePropertyUid;
  const exposed = externalPropUid != null ? index.get(externalPropUid) : undefined;
  if (exposed) {
    const visibleUid = externalIsTarget ? e.sourceUid : e.targetUid;
    const visiblePropUid = externalIsTarget ? e.sourcePropertyUid : e.targetPropertyUid;
    if (visiblePropUid != null) {
      return {
        kind: "port",
        edgeUid: e.uid,
        loopBack: e.loopBack === true,
        externalIsTarget,
        visibleUid,
        visiblePropUid,
        portParentUid: exposed.parentUid,
        portHandle: externalPropUid as number,
      };
    }
  }
  return {
    kind: "ghost",
    edgeUid: e.uid,
    loopBack: e.loopBack === true,
    externalIsTarget,
    side: externalIsTarget ? "input" : "output",
    visibleUid: externalIsTarget ? e.sourceUid : e.targetUid,
    visiblePropName: externalIsTarget ? e.sourceProperty : e.targetProperty,
    externalUid: externalIsTarget ? e.targetUid : e.sourceUid,
    externalPropName: externalIsTarget ? e.targetProperty : e.sourceProperty,
    externalPath: (externalIsTarget ? e.targetPath : e.sourcePath) ?? "",
  };
}
