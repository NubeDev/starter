import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { listDatasources } from "@/api/datasources/list";
import type { DatasourceSummary } from "@/api/types";

// The caller's tenant-scoped datasources, for the picker in the query
// editor and panel config. Returns the full query result so the UI can
// render loading/empty/error (F0 — no placeholder datasource).
export function useDatasources(): UseQueryResult<DatasourceSummary[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: ["nexus", "datasources"],
    queryFn: () => listDatasources(client),
    staleTime: 60_000,
  });
}
