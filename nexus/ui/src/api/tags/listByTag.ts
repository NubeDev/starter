import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { TaggableKind, TaggedEntity } from "@/api/types";

// `GET /api/v1/tags/entities/{kind}?key=…&value=…` — entities of a kind
// carrying a tag. `value` optional: omit to match any value for the key,
// supply to pin it exactly. Powers tag-filtered listings.
export function listEntitiesWithTag(
  client: StarterClient,
  kind: TaggableKind,
  key: string,
  value?: string,
): Promise<TaggedEntity[]> {
  const params = new URLSearchParams({ key });
  if (value !== undefined) params.set("value", value);
  return fetchJson<TaggedEntity[]>(
    client,
    `${client.apiPrefix}/tags/entities/${kind}?${params.toString()}`,
  );
}
