import { create } from "zustand";

import type { QueryVariable } from "@/api/types";
import type { ResolvedVariable } from "@/data/types";

// Dashboard variable state (WS-02): the user's per-variable selection plus
// the resolved variables (definition + computed options) the bar renders
// and the query layer reads. Ephemeral client state, like the time and UI
// stores — definitions live in TanStack Query, selections in the URL; this
// store is the in-memory join the panels subscribe to.
//
// `revision` is the cache-key nonce (mirrors the time store's `tick`): it
// bumps whenever any selection changes so panel queries re-key and refetch
// exactly once per change (item 7 / C3), not on every render.
//
// `create` is imported from the workspace's single `zustand` federation
// singleton so host and remotes share one store runtime.

interface VariableState {
  /** Resolved variables in bar order; the source of truth for rendering and
   *  for building the query-layer `QueryVariable[]`. */
  resolved: ReadonlyArray<ResolvedVariable>;
  /** Selections keyed by variable name (without `$`); the authoritative
   *  pick the resolver and URL sync read. */
  selections: Record<string, ReadonlyArray<string>>;
  /** Bumped on any selection change so query keys bust once per change. */
  revision: number;
  /** Replace the resolved set (after a fetch/re-resolve). Does not touch
   *  selections — those are owned by the user / URL. */
  setResolved: (resolved: ReadonlyArray<ResolvedVariable>) => void;
  /** Set one variable's selection and bump the revision. */
  setSelection: (name: string, values: ReadonlyArray<string>) => void;
  /** Bulk-apply selections (e.g. restoring from the URL on mount) without
   *  bumping per-key; bumps the revision once. */
  applySelections: (selections: Record<string, ReadonlyArray<string>>) => void;
  /** Clear everything (on dashboard unmount / slug change) so one
   *  dashboard's variables never leak into the next. */
  reset: () => void;
}

export const useVariableStore = create<VariableState>((set) => ({
  resolved: [],
  selections: {},
  revision: 0,
  setResolved: (resolved) => set({ resolved }),
  setSelection: (name, values) =>
    set((s) => ({
      selections: { ...s.selections, [name]: values },
      revision: s.revision + 1,
    })),
  applySelections: (selections) =>
    set((s) => ({
      selections: { ...s.selections, ...selections },
      revision: s.revision + 1,
    })),
  reset: () => set({ resolved: [], selections: {}, revision: 0 }),
}));

/** Build the query-layer `QueryVariable[]` from the resolved set: each
 *  variable contributes its current value(s). Variables with no value are
 *  omitted (an unselected variable interpolates to nothing). Pure selector
 *  so the query hook can derive its request body deterministically. */
export function toQueryVariables(
  resolved: ReadonlyArray<ResolvedVariable>,
): QueryVariable[] {
  return resolved
    .map((v) => ({ name: v.name, values: [...v.current] }))
    .filter((v) => v.values.length > 0);
}
