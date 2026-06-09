import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { CreateFolderRequest, FolderSummary } from "@/api/types";

// `POST /api/v1/folders` — create a folder (400 if the parent is absent or in
// another tenant). Omit `parent_id` for a root folder.
export function createFolder(
  client: StarterClient,
  request: CreateFolderRequest,
): Promise<FolderSummary> {
  return fetchJson<FolderSummary>(client, `${client.apiPrefix}/folders`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
