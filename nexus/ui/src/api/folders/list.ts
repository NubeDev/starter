import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { FolderSummary } from "@/api/types";

// `GET /api/v1/folders` — the caller's tenant-scoped folders as a flat list.
// The client assembles the tree from each folder's `parent_id` (null = root).
export function listFolders(client: StarterClient): Promise<FolderSummary[]> {
  return fetchJson<FolderSummary[]>(client, `${client.apiPrefix}/folders`);
}
