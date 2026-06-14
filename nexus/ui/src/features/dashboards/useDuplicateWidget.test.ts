import { describe, expect, it } from "vitest";

import { copyTitle } from "@/features/dashboards/useDuplicateWidget";

// Duplicating a panel suffixes its title so the copy is distinguishable, and
// duplicating a copy increments rather than stacking "(copy) (copy)".
describe("copyTitle", () => {
  it("suffixes a plain title", () => {
    expect(copyTitle("Energy")).toBe("Energy (copy)");
  });

  it("increments an existing copy to (copy 2)", () => {
    expect(copyTitle("Energy (copy)")).toBe("Energy (copy 2)");
  });

  it("increments a numbered copy", () => {
    expect(copyTitle("Energy (copy 2)")).toBe("Energy (copy 3)");
  });

  it("names an untitled panel", () => {
    expect(copyTitle("")).toBe("Untitled panel (copy)");
  });
});
