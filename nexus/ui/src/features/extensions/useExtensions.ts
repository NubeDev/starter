import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { listExtensions } from "@/api/extensions/list";
import { getProcessStats } from "@/api/extensions/process";
import type { ExtensionSummary, ProcessStats } from "@/api/extensions/types";

export const EXTENSIONS_KEY = ["nexus", "extensions"] as const;

// The installed extensions, from `GET /extensions`. Returns the full query
// result so the screen renders loading/empty/error (F0).
export function useExtensions(): UseQueryResult<ExtensionSummary[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: EXTENSIONS_KEY,
    queryFn: () => listExtensions(client),
  });
}

// Live process stats for one extension, from `GET /extensions/{id}/process`.
// Gated by `enabled` so the fetch only fires when a row is expanded — avoids
// an N+1 burst on list load. `null` data means "no live process" (builtin/wasm
// or a stopped child), which the row renders as a muted placeholder. Refetches
// on an interval while open so uptime/CPU stay roughly live.
export function useProcessStats(
  id: string,
  enabled: boolean,
): UseQueryResult<ProcessStats | null> {
  const client = useStarterClient();
  return useQuery({
    queryKey: [...EXTENSIONS_KEY, id, "process"],
    queryFn: () => getProcessStats(client, id),
    enabled,
    refetchInterval: enabled ? 5000 : false,
  });
}
