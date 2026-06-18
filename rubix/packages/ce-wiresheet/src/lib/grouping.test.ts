import { describe, expect, it } from "vitest";
import { groupBoundary, groupChainedBoundary } from "./grouping";
import { FACET_PROP, serializeFacet } from "./facet";
import type { Component, Edge, Property } from "./engine-types";

function prop(uid: number, componentUid: number): Property {
  return { uid, componentUid, category: 0, value: 0, statusFlags: 0 };
}
// props: map of propName -> uid; always carries a __facets prop too.
function comp(uid: number, props: Record<string, number>, facetUid: number): Component {
  const properties: Record<string, Property> = { [FACET_PROP]: prop(facetUid, uid) };
  for (const [name, p] of Object.entries(props)) properties[name] = prop(p, uid);
  return { uid, name: `c${uid}`, type: "math::add", path: `root/c${uid}`, parent: 0, properties };
}
function edge(
  uid: number,
  s: number,
  sp: string,
  spu: number | undefined,
  t: number,
  tp: string,
  tpu: number | undefined,
): Edge {
  return {
    uid,
    sourceUid: s,
    sourceProperty: sp,
    sourcePropertyUid: spu,
    targetUid: t,
    targetProperty: tp,
    targetPropertyUid: tpu,
  };
}

// Topology: c1.out → c2.in (c2.out is internal) → c3.in.  Group {c2}.
const comps = new Map<number, Component>([
  [1, comp(1, { out: 11 }, 19)],
  [2, comp(2, { in: 21, out: 22 }, 29)],
  [3, comp(3, { in: 31 }, 39)],
]);

describe("groupBoundary", () => {
  it("exposes the in-group target prop as an input and source prop as an output", () => {
    const edges = [
      edge(100, 1, "out", 11, 2, "in", 21), // crosses in → c2.in exposed (input)
      edge(101, 2, "out", 22, 3, "in", 31), // crosses out → c2.out exposed (output)
    ];
    const b = groupBoundary(new Set([2]), edges, comps);
    expect(b.get(21)).toEqual({
      childComponent: 2,
      side: "input",
      label: "in",
      facetProp: 29,
    });
    expect(b.get(22)).toEqual({
      childComponent: 2,
      side: "output",
      label: "out",
      facetProp: 29,
    });
  });

  it("ignores edges fully inside or fully outside the group", () => {
    const edges = [
      edge(100, 1, "out", 11, 3, "in", 31), // both outside {2}
      edge(101, 2, "out", 22, 2, "in", 21), // both inside {2}
    ];
    expect(groupBoundary(new Set([2]), edges, comps).size).toBe(0);
  });

  it("falls back to a name lookup when the edge lacks the prop uid (ghost-bug guard)", () => {
    const edges = [edge(100, 1, "out", 11, 2, "in", undefined)];
    const b = groupBoundary(new Set([2]), edges, comps);
    expect(b.get(21)?.side).toBe("input"); // resolved 21 via c2.properties.in.uid
  });
});

// A folder (uid 2) that already exposes deep child 200's prop 50 as an input port.
function folder(uid: number, facetUid: number, value: string): Component {
  const properties: Record<string, Property> = {
    [FACET_PROP]: { ...prop(facetUid, uid), value, systemRole: 2 } as Property,
  };
  return { uid, name: `f${uid}`, type: "core-extRoot::Folder", path: `root/f${uid}`, parent: 0, properties };
}

describe("groupChainedBoundary", () => {
  // F=2 exposes prop 50 (owned by deep child 200) as an INPUT port.
  const F = folder(2, 29, serializeFacet(new Map([[50, { expose: "input", childComponent: 200, facetProp: 250, label: "in" }]])));

  it("emits a chained port for a boundary edge through a grouped folder's exposed port", () => {
    // outside sibling 1.out → deep child 200.in (which F exposes as 50)
    const edges = [edge(100, 1, "out", 11, 200, "in", 50)];
    const c = groupChainedBoundary(new Set([2]), [F], edges);
    expect(c.get(50)).toEqual({ prop: 50, innerFolder: 2, side: "input", label: "in", facetProp: 29 });
  });

  it("skips when the other end is a selected member (internal)", () => {
    const edges = [edge(100, 7, "out", 11, 200, "in", 50)]; // 7 also grouped
    expect(groupChainedBoundary(new Set([2, 7]), [F], edges).size).toBe(0);
  });

  it("skips when both ends route through grouped folders' exposed ports (internal)", () => {
    const F2 = folder(3, 39, serializeFacet(new Map([[60, { expose: "output", childComponent: 300, facetProp: 350, label: "out" }]])));
    const edges = [edge(100, 300, "out", 60, 200, "in", 50)]; // 60 (F2) ↔ 50 (F), both grouped
    expect(groupChainedBoundary(new Set([2, 3]), [F, F2], edges).size).toBe(0);
  });
});
