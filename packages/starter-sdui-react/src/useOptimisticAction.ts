/**
 * `useOptimisticAction` — the dispatch helper that wires
 * `Action.optimistic: { target_component_id, fields }` into the
 * round-trip per SCOPE.md § R9.
 *
 * Flow per fire:
 *
 *   1. Snapshot the React-Query cache entry at `treeQueryKey`.
 *   2. If an `optimistic` hint is provided, write the merged tree
 *      back through `mergeAt` so the UI updates **before** the
 *      `POST /api/v1/ui/action` fires.
 *   3. `await dispatchAction(handler, args)`.
 *   4. On success, run `interpret(res)`. A `patch` / `full_render`
 *      reply goes through the same `mergeAt` / replace path; a
 *      `noop` / `toast` / `navigate` / `dialog` reply leaves the
 *      optimistic write in place as the new authoritative state.
 *      A `diagnostics` reply is propagated to the caller — forms
 *      handle it inline, buttons surface the first error via the
 *      shared interpreter; in both cases the optimistic write is
 *      **rolled back** to the pre-dispatch snapshot because a
 *      diagnostics response means the server rejected the action.
 *   5. On thrown error from `dispatchAction`, restore the snapshot.
 *
 * Returns:
 *
 *   `dispatch(handler, args?, optimistic?) → Promise<UiActionResponse>`
 *
 * The returned response is the **authoritative** server reply;
 * callers do not need to call `interpret` themselves unless they
 * want to suppress a specific variant (forms intercept
 * `diagnostics` before re-interpreting).
 *
 * Rollback semantics
 * ------------------
 * Snapshot/restore is done via direct `setQueryData` — pointer
 * identity on unaffected branches is preserved by `mergeAt` so a
 * rollback after a no-op interpret cycle remains O(1) at the React
 * level. There is no per-key tombstone tracking: a second
 * optimistic dispatch fired before the first settles re-snapshots
 * the post-first-write state; the user-visible effect is "if the
 * second fails, you keep the first." That matches Rubix's S6
 * behaviour and is documented at the SCOPE R9 level.
 */
import type { QueryClient, QueryKey } from "@tanstack/react-query";
import { useQueryClient } from "@tanstack/react-query";
import { useSdui } from "./context.js";
import { mergeAt } from "./applyPatch.js";
import { useActionResponse } from "./useActionResponse.js";
import type { ActionFn } from "./context.js";
import type {
  OptimisticHint,
  UiActionResponse,
  UiComponentTree,
} from "./types.js";

export type OptimisticDispatch = (
  handler: string,
  args?: unknown,
  optimistic?: OptimisticHint | null,
) => Promise<UiActionResponse>;

/**
 * Pure helper — exported for tests. Drives the snapshot /
 * apply-optimistic / dispatch / interpret-or-rollback sequence in
 * one place so the React hook below is a thin wrapper.
 */
export async function runOptimisticDispatch(
  args: {
    qc: QueryClient;
    treeQueryKey: QueryKey;
    dispatchAction: ActionFn;
    interpret: (res: UiActionResponse) => void;
  },
  handler: string,
  actionArgs: unknown,
  optimistic: OptimisticHint | null | undefined,
): Promise<UiActionResponse> {
  const { qc, treeQueryKey, dispatchAction, interpret } = args;
  const snapshot = qc.getQueryData<unknown>(treeQueryKey);
  let applied = false;

  if (optimistic) {
    qc.setQueryData<unknown>(treeQueryKey, (prev: unknown) => {
      const tree = readTree(prev);
      if (!tree) return prev;
      return writeTree(
        prev,
        mergeAt(tree, optimistic.target_component_id, optimistic.fields),
      );
    });
    applied = true;
  }

  let res: UiActionResponse;
  try {
    res = await dispatchAction(handler, actionArgs);
  } catch (err) {
    if (applied) qc.setQueryData(treeQueryKey, snapshot);
    throw err;
  }

  if (res.type === "diagnostics") {
    if (applied) qc.setQueryData(treeQueryKey, snapshot);
    interpret(res);
    return res;
  }

  interpret(res);
  return res;
}

export function useOptimisticAction(): OptimisticDispatch {
  const qc = useQueryClient();
  const { dispatchAction, treeQueryKey } = useSdui();
  const interpret = useActionResponse();

  return (handler, args, optimistic) =>
    runOptimisticDispatch(
      { qc, treeQueryKey, dispatchAction, interpret },
      handler,
      args,
      optimistic,
    );
}

function readTree(cached: unknown): UiComponentTree | null {
  if (!cached || typeof cached !== "object") return null;
  if ("render" in cached) {
    const c = cached as { render?: UiComponentTree };
    return c.render ?? null;
  }
  if ("ir_version" in cached && "root" in cached) {
    return cached as UiComponentTree;
  }
  return null;
}

function writeTree(prev: unknown, tree: UiComponentTree): unknown {
  if (!prev || typeof prev !== "object") return prev;
  if ("render" in prev) {
    return { ...(prev as object), render: tree };
  }
  return tree;
}
