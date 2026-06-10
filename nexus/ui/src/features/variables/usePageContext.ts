import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { useSearchParams } from "react-router-dom";
import { useStarterClient } from "@nube/starter-client-react";

import { getNavNode } from "@/api/nav/get";
import { getTags } from "@/api/tags/get";
import type { NavContext } from "@/api/types";
import type { PageContext } from "@/data/types";
import {
  assemblePageContext,
  EMPTY_PAGE_CONTEXT,
  type NavContextInput,
} from "@/features/variables/context";

/** Bare URL params (everything except the WS-02 `var-*` selection params and
 *  the WS-13 `nav` id) — the deep-link source a `context`/`url` variable reads
 *  (`?building=b1`). Multi-valued params keep all values. */
function bareUrlParams(params: URLSearchParams): Record<string, string | string[]> {
  const out: Record<string, string | string[]> = {};
  for (const key of new Set(params.keys())) {
    if (key === "nav" || key.startsWith("var-")) continue;
    const all = params.getAll(key);
    out[key] = all.length > 1 ? all : all[0];
  }
  return out;
}

/** A nav node's `context` payload as the assembler needs it (string-valued
 *  maps). The wire `NavContext.values` is `Record<string, unknown>` (jsonb), so
 *  coerce each to a string / string[] and drop anything else. */
function navInputFrom(
  node: { id: string; title: string; parent_id?: string | null; context?: NavContext | null },
  path: string[],
): NavContextInput {
  const values: Record<string, string | string[]> = {};
  for (const [k, v] of Object.entries(node.context?.values ?? {})) {
    if (typeof v === "string") values[k] = v;
    else if (Array.isArray(v) && v.every((e) => typeof e === "string")) {
      values[k] = v as string[];
    } else if (typeof v === "number" || typeof v === "boolean") {
      values[k] = String(v);
    }
  }
  return {
    nodeId: node.id,
    slug: "",
    name: node.title,
    path,
    values,
    tags: node.context?.tags ?? undefined,
  };
}

/** Assemble the page's {@link PageContext} (WS-13 §1) for the dashboard at
 *  `slug` (id `dashboardId`): the nav node from `?nav=:id`, the bare URL
 *  params, and the dashboard's own tags. Returns `EMPTY_PAGE_CONTEXT` until the
 *  async parts (node, tags) resolve, so resolution starts immediately and
 *  re-runs when they arrive — the variable hook keys on the context so panels
 *  re-resolve when navigating between two mounts of one page. */
export function usePageContext(
  slug: string | undefined,
  dashboardId: string | undefined,
): PageContext {
  const client = useStarterClient();
  const [params] = useSearchParams();
  const navId = params.get("nav") ?? undefined;

  // The slug the node carries is its dashboard target's slug; we already know
  // it (`slug`), so we pass it through for the `nav` + `slug` context source.
  const nodeQuery = useQuery({
    queryKey: ["nexus", "nav", "node", navId ?? ""],
    enabled: !!navId,
    queryFn: () => getNavNode(client, navId!),
  });

  const tagsQuery = useQuery({
    queryKey: ["nexus", "tags", "dashboard", dashboardId ?? ""],
    enabled: !!dashboardId,
    queryFn: () => getTags(client, "dashboard", dashboardId!),
  });

  // The bare params are derived synchronously from the URL; the nav node and
  // tags fold in once fetched. A stable string of the inputs keeps the memo
  // from re-assembling on every render.
  const urlBare = useMemo(() => bareUrlParams(params), [params]);

  return useMemo(() => {
    const node = nodeQuery.data;
    const nav: NavContextInput | undefined = node
      ? { ...navInputFrom(node, []), slug: slug ?? "" }
      : undefined;
    const dashboardTags: Record<string, string | null> = {};
    for (const t of tagsQuery.data ?? []) dashboardTags[t.key] = t.value ?? null;

    if (!nav && Object.keys(urlBare).length === 0 && !tagsQuery.data) {
      return EMPTY_PAGE_CONTEXT;
    }
    return assemblePageContext({ nav, url: urlBare, dashboardTags });
  }, [nodeQuery.data, tagsQuery.data, urlBare, slug]);
}
