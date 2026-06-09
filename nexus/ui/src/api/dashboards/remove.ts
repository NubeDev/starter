import { fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

// `DELETE /api/v1/dashboards/{slug}` — remove a dashboard (204).
export async function removeDashboard(
  client: StarterClient,
  slug: string,
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/dashboards/${encodeURIComponent(slug)}`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
}
