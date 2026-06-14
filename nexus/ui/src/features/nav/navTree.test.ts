import { describe, expect, it } from "vitest";

import type { NavNodeDetail } from "@/api/types";
import { buildNavTree, navNodeHref } from "@/features/nav/navTree";

function node(
  id: string,
  partial: Partial<NavNodeDetail> = {},
): NavNodeDetail {
  return {
    id,
    title: id,
    sort_order: 0,
    target: { kind: "group" },
    ...partial,
  } as NavNodeDetail;
}

describe("buildNavTree", () => {
  it("nests children under parents, ordered by sort_order then title", () => {
    const tree = buildNavTree([
      node("root", { sort_order: 1 }),
      node("a", { parent_id: "root", sort_order: 2 }),
      node("b", { parent_id: "root", sort_order: 1 }),
      node("other", { sort_order: 0 }),
    ]);
    expect(tree.map((n) => n.id)).toEqual(["other", "root"]);
    const root = tree.find((n) => n.id === "root")!;
    expect(root.children.map((n) => n.id)).toEqual(["b", "a"]);
  });

  it("carries the ancestor path root-first", () => {
    const tree = buildNavTree([
      node("buildings", { title: "Buildings" }),
      node("b1", { title: "Building-1", parent_id: "buildings" }),
    ]);
    const b1 = tree[0].children[0];
    expect(b1.path).toEqual(["Buildings"]);
  });

  it("re-roots a node whose parent was filtered out (access)", () => {
    // `b1`'s parent `buildings` is not in the visible set (ungranted), so b1
    // re-roots rather than disappearing — the grant is per-node.
    const tree = buildNavTree([
      node("b1", { title: "Building-1", parent_id: "buildings" }),
    ]);
    expect(tree.map((n) => n.id)).toEqual(["b1"]);
    expect(tree[0].path).toEqual([]);
  });
});

describe("navNodeHref", () => {
  const slugOf = (id: string) => (id === "dash-1" ? "energy" : undefined);

  it("a dashboard node opens d/:slug?nav=:id", () => {
    const n = node("n1", {
      target: { kind: "dashboard", dashboardId: "dash-1" },
    });
    expect(navNodeHref(n, slugOf)).toBe("/d/energy?nav=n1");
  });

  it("a route node goes to the static page", () => {
    const n = node("n2", { target: { kind: "route", route: "agents" } });
    expect(navNodeHref(n, slugOf)).toBe("/agents");
  });

  it("a group node is not a link", () => {
    expect(navNodeHref(node("g"), slugOf)).toBeNull();
  });

  it("a dashboard node with an unknown slug is not a link", () => {
    const n = node("n3", {
      target: { kind: "dashboard", dashboardId: "missing" },
    });
    expect(navNodeHref(n, slugOf)).toBeNull();
  });
});
