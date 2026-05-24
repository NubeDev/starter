// `useUndoLast` — hook for the `rubix.undo.last` tool.
//
// Mutating tool that reverses the most-recent undo group. On success
// every mutating family's prefix becomes stale, so we invalidate the
// whole `['rubix']` namespace — the cheapest correct answer for an
// admin-grade affordance that ought to fire rarely.

import {
  useMutation,
  useQueryClient,
  type UseMutationOptions,
  type UseMutationResult,
} from "@tanstack/react-query";

import type { UndoLastRequest, UndoLastResponse } from "@nube/rubix-client-ts";
import type { StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const UNDO_KEY = ["rubix", "undo"] as const;
export const RUBIX_ROOT_KEY = ["rubix"] as const;

type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;

/** Undo the most-recent undo group. Invalidates every `['rubix', ...]` query on success. */
export function useUndoLast(
  options?: WriteOptions<UndoLastRequest, UndoLastResponse>,
): UseMutationResult<UndoLastResponse, StarterError, UndoLastRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<UndoLastResponse, StarterError, UndoLastRequest>({
    mutationFn: (request = {}) => client.undoLast(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: RUBIX_ROOT_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}
