import { useMutation, useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { listNodeTypes } from "@/api/flows/nodeTypes";
import { dryRunFlow } from "@/api/flows/dryRun";
import type { DryRunRequest, DryRunResponse, NodeType } from "@/api/types";

const NODE_TYPES_KEY = ["nexus", "flows", "node-types"] as const;

// The flow-builder palette. The node set changes only when the server's
// registered connectors change, so it is effectively static for a session.
export function useNodeTypes(): UseQueryResult<NodeType[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: NODE_TYPES_KEY,
    queryFn: async () => (await listNodeTypes(client)).node_types,
    staleTime: Infinity,
  });
}

// Run a bounded dry-run of the current graph. A build/runtime error comes back
// in the response's `error` field (a 200), so the only mutation error is the
// request itself failing.
export function useDryRun() {
  const client = useStarterClient();
  return useMutation<DryRunResponse, Error, DryRunRequest>({
    mutationFn: (request) => dryRunFlow(client, request),
  });
}
