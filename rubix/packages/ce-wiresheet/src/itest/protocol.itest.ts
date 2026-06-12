import { describe, it, expect, beforeEach, afterAll } from "vitest";
import { setEngineBase } from "../lib/rest";
import { STATUS_OVERRIDDEN } from "../lib/engine-types";
import {
  CE_BASE,
  ceReachable,
  resetTestFolder,
  deleteTestFolder,
  makeAdd,
  readValues,
  waitForTopology,
} from "./ce";

// WS protocol behaviours the editor depends on, asserted against the live engine.
setEngineBase(CE_BASE);
const reachable = await ceReachable();

describe.skipIf(!reachable)("CE WebSocket protocol (live engine)", () => {
  let folder: number;
  beforeEach(async () => {
    folder = await resetTestFolder();
  });
  afterAll(async () => {
    await deleteTestFolder();
  });

  it("streams a single prop via property-level subscription (exposed-ports path)", async () => {
    const a = await makeAdd(folder, "propSub", 6, 1); // out = 7
    const out = a.properties.out.uid;
    // Subscribe ONLY to the prop uid — no component subscription. This is how an
    // off-canvas exposed port streams its value (API_REQUESTS §1).
    const vals = await readValues({ properties: [out], watch: [out], expect: { [out]: 7 } });
    expect(vals.get(out)).toBe(7);
  });

  it("sets the OVERRIDDEN status bit on the STATUS frame when a prop is overridden", async () => {
    const a = await makeAdd(folder, "statBit", 0, 0);
    const in1 = a.properties.in1.uid;
    const { patchOverrides } = await import("../lib/rest");
    await patchOverrides(a.uid, { setOverrides: [{ property: "in1", value: 5, duration: 60 }] });
    const flags = await readValues({
      components: [a.uid],
      watch: [in1],
      status: true,
      expect: { [in1]: STATUS_OVERRIDDEN },
    });
    expect((Number(flags.get(in1)) & STATUS_OVERRIDDEN) === STATUS_OVERRIDDEN).toBe(true);
  });

  it("pushes a topologyAdded event to a connected socket on a REST add", async () => {
    const msg = await waitForTopology("topologyAdded", () => makeAdd(folder, "pushNode", 1, 0));
    expect(msg).not.toBeNull();
    expect(msg?.type).toBe("topologyAdded");
  });
});
