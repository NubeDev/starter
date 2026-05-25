// `useFlowList` / `useFlowLint` / `useFlowDeploy` / `useFlowDuplicate`
// — hooks for the `rubix.flow_ops.*` tool family.
//
// `flowList` is a `useQuery`; the rest are mutations. All four go
// through the same `/api/v1/tools/*` POST transport, so `lint` and
// `list` thread CSRF for symmetry. Mutations invalidate the shared
// `['rubix','flow_ops']` prefix on success.

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
  FlowDeployRequest,
  FlowDeployResponse,
  FlowDuplicateRequest,
  FlowDuplicateResponse,
  FlowKindsRequest,
  FlowKindsResponse,
  FlowLintRequest,
  FlowLintResponse,
  FlowListRequest,
  FlowListResponse,
} from "@nube/rubix-client-ts";
import type { StarterError } from "@nube/starter-client-ts";

/**
 * Structural mirror of `@nube/starter-ui-flow`'s `FlowGraph`.
 * Defined inline so this transport package does not take a hard
 * dependency on the UI package. The shape is assignable to
 * `import("@nube/starter-ui-flow").FlowGraph` at the call site.
 */
export interface FlowGraph {
  nodes: Array<{
    id: string;
    kind: string;
    position: { x: number; y: number };
    label?: string;
    data?: Record<string, unknown>;
  }>;
  edges: Array<{
    id: string;
    source: string;
    sourceSlot: string;
    target: string;
    targetSlot: string;
  }>;
}

import { useRubixClient } from "../provider/rubix-client-provider.js";

export const FLOW_OPS_KEY = ["rubix", "flow_ops"] as const;

type ReadOptions<T> = Omit<UseQueryOptions<T, StarterError>, "queryKey" | "queryFn">;
type WriteOptions<TReq, TRes> = Omit<
  UseMutationOptions<TRes, StarterError, TReq>,
  "mutationFn"
>;

/** List deployed flows. Query key: `['rubix','flow_ops','list']`. */
export function useFlowList(
  request: FlowListRequest = {},
  options?: ReadOptions<FlowListResponse>,
): UseQueryResult<FlowListResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<FlowListResponse, StarterError>({
    queryKey: [...FLOW_OPS_KEY, "list"],
    queryFn: () => client.flowList(request),
    ...options,
  });
}

/**
 * List the node kinds the rubix flow runtime knows about.
 *
 * Result is cached under `['rubix','flow_ops','kinds']` — flat
 * because the response is process-static (kinds are registered at
 * boot from the `NodeKindRegistry`) so a single shared cache entry
 * is enough across every consumer that wants the palette / settings
 * sidebar.
 */
export function useFlowKinds(
  request: FlowKindsRequest = {},
  options?: ReadOptions<FlowKindsResponse>,
): UseQueryResult<FlowKindsResponse, StarterError> {
  const client = useRubixClient();
  return useQuery<FlowKindsResponse, StarterError>({
    queryKey: [...FLOW_OPS_KEY, "kinds"],
    queryFn: () => client.flowKinds(request),
    ...options,
  });
}

/**
 * Plural alias of `useFlowList`. Surfaced to match the consumer-side
 * naming used by route-level callers (e.g. `/flows/index.tsx`) so the
 * import reads naturally even though the underlying tool id is the
 * singular `rubix.flow_ops.list`.
 */
export const useFlowsList = useFlowList;

/**
 * `useFlowDefinition(flowId)` — read a single deployed flow's graph
 * for read-only rendering on `/flows/$flowId`.
 *
 * NOTE: rubix-agent currently has no `rubix.flow_ops.get` verb and
 * no `/api/v1/flows-definitions/<id>` HTTP route — `body_yaml` is
 * only addressable via the internal `FlowDefStore` SPI. Until that
 * lands (see stage 3 BLOCKED handover) this hook synthesises a
 * minimal `FlowGraph` from the `list` response so the
 * `<FlowCanvas readOnly>` mount on `/flows/$flowId` exercises the
 * registry + canvas + xyflow stylesheet end-to-end. Once the
 * backend exposes a body endpoint this hook gets the real
 * `client.flowGet(flowId)` call and a `yaml.parse(body_yaml)` step
 * — the consumer API (a `UseQueryResult<FlowGraph>`) stays stable.
 *
 * Query key: `['rubix','flow_ops','definition', flowId]`.
 */
export function useFlowDefinition(
  flowId: string,
  options?: ReadOptions<FlowGraphResult>,
): UseQueryResult<FlowGraphResult, StarterError> {
  const client = useRubixClient();
  return useQuery<FlowGraphResult, StarterError>({
    queryKey: [...FLOW_OPS_KEY, "definition", flowId],
    queryFn: async () => {
      const list = await client.flowList({});
      const item = list.flows.find((f) => f.flow_id === flowId);
      if (!item) {
        return {
          flow_id: flowId,
          revision_id: "",
          graph: { nodes: [], edges: [] },
          placeholder: true,
        };
      }
      // Placeholder graph: one ai-agent node naming the flow.
      // Replace with `yaml.parse(item.body_yaml)` once available.
      return {
        flow_id: item.flow_id,
        revision_id: item.revision_id,
        graph: {
          nodes: [
            {
              id: "root",
              kind: "ai-agent",
              position: { x: 240, y: 160 },
              label: item.flow_id,
              data: {
                skill_hint: `revision ${item.revision_id.slice(0, 8)}`,
                allowed_tools: [],
              },
            },
          ],
          edges: [],
        },
        placeholder: true,
      };
    },
    ...options,
  });
}

/** Return shape of `useFlowDefinition`. */
export interface FlowGraphResult {
  flow_id: string;
  revision_id: string;
  graph: FlowGraph;
  /** True while the backend body endpoint is missing — UI may show a banner. */
  placeholder?: boolean;
}

/**
 * Lint a flow body. Modelled as a mutation rather than a query because
 * the caller supplies fresh YAML per invocation; caching by YAML hash
 * would only confuse callers. Does NOT invalidate the prefix.
 */
export function useFlowLint(
  options?: WriteOptions<FlowLintRequest, FlowLintResponse>,
): UseMutationResult<FlowLintResponse, StarterError, FlowLintRequest> {
  const client = useRubixClient();
  return useMutation<FlowLintResponse, StarterError, FlowLintRequest>({
    mutationFn: (request) => client.flowLint(request),
    ...options,
  });
}

/** Deploy a flow revision. Invalidates the flow_ops prefix on success. */
export function useFlowDeploy(
  options?: WriteOptions<FlowDeployRequest, FlowDeployResponse>,
): UseMutationResult<FlowDeployResponse, StarterError, FlowDeployRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<FlowDeployResponse, StarterError, FlowDeployRequest>({
    mutationFn: (request) => client.flowDeploy(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: FLOW_OPS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}

/** Duplicate a flow to a new id. Invalidates the flow_ops prefix on success. */
export function useFlowDuplicate(
  options?: WriteOptions<FlowDuplicateRequest, FlowDuplicateResponse>,
): UseMutationResult<FlowDuplicateResponse, StarterError, FlowDuplicateRequest> {
  const client = useRubixClient();
  const qc = useQueryClient();
  return useMutation<FlowDuplicateResponse, StarterError, FlowDuplicateRequest>({
    mutationFn: (request) => client.flowDuplicate(request),
    ...options,
    onSuccess: async (...args) => {
      await qc.invalidateQueries({ queryKey: FLOW_OPS_KEY });
      await options?.onSuccess?.(...args);
    },
  });
}
