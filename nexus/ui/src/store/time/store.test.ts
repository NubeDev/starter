import { beforeEach, describe, expect, it } from "vitest";

import { DEFAULT_RANGE, useTimeStore } from "@/store/time/store";

// The time store holds ephemeral client state (range, refresh, the per-tick
// frozen `now`). Tested by driving its actions directly. The `tick` is the
// cache-key correctness primitive — assert it advances exactly when it should.
describe("time store", () => {
  beforeEach(() => {
    useTimeStore.setState({
      range: DEFAULT_RANGE,
      refresh: 0,
      tick: 0,
      now: new Date(),
    });
  });

  it("defaults to last-6h with refresh off", () => {
    const s = useTimeStore.getState();
    expect(s.range).toEqual({ from: "now-6h", to: "now" });
    expect(s.refresh).toBe(0);
  });

  it("setRange freezes a fresh instant and advances the tick", () => {
    const before = useTimeStore.getState().tick;
    useTimeStore.getState().setRange({ from: "now-24h", to: "now" });
    const s = useTimeStore.getState();
    expect(s.range).toEqual({ from: "now-24h", to: "now" });
    expect(s.tick).toBe(before + 1);
  });

  it("bump advances the tick (cache busts once per refresh)", () => {
    const before = useTimeStore.getState().tick;
    useTimeStore.getState().bump();
    expect(useTimeStore.getState().tick).toBe(before + 1);
  });

  it("setRefresh does not advance the tick", () => {
    const before = useTimeStore.getState().tick;
    useTimeStore.getState().setRefresh(10);
    expect(useTimeStore.getState().refresh).toBe(10);
    expect(useTimeStore.getState().tick).toBe(before);
  });
});
