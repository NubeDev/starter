// React-query hooks for agent CRUD. Mirrors the datasources/dashboards hook
// style: tuple query keys under the "nexus" namespace, `useStarterClient()` for
// the client, and mutations that invalidate the list on success.
import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import {
  createAgent,
  deleteAgent,
  getAgent,
  listAgents,
  updateAgent,
} from "@/api/agents";
import type {
  AgentDetail,
  AgentSummary,
  CreateAgentRequest,
  UpdateAgentRequest,
} from "@/api/types";

export const AGENTS_KEY = ["nexus", "agents"] as const;
export const agentKey = (id: string) => ["nexus", "agent", id] as const;

/** List the caller's agents. */
export function useAgents(): UseQueryResult<AgentSummary[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: AGENTS_KEY,
    queryFn: () => listAgents(client),
  });
}

/** One agent in full (config, system prompt). */
export function useAgent(id: string | undefined): UseQueryResult<AgentDetail> {
  const client = useStarterClient();
  return useQuery({
    queryKey: agentKey(id ?? ""),
    enabled: !!id,
    queryFn: () => getAgent(client, id!),
  });
}

export function useCreateAgent() {
  const client = useStarterClient();
  const qc = useQueryClient();
  return useMutation<AgentDetail, Error, CreateAgentRequest>({
    mutationFn: (body) => createAgent(client, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: AGENTS_KEY }),
  });
}

export function useUpdateAgent() {
  const client = useStarterClient();
  const qc = useQueryClient();
  return useMutation<AgentDetail, Error, { id: string; patch: UpdateAgentRequest }>({
    mutationFn: ({ id, patch }) => updateAgent(client, id, patch),
    onSuccess: (_data, { id }) => {
      qc.invalidateQueries({ queryKey: AGENTS_KEY });
      qc.invalidateQueries({ queryKey: agentKey(id) });
    },
  });
}

export function useDeleteAgent() {
  const client = useStarterClient();
  const qc = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: (id) => deleteAgent(client, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: AGENTS_KEY }),
  });
}
