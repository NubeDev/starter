import { fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

// `DELETE /api/v1/nav/{id}` — remove a nav node (204); its children re-root
// (WS-13 §4). Named `remove` rather than `delete` (a reserved word) per the
// verb-per-file layout.
export async function removeNavNode(
  client: StarterClient,
  id: string,
): Promise<void> {
  await fetchVoid(client, `${client.apiPrefix}/nav/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: readCsrfHeader(),
  });
}
