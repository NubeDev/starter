import { fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

// `DELETE /api/v1/flows/{id}` — remove a flow (204). The FlowManager stops
// it first if it's running.
export async function removeFlow(
  client: StarterClient,
  id: string,
): Promise<void> {
  await fetchVoid(client, `${client.apiPrefix}/flows/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: readCsrfHeader(),
  });
}
