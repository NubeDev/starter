import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { listExtensions } from "@/api/extensions/list";
import type { ExtensionSummary } from "@/api/extensions/types";

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
