import { describe, expect, it } from "vitest";
import { partitionEdges, exposedPortIndex, classifyCrossEdge } from "./routing";
import { serializeFacet, FACET_PROP } from "./facet";
import type { Component, Edge, Property } from "./engine-types";

function prop(uid: number, componentUid: number): Property {
  return { uid, componentUid, category: 0, value: 0, statusFlags: 0 };
}
function comp(uid: number, props: Record<string, number>, facet?: string): Component {
  const properties: Record<string, Property> = {};
  for (const [n, p] of Object.entries(props)) properties[n] = prop(p, uid);
  if (facet != null) {
    properties[FACET_PROP] = { ...prop(900 + uid, uid), value: facet, systemRole: 2 };
  }
  return { uid, name: `c${uid}`, type: "math::add", path: `root/c${uid}`, parent: 0, properties };
}
function edge(p: Partial<Edge> & { uid: number; sourceUid: number; targetUid: number }): Edge {
  return {
    sourceProperty: "out",
    targetProperty: "in",
    ...p,
  } as Edge;
}

describe("partitionEdges", () => {
  const childUids = new Set([1, 2]);
  it("splits into in-view, cross, and dropped", () => {
    const { inEdges, crossEdges } = partitionEdges(
      [
        edge({ uid: 100, sourceUid: 1, targetUid: 2 }), // both visible → in
        edge({ uid: 101, sourceUid: 1, targetUid: 9 }), // one visible → cross
        edge({ uid: 102, sourceUid: 8, targetUid: 2 }), // one visible → cross
        edge({ uid: 103, sourceUid: 7, targetUid: 9 }), // none visible → dropped
      ],
      childUids,
    );
    expect(inEdges.map((e) => e.uid)).toEqual([100]);
    expect(crossEdges.map((e) => e.uid)).toEqual([101, 102]);
  });
});

describe("exposedPortIndex", () => {
  it("indexes ports, child remap, and the prop-subscription set", () => {
    // folder c1 exposes child prop 500 (owned by component 50, child __facets 60)
    const facet = serializeFacet(
      new Map([[500, { expose: "input", childComponent: 50, facetProp: 60 }]]),
    );
    const { index, remap, subProps } = exposedPortIndex([comp(1, {}, facet), comp(2, { in: 21 })]);
    expect(index.get(500)).toEqual({ parentUid: 1 });
    expect(remap.get(500)).toBe(50);
    // subscribe both the port value (500) and the child's live __facets (60)
    expect([...subProps].sort((a, b) => a - b)).toEqual([60, 500]);
  });

  it("indexes + subscribes a CHAINED port but skips remap (chain resolved elsewhere)", () => {
    // folder c1 re-projects an inner folder (99)'s already-exposed port 500
    const facet = serializeFacet(
      new Map([[500, { expose: "input", childComponent: 99, facetProp: 88, chain: true }]]),
    );
    const { index, remap, subProps } = exposedPortIndex([comp(1, {}, facet)]);
    expect(index.get(500)).toEqual({ parentUid: 1 }); // existing edges still route to the port
    expect(remap.has(500)).toBe(false); // don't retarget new edges to the inner folder
    expect([...subProps].sort((a, b) => a - b)).toEqual([88, 500]); // value + inner folder's facets
  });
});

describe("classifyCrossEdge", () => {
  const childUids = new Set([1]); // only c1 is visible
  // c1 exposes child prop 500 as a port
  const index = new Map([[500, { parentUid: 1 }]]);

  it("routes to a port when the off-canvas end is an exposed prop (visible = source)", () => {
    const r = classifyCrossEdge(
      edge({ uid: 100, sourceUid: 1, sourcePropertyUid: 11, targetUid: 50, targetPropertyUid: 500 }),
      childUids,
      index,
    );
    expect(r).toMatchObject({
      kind: "port",
      externalIsTarget: true,
      visibleUid: 1,
      visiblePropUid: 11,
      portParentUid: 1,
      portHandle: 500,
    });
  });

  it("routes to a port when the visible end is the target", () => {
    const r = classifyCrossEdge(
      edge({ uid: 101, sourceUid: 50, sourcePropertyUid: 500, targetUid: 1, targetPropertyUid: 12 }),
      childUids,
      index,
    );
    expect(r).toMatchObject({ kind: "port", externalIsTarget: false, visiblePropUid: 12, portHandle: 500 });
  });

  it("falls back to a ghost when the off-canvas prop isn't exposed here", () => {
    const r = classifyCrossEdge(
      edge({
        uid: 102,
        sourceUid: 1,
        sourceProperty: "out",
        sourcePropertyUid: 11,
        targetUid: 9,
        targetProperty: "in",
        targetPropertyUid: 999,
        targetPath: "root/elsewhere",
      }),
      childUids,
      index,
    );
    expect(r).toMatchObject({
      kind: "ghost",
      side: "input", // visible end (source) drives an output into off-canvas input
      visibleUid: 1,
      visiblePropName: "out",
      externalUid: 9,
      externalPropName: "in",
      externalPath: "root/elsewhere",
    });
  });

  it("carries the loopBack flag through", () => {
    const r = classifyCrossEdge(
      edge({ uid: 103, sourceUid: 1, sourcePropertyUid: 11, targetUid: 9, targetPropertyUid: 7, loopBack: true }),
      childUids,
      index,
    );
    expect(r.loopBack).toBe(true);
  });
});
