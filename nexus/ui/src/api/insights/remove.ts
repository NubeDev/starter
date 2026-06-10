import { fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

// `DELETE /api/v1/insights/{id}` — remove an insight (204). Named `remove`
// rather than `delete` (a reserved word) per the verb-per-file layout.
export async function removeInsight(
  client: StarterClient,
  id: string,
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/insights/${encodeURIComponent(id)}`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
}
