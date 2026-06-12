import { describe, expect, it } from "vitest";
import { groupBoundary } from "./grouping";
import { FACET_PROP } from "./facet";
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
