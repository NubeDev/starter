import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { getUserSettings, putUserSettings } from "@/api/me/settings";
import type { UserSettings } from "@/api/types";

export const USER_SETTINGS_KEY = ["nexus", "me", "settings"] as const;

// The caller's freeform settings bag from `GET /api/v1/me/settings`. One cached
// query the whole app reads; feature hooks (e.g. starred dashboards) derive
// their slice from it. Returns the full query result so call sites can branch
// on loading/error rather than assume a bag.
export function useUserSettings(): UseQueryResult<UserSettings> {
  const client = useStarterClient();
  return useQuery({
    queryKey: USER_SETTINGS_KEY,
    queryFn: () => getUserSettings(client),
    // Personal UI state rarely changes from under us within a session.
    staleTime: 5 * 60_000,
  });
}

// The settings bag as a plain object, defaulting to `{}` while loading or on
// error — callers reading a single key never need to special-case the envelope.
export function useSettingsBag(): Record<string, unknown> {
  const { data } = useUserSettings();
  const bag = data?.settings;
  return bag && typeof bag === "object" && !Array.isArray(bag)
    ? (bag as Record<string, unknown>)
    : {};
}

// Save a transformed settings bag (full replace). The transform receives the
// current bag and returns the next one, so callers express an edit without
// racing on a stale read. Optimistically updates the cache, rolls back on
// error, and reconciles with the server's response.
export function useSaveUserSettings() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (next: Record<string, unknown>) =>
      putUserSettings(client, { settings: next }),
    onMutate: async (next) => {
      await queryClient.cancelQueries({ queryKey: USER_SETTINGS_KEY });
      const previous = queryClient.getQueryData<UserSettings>(USER_SETTINGS_KEY);
      queryClient.setQueryData<UserSettings>(USER_SETTINGS_KEY, {
        settings: next,
      });
      return { previous };
    },
    onError: (_err, _next, context) => {
      if (context?.previous) {
        queryClient.setQueryData(USER_SETTINGS_KEY, context.previous);
      }
    },
    onSuccess: (saved) => {
      queryClient.setQueryData(USER_SETTINGS_KEY, saved);
    },
  });
}
