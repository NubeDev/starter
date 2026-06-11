import { describe, expect, it } from "vitest";

import { thresholdState } from "@/features/widgets/_shared/thresholdState";

// Threshold colouring must handle both ascending metrics (load: high is
// bad) and descending ones (battery SoC: low is bad). The orientation is
// inferred from whether crit sits above or below warn. Carried forward
// from the mock's gauge as pure logic — no fake data attached.
describe("thresholdState", () => {
  it("returns ok when thresholds are absent", () => {
    expect(thresholdState(99, undefined, undefined)).toBe("ok");
    expect(thresholdState(99, 80, undefined)).toBe("ok");
  });

  it("ascending: crit above warn (e.g. CPU load)", () => {
    expect(thresholdState(50, 70, 90)).toBe("ok");
    expect(thresholdState(75, 70, 90)).toBe("warn");
    expect(thresholdState(95, 70, 90)).toBe("crit");
  });

  it("descending: crit below warn (e.g. battery SoC)", () => {
    expect(thresholdState(80, 35, 15)).toBe("ok");
    expect(thresholdState(30, 35, 15)).toBe("warn");
    expect(thresholdState(10, 35, 15)).toBe("crit");
  });

  it("boundaries are inclusive on the breaching side", () => {
    expect(thresholdState(90, 70, 90)).toBe("crit");
    expect(thresholdState(15, 35, 15)).toBe("crit");
  });
});
