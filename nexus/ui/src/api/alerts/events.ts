import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { AlertEvent } from "@/api/types";

// Fired-alert history — each event is a rule transition (firing/resolved)
// with its value and whether it was silenced/notified. Read-only:
// `GET /api/v1/alerts/events`.
export function listAlertEvents(client: StarterClient): Promise<AlertEvent[]> {
  return fetchJson<AlertEvent[]>(client, `${client.apiPrefix}/alerts/events`);
}
