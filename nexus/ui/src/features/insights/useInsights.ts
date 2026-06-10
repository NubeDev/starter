import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { listInsights } from "@/api/insights/list";
import type { InsightSummary } from "@/api/types";

// The caller's tenant-scoped insights — reusable Rhai transforms applied to
// query results. Returns the full query result so the UI can render
// loading/empty/error (F0 — no placeholder data).
export function useInsights(): UseQueryResult<InsightSummary[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: ["nexus", "insights"],
    queryFn: () => listInsights(client),
    staleTime: 60_000,
  });
}
