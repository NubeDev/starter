import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { listFolders } from "@/api/folders/list";
import type { FolderSummary } from "@/api/types";

export const FOLDERS_KEY = ["nexus", "folders"] as const;

// The dashboard organisation tree, from `GET /folders`, as a flat list the
// sidebar nests by `parent_id`. Returns the full query result so the sidebar
// renders loading/empty/error consistently with the dashboard list (WS-05).
export function useFolders(): UseQueryResult<FolderSummary[]> {
  const client = useStarterClient();
  return useQuery({
    queryKey: FOLDERS_KEY,
    queryFn: () => listFolders(client),
  });
}
