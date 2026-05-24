// `useUserList` / `useUserCreate` / `useUserDisable` — read + write
// hooks for the `rubix.user.*` tool family.
//
// The read hook is a `useQuery` against `client.userList()`; the two
// write hooks are `useMutation`s that invalidate the `['rubix',
// 'users']` query prefix on success so a re-list fires automatically.
// All three rely on `RubixClient` methods which already thread the
// CSRF header through `readCsrfHeader()`.

import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationOptions,
  type UseMutationResult,
  type UseQueryOptions,
  type UseQueryResult,
} from "@tanstack/react-query";

import type {
  UserCreateRequest,
  UserCreateResponse,
  UserDisableRequest,
  UserDisableResponse,
  UserListRequest,
  UserListResponse,
} from "@nube/rubix-client-ts";
import type { StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const USERS_KEY = ["rubix", "users"] as const;

type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;
type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;

/** List all users. Query key: `['rubix','users','list']`. */
export function useUserList(
  request: UserListRequest = {},
  options?: ReadOptions<UserListResponse>,
): UseQueryResult<UserListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<UserListResponse, StarterError>({
    queryKey: [...USERS_KEY, "list"],
    queryFn: () => client.userList(request),
    ...options,
  });
}

/**
 * Create a user. On success invalidates the `['rubix','users']`
 * prefix so any active `useUserList` re-fetches.
 */
export function useUserCreate(
  options?: WriteOptions<UserCreateRequest, UserCreateResponse>,
): UseMutationResult<UserCreateResponse, StarterError, UserCreateRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<UserCreateResponse, StarterError, UserCreateRequest>({
    mutationFn: (request) => client.userCreate(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: USERS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Disable an existing user. Invalidates the users prefix on success. */
export function useUserDisable(
  options?: WriteOptions<UserDisableRequest, UserDisableResponse>,
): UseMutationResult<UserDisableResponse, StarterError, UserDisableRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<UserDisableResponse, StarterError, UserDisableRequest>({
    mutationFn: (request) => client.userDisable(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: USERS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}
