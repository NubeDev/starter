import { fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

// `DELETE /api/v1/panels/{id}` — remove a panel from its dashboard (204).
// Keyed by panel id, not dashboard slug — a panel id is globally unique.
export async function removePanel(
  client: StarterClient,
  panelId: string,
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/panels/${encodeURIComponent(panelId)}`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
}
