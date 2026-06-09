import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import {
  createAlertRule,
  listAlertRules,
  removeAlertRule,
} from "@/api/alerts/rules";
import {
  createChannel,
  listChannels,
  removeChannel,
} from "@/api/alerts/channels";
import {
  createSilence,
  listSilences,
  removeSilence,
} from "@/api/alerts/silences";
import { listAlertEvents } from "@/api/alerts/events";
import type {
  AlertEvent,
  AlertRuleDetail,
  ChannelDetail,
  CreateAlertRuleRequest,
  CreateChannelRequest,
  CreateSilenceRequest,
  SilenceDetail,
} from "@/api/types";

const KEY = {
  rules: ["nexus", "alerts", "rules"] as const,
  channels: ["nexus", "alerts", "channels"] as const,
  silences: ["nexus", "alerts", "silences"] as const,
  events: ["nexus", "alerts", "events"] as const,
};

// Each list is its own query; mutations invalidate the matching list so a
// create/delete refreshes immediately. All return the full query result so
// the tabs render loading/empty/error (F0).
export function useAlertRules(): UseQueryResult<AlertRuleDetail[]> {
  const client = useStarterClient();
  return useQuery({ queryKey: KEY.rules, queryFn: () => listAlertRules(client) });
}

export function useChannels(): UseQueryResult<ChannelDetail[]> {
  const client = useStarterClient();
  return useQuery({ queryKey: KEY.channels, queryFn: () => listChannels(client) });
}

export function useSilences(): UseQueryResult<SilenceDetail[]> {
  const client = useStarterClient();
  return useQuery({ queryKey: KEY.silences, queryFn: () => listSilences(client) });
}

export function useAlertEvents(): UseQueryResult<AlertEvent[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: KEY.events,
    queryFn: () => listAlertEvents(client),
    // History is append-only and read often; keep it briefly fresh.
    staleTime: 15_000,
  });
}

export function useRuleMutations() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  const invalidate = () => queryClient.invalidateQueries({ queryKey: KEY.rules });
  return {
    create: useMutation<AlertRuleDetail, Error, CreateAlertRuleRequest>({
      mutationFn: (body) => createAlertRule(client, body),
      onSuccess: invalidate,
    }),
    remove: useMutation<void, Error, string>({
      mutationFn: (id) => removeAlertRule(client, id),
      onSuccess: invalidate,
    }),
  };
}

export function useChannelMutations() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: KEY.channels });
  return {
    create: useMutation<ChannelDetail, Error, CreateChannelRequest>({
      mutationFn: (body) => createChannel(client, body),
      onSuccess: invalidate,
    }),
    remove: useMutation<void, Error, string>({
      mutationFn: (id) => removeChannel(client, id),
      onSuccess: invalidate,
    }),
  };
}

export function useSilenceMutations() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: KEY.silences });
  return {
    create: useMutation<SilenceDetail, Error, CreateSilenceRequest>({
      mutationFn: (body) => createSilence(client, body),
      onSuccess: invalidate,
    }),
    remove: useMutation<void, Error, string>({
      mutationFn: (id) => removeSilence(client, id),
      onSuccess: invalidate,
    }),
  };
}
