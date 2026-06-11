import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { NotifyEvent } from "@/api/types";

// Notification history — each event is a finding transition (opened/resolved)
// the runner tried to deliver, with its value and whether it was
// silenced/notified. Read-only: `GET /api/v1/detections/notify-events`.
export function listNotifyEvents(
  client: StarterClient,
): Promise<NotifyEvent[]> {
  return fetchJson<NotifyEvent[]>(
    client,
    `${client.apiPrefix}/detections/notify-events`,
  );
}
