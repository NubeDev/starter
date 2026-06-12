import { describe, it, expect, beforeEach, afterAll } from "vitest";
import { setEngineBase, addNode, addEdge } from "../lib/rest";
import {
  CE_BASE,
  ADD,
  SUBTRACT,
  ceReachable,
  resetTestFolder,
  deleteTestFolder,
  makeAdd,
  readValues,
} from "./ce";

// Live-engine dataflow correctness. Builds real graphs via REST, reads the
// computed outputs off the binary WebSocket, and asserts the math. Skips when no
// CE is reachable (see ce.ts). Clean slate before each test: the test folder is
// deleted + recreated, so no test sees another's components.
setEngineBase(CE_BASE);
const reachable = await ceReachable();
if (!reachable) {
  // eslint-disable-next-line no-console
  console.warn(`[itest] CE not reachable at ${CE_BASE} — skipping live dataflow tests`);
}

describe.skipIf(!reachable)("CE dataflow (live engine + NubeIO math)", () => {
  let folder: number;
  beforeEach(async () => {
    folder = await resetTestFolder();
  });
  afterAll(async () => {
    await deleteTestFolder();
  });

  it("add computes in1 + in2", async () => {
    const a = await makeAdd(folder, "addA", 2, 3);
    const out = a.properties.out.uid;
    const vals = await readValues({ components: [a.uid], watch: [out], expect: { [out]: 5 } });
    expect(vals.get(out)).toBe(5);
  });

  it("subtract computes in1 - in2", async () => {
    const s = await addNode({
      type: SUBTRACT,
      name: "subS",
      parentUid: folder,
      defaultValues: { properties: { in1: { value: 10 }, in2: { value: 3 } } },
    });
    const out = s.properties.out.uid;
    const vals = await readValues({ components: [s.uid], watch: [out], expect: { [out]: 7 } });
    expect(vals.get(out)).toBe(7);
  });

  it("a chain of 5 adds, each with one input = 1, sums to 5", async () => {
    const N = 5;
    const adds = [];
    for (let i = 0; i < N; i++) adds.push(await makeAdd(folder, `a${i}`, 1, 0));
    // Wire add[k-1].out → add[k].in2, so each stage adds 1 to the running total.
    for (let k = 1; k < N; k++) {
      await addEdge({
        sourceUid: adds[k - 1].uid,
        sourcePropUid: adds[k - 1].properties.out.uid,
        targetUid: adds[k].uid,
        targetPropUid: adds[k].properties.in2.uid,
      });
    }
    const finalOut = adds[N - 1].properties.out.uid;
    const vals = await readValues({
      components: adds.map((a) => a.uid),
      watch: [finalOut],
      expect: { [finalOut]: N },
    });
    expect(vals.get(finalOut)).toBe(N);
  });

  it("an override sets the live value, and clearing returns it to engine control", async () => {
    const a = await makeAdd(folder, "ovr", 0, 0); // both inputs 0 → out 0
    const out = a.properties.out.uid;
    const { patchOverrides } = await import("../lib/rest");

    await patchOverrides(a.uid, {
      setOverrides: [
        { property: "in1", value: 4, duration: 60 },
        { property: "in2", value: 3, duration: 60 },
      ],
    });
    let vals = await readValues({ components: [a.uid], watch: [out], expect: { [out]: 7 } });
    expect(vals.get(out)).toBe(7);

    await patchOverrides(a.uid, { clearOverrides: ["in1", "in2"] });
    vals = await readValues({ components: [a.uid], watch: [out], expect: { [out]: 0 } });
    expect(vals.get(out)).toBe(0);
  });

  it("a deleted edge stops propagating (last stage falls back to its own input)", async () => {
    // a(in1=1,in2=0)=1 → b.in2 ; b(in1=1) = 2. Remove the edge → b = in1 + 0 = 1.
    const a = await makeAdd(folder, "nodeA", 1, 0);
    const b = await makeAdd(folder, "nodeB", 1, 0);
    const edge = await addEdge({
      sourceUid: a.uid,
      sourcePropUid: a.properties.out.uid,
      targetUid: b.uid,
      targetPropUid: b.properties.in2.uid,
    });
    const bOut = b.properties.out.uid;
    let vals = await readValues({ components: [a.uid, b.uid], watch: [bOut], expect: { [bOut]: 2 } });
    expect(vals.get(bOut)).toBe(2);

    const { removeEdge } = await import("../lib/rest");
    await removeEdge(edge.uid);
    vals = await readValues({ components: [b.uid], watch: [bOut], expect: { [bOut]: 1 } });
    expect(vals.get(bOut)).toBe(1);
  });
});
