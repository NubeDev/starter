// TanStack-Query wrapper around `transport.resolve()`. Keyed by the
// (page_ref, target_ref, stack, page_state) tuple so a page-state
// write re-fetches without any imperative refetch call.

import { useQuery } from "@tanstack/react-query";
import type { UseQueryResult } from "@tanstack/react-query";
import type {
  ClientCapabilities,
  ResolveRequest,
  UiResolveResponse,
} from "@nube/starter-ui-ir";
import { useSduiTransport } from "../provider/sdui-provider.js";
import { usePageState } from "../page-state.js";

export interface UseSduiResolveOptions {
  pageRef: string;
  targetRef?: string;
  stack?: Record<string, string>;
  capabilities?: ClientCapabilities;
}

export function useSduiResolve(
  opts: UseSduiResolveOptions,
): UseQueryResult<UiResolveResponse, Error> {
  const transport = useSduiTransport();
  const [pageState] = usePageState();
  const req: ResolveRequest = {
    page_ref: opts.pageRef,
    target_ref: opts.targetRef,
    stack: opts.stack,
    page_state: pageState,
    capabilities: opts.capabilities,
  };
  return useQuery<UiResolveResponse, Error>({
    queryKey: ["sdui", "resolve", req],
    queryFn: ({ signal }) => transport.resolve(req, signal),
    staleTime: 0,
  });
}
