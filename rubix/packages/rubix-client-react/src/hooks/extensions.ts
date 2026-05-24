// Extension admin hooks — list + lifecycle mutations.
//
// The matching typed methods on `RubixClient` (`extensionsList`,
// `extensionsStart`, …) are scheduled to land via the
// `rubix-client-ts` extensions endpoint module. Until that ships
// these hooks talk to rubix-agent directly through `fetchJson` /
// `readCsrfHeader` on the wrapped starter client. The hook shapes
// match the eventual typed-method API so swapping the call sites is
// a single-line change per hook.

import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationOptions,
  type UseMutationResult,
  type UseQueryOptions,
  type UseQueryResult,
} from "@tanstack/react-query";

import { fetchJson, readCsrfHeader, type StarterError } from "@nube/starter-client-ts";

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const EXTENSIONS_KEY = ["rubix", "extensions"] as const;

export interface ExtensionSummary {
  id: string;
  name: string;
  enabled: boolean;
  state: "running" | "stopped" | "starting" | "stopping" | "errored";
  last_error?: string | null;
}

export interface ExtensionListResponse {
  extensions: ExtensionSummary[];
}

/** `POST /api/v1/extensions/{id}/{action}` — empty response on success. */
export interface ExtensionMutationVars {
  id: string;
}

type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;
type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;

/** List installed extensions. Query key: `['rubix','extensions','list']`. */
export function useExtensionsList(
  options?: ReadOptions<ExtensionListResponse>,
): UseQueryResult<ExtensionListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<ExtensionListResponse, StarterError>({
    queryKey: [...EXTENSIONS_KEY, "list"],
    queryFn: () => fetchJson<ExtensionListResponse>(client.starter, "/api/v1/extensions"),
    ...options,
  });
}

function makeAction(action: "start" | "stop" | "restart" | "enable" | "disable") {
  return function useExtensionAction(
    options?: WriteOptions<ExtensionMutationVars, void>,
  ): UseMutationResult<void, StarterError, ExtensionMutationVars> {
    // eslint-disable-next-line react-hooks/rules-of-hooks
    const client = useRubixClient();
    // eslint-disable-next-line react-hooks/rules-of-hooks
    const qc = useQueryClient();
    // eslint-disable-next-line react-hooks/rules-of-hooks
    return useMutation<void, StarterError, ExtensionMutationVars>({
      mutationFn: async ({ id }) => {
        await fetchJson<unknown>(
          client.starter,
          `/api/v1/extensions/${encodeURIComponent(id)}/${action}`,
          {
            method: "POST",
            headers: { "content-type": "application/json", ...readCsrfHeader() },
            body: "{}",
          },
        );
      },
      ...options,
      onSuccess: async (...args) => {
        await qc.invalidateQueries({ queryKey: EXTENSIONS_KEY });
        await options?.onSuccess?.(...args);
      },
    });
  };
}

export const useExtensionStart = makeAction("start");
export const useExtensionStop = makeAction("stop");
export const useExtensionRestart = makeAction("restart");
export const useExtensionEnable = makeAction("enable");
export const useExtensionDisable = makeAction("disable");
