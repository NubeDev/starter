// Build a nested nav tree from the flat, access-filtered node list the API
// returns (WS-13 §4). The server filters to nodes the principal holds `view`
// on, so a granted leaf may arrive without its (ungranted) parent — such a node
// re-roots here rather than vanishing, so the grant is honoured per-node.

import type { NavNodeDetail } from "@/api/types";

/** A nav node plus its resolved children, for rendering the tree. */
export interface NavTreeNode extends NavNodeDetail {
  children: NavTreeNode[];
  /** Ancestor titles, root-first — the `path` a context/nav variable reads. */
  path: string[];
}

/** Assemble the flat list into a forest, ordered by `sort_order` then title at
 *  each level. A node whose `parent_id` is absent from the list (filtered out
 *  by access, or dangling) is treated as a root so it stays reachable. */
export function buildNavTree(nodes: ReadonlyArray<NavNodeDetail>): NavTreeNode[] {
  const present = new Set(nodes.map((n) => n.id));
  const byParent = new Map<string | null, NavNodeDetail[]>();
  for (const n of nodes) {
    // Re-root a node whose parent is not in the visible set.
    const parent =
      n.parent_id && present.has(n.parent_id) ? n.parent_id : null;
    const bucket = byParent.get(parent) ?? [];
    bucket.push(n);
    byParent.set(parent, bucket);
  }

  const order = (a: NavNodeDetail, b: NavNodeDetail) =>
    a.sort_order - b.sort_order || a.title.localeCompare(b.title);

  const build = (parent: string | null, path: string[]): NavTreeNode[] =>
    (byParent.get(parent) ?? [])
      .slice()
      .sort(order)
      .map((n) => ({
        ...n,
        path,
        children: build(n.id, [...path, n.title]),
      }));

  return build(null, []);
}

/** The route a `dashboard`/`route` node navigates to. A `dashboard` node opens
 *  `d/:slug?nav=:id` so the page reads its context (WS-13 §4); a `route` node
 *  goes to the static page. `group` nodes are not links. Returns `null` for a
 *  group or an unresolvable target. */
export function navNodeHref(
  node: NavNodeDetail,
  dashboardSlug: (dashboardId: string) => string | undefined,
): string | null {
  switch (node.target.kind) {
    case "group":
      return null;
    case "route":
      return `/${node.target.route}`;
    case "dashboard": {
      const slug = dashboardSlug(node.target.dashboardId);
      return slug
        ? `/d/${slug}?nav=${encodeURIComponent(node.id)}`
        : null;
    }
  }
}
