import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { listAudit, type AuditFilter } from "@/api/audit/list";
import type { ChangePage } from "@/api/types";

export const AUDIT_KEY = ["nexus", "audit"] as const;

// The admin audit log, from `GET /audit`. The filter is part of the query key
// so changing a filter refetches the matching page. Returns the full query
// result so the screen renders loading/empty/error (F0).
export function useAudit(
  filter: AuditFilter = {},
): UseQueryResult<ChangePage> {
  const client = useStarterClient();
  return useQuery({
    queryKey: [...AUDIT_KEY, filter],
    queryFn: () => listAudit(client, filter),
  });
}
