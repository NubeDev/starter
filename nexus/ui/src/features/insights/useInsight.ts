import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { getInsight } from "@/api/insights/get";
import type { InsightSummary } from "@/api/types";

// Full detail for one saved insight (id, name, script, params schema), fetched
// on demand. Used by the Workbench when opened in edit mode (`?id=…`) to
// pre-fill the editor. Disabled when `id` is undefined so the "new insight"
// path makes no request. Returns the whole query result so the caller can
// render loading / error (F0 — no placeholder data).
export function useInsight(
  id: string | undefined,
): UseQueryResult<InsightSummary> {
  const client = useStarterClient();
  return useQuery({
    queryKey: ["nexus", "insights", id],
    queryFn: () => getInsight(client, id as string),
    enabled: !!id,
    staleTime: 60_000,
  });
}
