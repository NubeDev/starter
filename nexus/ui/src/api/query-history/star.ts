import { fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { StarQueryRequest } from "@/api/types";

// `POST /api/v1/query-history/{id}/star` — pin or unpin a past run. A starred
// row sorts to the top of the recall drawer and is exempt from the rolling
// retention window, so a favourite query survives.
export async function starQueryHistory(
  client: StarterClient,
  id: string,
  request: StarQueryRequest,
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/query-history/${encodeURIComponent(id)}/star`,
    {
      method: "POST",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}
