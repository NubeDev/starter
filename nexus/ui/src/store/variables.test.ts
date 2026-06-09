import { describe, expect, it } from "vitest";

import type { ResolvedVariable } from "@/data/types";
import { toQueryVariables, useVariableStore } from "@/store/variables";

// The store selector that builds the query-layer `QueryVariable[]` (C3), and
// the selection/revision bump that busts panel cache exactly once per change.
function rv(name: string, current: string[]): ResolvedVariable {
  return {
    id: name,
    name,
    kind: "custom",
    options: [],
    optionsConfig: {},
    current,
    multi: current.length > 1,
    includeAll: false,
    hidden: false,
    sortOrder: 0,
  };
}

describe("toQueryVariables", () => {
  it("maps current values and omits empty selections", () => {
    expect(
      toQueryVariables([rv("region", ["us", "eu"]), rv("host", [])]),
    ).toEqual([{ name: "region", values: ["us", "eu"] }]);
  });
});

describe("useVariableStore", () => {
  it("bumps revision on each selection change", () => {
    const { setSelection, reset } = useVariableStore.getState();
    reset();
    const before = useVariableStore.getState().revision;
    setSelection("region", ["us"]);
    expect(useVariableStore.getState().revision).toBe(before + 1);
    expect(useVariableStore.getState().selections.region).toEqual(["us"]);
  });

  it("reset clears resolved + selections", () => {
    useVariableStore.getState().setSelection("a", ["1"]);
    useVariableStore.getState().reset();
    expect(useVariableStore.getState().selections).toEqual({});
    expect(useVariableStore.getState().resolved).toEqual([]);
  });
});
