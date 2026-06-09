import { fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { UpdateFlowRequest } from "@/api/types";

// `PUT /api/v1/flows/{id}` — update a flow's name, enabled flag, or
// ArkFlow config. Partial: omitted fields are left unchanged (204).
export async function updateFlow(
  client: StarterClient,
  id: string,
  request: UpdateFlowRequest,
): Promise<void> {
  await fetchVoid(client, `${client.apiPrefix}/flows/${encodeURIComponent(id)}`, {
    method: "PUT",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
