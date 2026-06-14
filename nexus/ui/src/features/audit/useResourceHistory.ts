import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import {
  resourceHistory,
  type ResourceHistoryParams,
} from "@/api/audit/resourceHistory";
import type { ChangePage } from "@/api/types";

export const RESOURCE_HISTORY_KEY = ["nexus", "audit", "resource"] as const;

// One resource's change history, from `GET /audit/resources/{kind}/{id}`.
// Powers a "History" tab on a dashboard/datasource. `enabled` lets the caller
// defer the fetch until the tab is opened. Returns the full query result so the
// tab renders loading/empty/error (F0).
export function useResourceHistory(
  kind: string,
  id: string,
  params: ResourceHistoryParams = {},
  enabled = true,
): UseQueryResult<ChangePage> {
  const client = useStarterClient();
  return useQuery({
    queryKey: [...RESOURCE_HISTORY_KEY, kind, id, params],
    queryFn: () => resourceHistory(client, kind, id, params),
    enabled: enabled && Boolean(kind) && Boolean(id),
  });
}
