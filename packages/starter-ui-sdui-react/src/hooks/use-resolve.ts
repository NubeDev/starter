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
    // Intentionally drop the per-observer `signal`. In React 19
    // `<StrictMode>` dev mode the first mount's signal aborts on
    // the synthetic unmount, which makes the in-flight resolve
    // request fail and forces the second mount to refetch — two
    // network requests for one render. Without the signal, the
    // second mount re-subscribes to the still-pending promise and
    // we see one request. Cancellation still happens implicitly
    // when the queryKey changes (page-state writes) — TanStack
    // Query GCs the old query and starts a new one.
    queryFn: () => transport.resolve(req),
    staleTime: 0,
  });
}
