import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { createFlow } from "@/api/flows/create";
import { listFlows } from "@/api/flows/list";
import { removeFlow } from "@/api/flows/remove";
import { startFlow, stopFlow } from "@/api/flows/lifecycle";
import type { CreateFlowRequest, FlowDetail, FlowSummary } from "@/api/types";

const FLOWS_KEY = ["nexus", "flows"] as const;

// The tenant's saved flows with their enabled/running state. Returns the
// full query result so the screen renders loading/empty/error (F0).
export function useFlows(): UseQueryResult<FlowSummary[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: FLOWS_KEY,
    queryFn: () => listFlows(client),
    // Running state changes server-side (FlowManager), so keep it fresh.
    staleTime: 10_000,
  });
}

// Start / stop / delete, each refreshing the list so the running pill and
// row set reflect the new state.
export function useFlowActions() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: FLOWS_KEY });

  const start = useMutation<FlowDetail, Error, string>({
    mutationFn: (id) => startFlow(client, id),
    onSuccess: invalidate,
  });
  const stop = useMutation<FlowDetail, Error, string>({
    mutationFn: (id) => stopFlow(client, id),
    onSuccess: invalidate,
  });
  const remove = useMutation<void, Error, string>({
    mutationFn: (id) => removeFlow(client, id),
    onSuccess: invalidate,
  });

  return { start, stop, remove };
}

// Create a flow from an assembled request, then refresh the list.
export function useCreateFlow() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<FlowDetail, Error, CreateFlowRequest>({
    mutationFn: (body) => createFlow(client, body),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: FLOWS_KEY }),
  });
}
