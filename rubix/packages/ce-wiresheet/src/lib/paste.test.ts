import { describe, expect, it } from "vitest";
import { planPaste } from "./paste";
import { serializeFacet, parseFacet, FACET_PROP } from "./facet";
import type { Component, Property } from "./engine-types";

function prop(uid: number, componentUid: number): Property {
  return { uid, componentUid, category: 0, value: 0, statusFlags: 0 };
}
function comp(
  uid: number,
  parent: number,
  pos: { x: number; y: number },
  facet?: string,
  children?: Component[],
): Component {
  const properties: Record<string, Property> = {};
  if (facet != null) properties[FACET_PROP] = { ...prop(900 + uid, uid), value: facet, systemRole: 2 };
  return {
    uid,
    name: `c${uid}`,
    type: "math::add",
    path: `root/c${uid}`,
    parent,
    metadata: { position: pos },
    properties,
    children,
  };
}

describe("planPaste", () => {
  it("translates top-level clones so their bbox centre lands at the cursor", () => {
    const clones = [comp(1, 0, { x: 0, y: 0 }), comp(2, 0, { x: 100, y: 0 })];
    const { updates, newUids } = planPaste(clones, 0, { x: 250, y: 250 });
    // centre of (0,0)+(100,0) = (50,0); offset to (250,250) = (+200,+250)
    expect(updates.find((u) => u.uid === 1)?.position).toEqual({ x: 200, y: 250 });
    expect(updates.find((u) => u.uid === 2)?.position).toEqual({ x: 300, y: 250 });
    expect(newUids).toEqual([1, 2]);
  });

  it("repositions only top-level clones; descendants are flattened but not moved/selected", () => {
    const folder = comp(1, 0, { x: 0, y: 0 }, undefined, [comp(2, 1, { x: 10, y: 10 })]);
    const { updates, newUids } = planPaste([folder], 0, { x: 0, y: 0 });
    expect(newUids).toEqual([1]); // only the folder (parent === dest)
    expect(updates.find((u) => u.uid === 2)?.position).toBeUndefined();
  });

  it("remaps copied __facets uid references using the uidMap", () => {
    const facet = serializeFacet(
      new Map([[200, { expose: "input", childComponent: 50, facetProp: 60 }]]),
    );
    const folder = comp(1, 0, { x: 0, y: 0 }, facet);
    const { updates } = planPaste([folder], 0, { x: 0, y: 0 }, {
      components: { 50: 5050 },
      properties: { 200: 2002, 60: 6060 },
    });
    const remapped = parseFacet(updates.find((u) => u.uid === 1)!.properties![FACET_PROP].value);
    const rec = remapped.get(2002);
    expect(rec?.childComponent).toBe(5050);
    expect(rec?.facetProp).toBe(6060);
  });

  it("leaves __facets untouched when no uidMap is provided", () => {
    const facet = serializeFacet(new Map([[200, { expose: "input", childComponent: 50 }]]));
    const { updates } = planPaste([comp(1, 0, { x: 0, y: 0 }, facet)], 0, { x: 0, y: 0 });
    expect(updates.find((u) => u.uid === 1)?.properties).toBeUndefined();
  });
});
