// React-Query-backed store for `GET /api/node-kinds`.
//
// Slice A of `DOCS/extensions/scope/FLOW-NODES.md`: the FlowEditor
// palette reads from this store rather than the hard-coded
// `BUILTIN_NODE_KINDS` array from `@nube/starter-ui-flow`. Slice B's
// `POST /admin/extensions/reload` will publish an `extensions.reload`
// SSE event; for now a `staleTime: 30s` window keeps the UI in step
// without re-fetching on every render.

import { useQuery, useQueryClient } from "@tanstack/react-query";

import { api, type NodeKindDto } from "@/lib/api";

const NODE_KINDS_QUERY_KEY = ["node-kinds"] as const;

/**
 * Fetch the list of node kinds the host has registered.
 *
 * Returns a stable, alphabetically-sorted slice. The query stays
 * fresh for 30 seconds so a paint of the editor doesn't refetch on
 * every navigation; slice B's `extensions.reload` SSE will
 * `invalidateQueries(["node-kinds"])` to force an immediate refresh
 * after a hot reload.
 */
export function useNodeKinds() {
  return useQuery<NodeKindDto[]>({
    queryKey: NODE_KINDS_QUERY_KEY,
    queryFn: () => api.nodeKinds.list(),
    staleTime: 30_000,
  });
}

/**
 * Imperative refresh — slice B's SSE listener calls this after
 * `extensions.reload`. Exposed today so a developer can test the
 * refresh path manually via the React DevTools.
 */
export function useInvalidateNodeKinds() {
  const qc = useQueryClient();
  return () => qc.invalidateQueries({ queryKey: NODE_KINDS_QUERY_KEY });
}
