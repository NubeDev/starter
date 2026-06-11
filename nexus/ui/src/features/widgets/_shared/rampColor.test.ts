import { describe, expect, it } from "vitest";

import type { ThresholdStep } from "@/data/types";
import { rampColor } from "@/features/widgets/_shared/rampColor";

// The multi-step threshold ramp picks the highest step a value meets or
// exceeds; the base step (`value: null`) is the floor. Pure (F10).
describe("rampColor", () => {
  const steps: ThresholdStep[] = [
    { value: null, color: "152 76% 44%" },
    { value: 50, color: "38 92% 50%" },
    { value: 80, color: "0 84% 60%" },
  ];

  it("returns undefined for an empty ramp so the caller keeps its colour", () => {
    expect(rampColor(42, [])).toBeUndefined();
  });

  it("falls to the base step below the first bound", () => {
    expect(rampColor(10, steps)).toBe("hsl(152 76% 44%)");
  });

  it("picks the highest step the value meets or exceeds", () => {
    expect(rampColor(50, steps)).toBe("hsl(38 92% 50%)");
    expect(rampColor(79, steps)).toBe("hsl(38 92% 50%)");
    expect(rampColor(80, steps)).toBe("hsl(0 84% 60%)");
    expect(rampColor(120, steps)).toBe("hsl(0 84% 60%)");
  });

  it("does not require pre-sorted steps", () => {
    const shuffled: ThresholdStep[] = [steps[2], steps[0], steps[1]];
    expect(rampColor(80, shuffled)).toBe("hsl(0 84% 60%)");
  });
});
