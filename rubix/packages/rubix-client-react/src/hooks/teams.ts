// `useTeamCreate` / `useTeamAssign` — write hooks for the
// `rubix.team.*` tool family.
//
// Both are mutations; on success they invalidate the
// `['rubix','teams']` prefix so any co-mounted team list (added in
// a later stage) re-fetches. Mirrors the rubix-client-ts shape and
// inherits the CSRF cookie wiring from `RubixClient.team*`.

import {
  useMutation,
  useQueryClient,
  type UseMutationOptions,
  type UseMutationResult,
} from "@tanstack/react-query";

import type {
  TeamAssignRequest,
  TeamAssignResponse,
  TeamCreateRequest,
  TeamCreateResponse,
} from "@nube/rubix-client-ts";
import type { StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const TEAMS_KEY = ["rubix", "teams"] as const;

type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;

/** Create a team. Invalidates the `['rubix','teams']` prefix on success. */
export function useTeamCreate(
  options?: WriteOptions<TeamCreateRequest, TeamCreateResponse>,
): UseMutationResult<TeamCreateResponse, StarterError, TeamCreateRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<TeamCreateResponse, StarterError, TeamCreateRequest>({
    mutationFn: (request) => client.teamCreate(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: TEAMS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Assign a user to a team. Invalidates the teams prefix on success. */
export function useTeamAssign(
  options?: WriteOptions<TeamAssignRequest, TeamAssignResponse>,
): UseMutationResult<TeamAssignResponse, StarterError, TeamAssignRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<TeamAssignResponse, StarterError, TeamAssignRequest>({
    mutationFn: (request) => client.teamAssign(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: TEAMS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}
