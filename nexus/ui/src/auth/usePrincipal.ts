import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { getMe } from "@/api/me/get";
import type { MeResponse } from "@/api/types";

export const ME_QUERY_KEY = ["nexus", "me"] as const;

// The current Nexus principal — subject, role, scopes, teams, tenant —
// from `GET /api/v1/me`. One cached query the whole app reads; `useCan`
// derives authorization from it. Returns the full query result so call
// sites can branch on loading/error (F0: no fabricated principal).
export function usePrincipal(): UseQueryResult<MeResponse> {
  const client = useStarterClient();
  return useQuery({
    queryKey: ME_QUERY_KEY,
    queryFn: () => getMe(client),
    // The principal rarely changes within a session; keep it warm so
    // `useCan` checks don't refetch on every gate.
    staleTime: 5 * 60_000,
  });
}
