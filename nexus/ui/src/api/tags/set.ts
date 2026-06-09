import { fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { SetTagsRequest, TaggableKind } from "@/api/types";

// `PUT /api/v1/tags/{kind}/{id}` — replace an entity's full tag set (204).
// A full replace, not a delta: send the complete set to persist; tags not
// listed are removed.
export async function setTags(
  client: StarterClient,
  kind: TaggableKind,
  id: string,
  request: SetTagsRequest,
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/tags/${kind}/${encodeURIComponent(id)}`,
    {
      method: "PUT",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}
