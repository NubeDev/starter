import { describe, it, expect, beforeEach, afterAll } from "vitest";
import { setEngineBase, addNode, addEdge, updateNode, getNodeByUid, copyNodes } from "../lib/rest";
import { serializeFacet, parseFacet, rawFacet, FACET_PROP } from "../lib/facet";
import {
  CE_BASE,
  ceReachable,
  resetTestFolder,
  deleteTestFolder,
  makeAdd,
  readValues,
} from "./ce";

// Structural operations the editor performs (copy, Configure persistence,
// grouping) asserted against the live engine.
setEngineBase(CE_BASE);
const reachable = await ceReachable();

const FOLDER = "core-extRoot::Folder";

describe.skipIf(!reachable)("CE structural ops (live engine)", () => {
  let folder: number;
  beforeEach(async () => {
    folder = await resetTestFolder();
  });
  afterAll(async () => {
    await deleteTestFolder();
  });

  it("copy/nodes duplicates a component and returns an old→new uidMap", async () => {
    const a = await makeAdd(folder, "orig", 3, 4); // out = 7
    const res = await copyNodes({
      componentUids: [a.uid],
      destParentUid: folder,
      includeInternalEdges: true,
    });
    const compMap = res.uidMap?.components ?? {};
    const newUid = compMap[String(a.uid)];
    expect(newUid).toBeDefined();
    expect(newUid).not.toBe(a.uid);

    // The copy is an independent component that computes the same sum.
    const copy = (res.nodes ?? []).find((n) => n.uid === newUid);
    expect(copy).toBeTruthy();
    const out = copy!.properties.out.uid;
    const vals = await readValues({ components: [newUid], watch: [out], expect: { [out]: 7 } });
    expect(vals.get(out)).toBe(7);
  });

  it("persists a written __facets string (Configure panel write-through)", async () => {
    const a = await makeAdd(folder, "cfg", 1, 0);
    const in1Uid = a.properties.in1.uid;
    const facet = serializeFacet(
      new Map([
        [
          in1Uid,
          {
            label: "Speed",
            unit: "rpm",
            decimals: 1,
            aliases: [
              { code: 0, label: "off" },
              { code: 1, label: "on" },
            ],
          },
        ],
      ]),
    );
    await updateNode(a.uid, { properties: { [FACET_PROP]: { value: facet } } });

    const resp = await getNodeByUid(a.uid, { depth: 0 });
    const parsed = parseFacet(rawFacet(resp.nodes[0]?.properties) ?? "");
    const rec = parsed.get(in1Uid);
    expect(rec?.label).toBe("Speed");
    expect(rec?.unit).toBe("rpm");
    expect(rec?.decimals).toBe(1);
    expect(rec?.aliases?.map((x) => x.label)).toEqual(["off", "on"]);
  });

  it("grouping: reparenting a component into a folder keeps cross-folder dataflow alive", async () => {
    const a = await makeAdd(folder, "grpA", 2, 0); // out = 2
    const b = await makeAdd(folder, "grpB", 10, 0); // 10 + a.out
    await addEdge({
      sourceUid: a.uid,
      sourcePropUid: a.properties.out.uid,
      targetUid: b.uid,
      targetPropUid: b.properties.in2.uid,
    });
    const bOut = b.properties.out.uid;
    let vals = await readValues({ components: [a.uid, b.uid], watch: [bOut], expect: { [bOut]: 12 } });
    expect(vals.get(bOut)).toBe(12);

    // The engine-level half of "Group": create a folder and reparent B into it.
    // The A→B edge now crosses the new boundary (the editor would project it
    // through an exposed port); the value must keep propagating.
    const g = await addNode({ type: FOLDER, name: "grp", parentUid: folder });
    await updateNode(b.uid, { parentUid: g.uid });

    const moved = await getNodeByUid(b.uid, { depth: 0 });
    expect(moved.nodes[0]?.parent).toBe(g.uid);
    vals = await readValues({ components: [b.uid], watch: [bOut], expect: { [bOut]: 12 } });
    expect(vals.get(bOut)).toBe(12);
  });
});
