import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { FolderSummary, UpdateFolderRequest } from "@/api/types";

// `PATCH /api/v1/folders/{id}` — rename or reparent a folder. The parent is
// three-valued: send `parent_id` to move under a folder, `clear_parent: true`
// to re-root, or neither to leave it unchanged.
export function updateFolder(
  client: StarterClient,
  id: string,
  request: UpdateFolderRequest,
): Promise<FolderSummary> {
  return fetchJson<FolderSummary>(
    client,
    `${client.apiPrefix}/folders/${encodeURIComponent(id)}`,
    {
      method: "PATCH",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}
