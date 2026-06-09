import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { listDashboards } from "@/api/dashboards/list";
import type { DashboardSummary } from "@/api/types";

export const DASHBOARDS_KEY = ["nexus", "dashboards"] as const;

// The sidebar's dashboard list, from `GET /dashboards`. Returns the full
// query result so the sidebar renders loading/empty/error (F0).
export function useDashboards(): UseQueryResult<DashboardSummary[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: DASHBOARDS_KEY,
    queryFn: () => listDashboards(client),
  });
}
