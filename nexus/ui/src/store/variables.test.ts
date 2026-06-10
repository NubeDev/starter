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

  it("setResolved bumps revision when a bound value changes (WS-13 §5)", () => {
    const { setResolved, reset } = useVariableStore.getState();
    reset();
    setResolved([rv("building", ["b1"])]);
    const after1 = useVariableStore.getState().revision;
    // A context navigation changes `current` with no user selection — panels
    // must re-key, so the revision bumps.
    setResolved([rv("building", ["b2"])]);
    expect(useVariableStore.getState().revision).toBe(after1 + 1);
  });

  it("setResolved does not bump revision when only options change", () => {
    const { setResolved, reset } = useVariableStore.getState();
    reset();
    setResolved([rv("building", ["b1"])]);
    const after1 = useVariableStore.getState().revision;
    // Same bound value, different option metadata → no needless cache bust.
    const sameValueMoreOptions: ResolvedVariable = {
      ...rv("building", ["b1"]),
      options: [{ text: "b1", value: "b1" }, { text: "b2", value: "b2" }],
    };
    setResolved([sameValueMoreOptions]);
    expect(useVariableStore.getState().revision).toBe(after1);
  });
});
