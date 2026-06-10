import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { InstallResponse } from "@/api/extensions/types";

// `POST /api/v1/extensions/install` — multipart upload of a `.tar.gz`
// bundle under the `file` field. No explicit `content-type` header: the
// browser sets the multipart boundary itself. The new extension only goes
// live after a server restart (`pending_restart: true`).
export function installExtension(
  client: StarterClient,
  file: File,
): Promise<InstallResponse> {
  const form = new FormData();
  form.append("file", file);
  return fetchJson<InstallResponse>(
    client,
    `${client.apiPrefix}/extensions/install`,
    { method: "POST", headers: readCsrfHeader(), body: form },
  );
}
