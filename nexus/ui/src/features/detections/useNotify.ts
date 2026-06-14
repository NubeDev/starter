import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import {
  createChannel,
  listChannels,
  removeChannel,
} from "@/api/notify/channels";
import {
  createSilence,
  listSilences,
  removeSilence,
} from "@/api/notify/silences";
import { listNotifyEvents } from "@/api/notify/events";
import type {
  ChannelDetail,
  CreateChannelRequest,
  CreateSilenceRequest,
  NotifyEvent,
  SilenceDetail,
} from "@/api/types";

// Notification delivery for alert-type detections: channels, silences, and the
// notify-event history. Each list is its own query; mutations invalidate the
// matching list so a create/delete refreshes immediately. All return the full
// query result so the tabs render loading/empty/error (F0).
const KEY = {
  channels: ["nexus", "notify", "channels"] as const,
  silences: ["nexus", "notify", "silences"] as const,
  events: ["nexus", "notify", "events"] as const,
};

export function useChannels(): UseQueryResult<ChannelDetail[]> {
  const client = useStarterClient();
  return useQuery({ queryKey: KEY.channels, queryFn: () => listChannels(client) });
}

export function useSilences(): UseQueryResult<SilenceDetail[]> {
  const client = useStarterClient();
  return useQuery({ queryKey: KEY.silences, queryFn: () => listSilences(client) });
}

export function useNotifyEvents(): UseQueryResult<NotifyEvent[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: KEY.events,
    queryFn: () => listNotifyEvents(client),
    // History is append-only and read often; keep it briefly fresh.
    staleTime: 15_000,
  });
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
