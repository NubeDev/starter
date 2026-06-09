import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { Tag, TaggableKind } from "@/api/types";

// `GET /api/v1/tags/{kind}/{id}` — the tags on one entity. A tag is a key
// with an optional value (a bare label has no value).
export function getTags(
  client: StarterClient,
  kind: TaggableKind,
  id: string,
): Promise<Tag[]> {
  return fetchJson<Tag[]>(
    client,
    `${client.apiPrefix}/tags/${kind}/${encodeURIComponent(id)}`,
  );
}
