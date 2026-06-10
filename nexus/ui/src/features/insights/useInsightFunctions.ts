import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { listInsightFunctions } from "@/api/insights/functions";
import type { InsightFunctionDoc } from "@/api/types";

// The curated insight-function catalogue. It is effectively static for a
// session (the surface only changes when the server is redeployed), so cache it
// forever (`staleTime: Infinity`). Powers both the cheatsheet and the editor's
// function-name autocomplete.
export function useInsightFunctions(): UseQueryResult<InsightFunctionDoc[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: ["nexus", "insights", "functions"],
    queryFn: () => listInsightFunctions(client),
    staleTime: Infinity,
  });
}
