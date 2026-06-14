import { describe, expect, it } from "vitest";

import {
  dependenciesOf,
  dependentsOf,
  referencedVariables,
  resolutionOrder,
  VariableCycleError,
  type VarDef,
} from "@/features/variables/deps";

// Dependency analysis underpins cascading (item 6): which variables a query
// references, the order to resolve them, cycle rejection, and which children
// a change invalidates (item 7).

function q(name: string, sql: string, order = 0): VarDef & { sortOrder: number } {
  return { name, kind: "query", optionsConfig: { sql, datasourceId: "d" }, sortOrder: order };
}
function c(name: string, order = 0): VarDef & { sortOrder: number } {
  return { name, kind: "custom", optionsConfig: { optionsText: "a,b" }, sortOrder: order };
}

describe("referencedVariables", () => {
  it("finds $var, ${var}, ${var:csv}, $__sqlIn(var)", () => {
    const refs = referencedVariables(
      "select x from t where a=$region and b in ($__sqlIn(host)) and c=${dc:csv}",
    );
    expect(refs.sort()).toEqual(["dc", "host", "region"]);
  });

  it("excludes built-in __ macros", () => {
    const refs = referencedVariables(
      "select $__timeFilter(ts), $__from, $__to, $__interval, $real",
    );
    expect(refs).toEqual(["real"]);
  });
});

describe("dependenciesOf", () => {
  it("a query variable depends only on known references", () => {
    const known = new Set(["dc"]);
    const def = q("host", "select h where d=$dc and z=$unknown");
    expect(dependenciesOf(def, known)).toEqual(["dc"]);
  });

  it("non-query kinds have no dependencies", () => {
    expect(dependenciesOf(c("region"), new Set(["region"]))).toEqual([]);
  });

  it("a variable never depends on itself", () => {
    const known = new Set(["x"]);
    expect(dependenciesOf(q("x", "select x where a=$x"), known)).toEqual([]);
  });
});

describe("resolutionOrder", () => {
  it("orders dependencies before dependents", () => {
    const order = resolutionOrder([
      q("host", "select h where d=$dc", 1),
      c("dc", 0),
    ]);
    const names = order.map((d) => d.name);
    expect(names.indexOf("dc")).toBeLessThan(names.indexOf("host"));
  });

  it("breaks ties by sortOrder then name for independents", () => {
    const order = resolutionOrder([c("b", 1), c("a", 0)]);
    expect(order.map((d) => d.name)).toEqual(["a", "b"]);
  });

  it("rejects a direct cycle with the chain", () => {
    expect(() =>
      resolutionOrder([q("a", "x=$b"), q("b", "x=$a")]),
    ).toThrow(VariableCycleError);
  });

  it("rejects a transitive cycle", () => {
    try {
      resolutionOrder([q("a", "x=$b"), q("b", "x=$c"), q("c", "x=$a")]);
      throw new Error("expected a cycle");
    } catch (e) {
      expect(e).toBeInstanceOf(VariableCycleError);
      expect((e as VariableCycleError).cycle).toContain("a");
    }
  });
});

describe("dependentsOf", () => {
  it("returns the transitive children of a change", () => {
    const defs = [q("a", "x=$root"), q("b", "x=$a"), c("root")];
    const out = dependentsOf("root", defs);
    expect([...out].sort()).toEqual(["a", "b"]);
  });

  it("is empty for a leaf with no dependents", () => {
    const defs = [q("a", "x=$root"), c("root")];
    expect([...dependentsOf("a", defs)]).toEqual([]);
  });
});
