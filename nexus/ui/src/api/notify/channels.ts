import { fetchJson, fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { ChannelDetail, CreateChannelRequest } from "@/api/types";

// Notification channels — `{name, kind, config}`; detections reference them by
// id. `GET/POST /api/v1/detections/channels`, `DELETE …/{id}`.
export function listChannels(client: StarterClient): Promise<ChannelDetail[]> {
  return fetchJson<ChannelDetail[]>(
    client,
    `${client.apiPrefix}/detections/channels`,
  );
}

export function createChannel(
  client: StarterClient,
  request: CreateChannelRequest,
): Promise<ChannelDetail> {
  return fetchJson<ChannelDetail>(
    client,
    `${client.apiPrefix}/detections/channels`,
    {
      method: "POST",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}

export async function removeChannel(
  client: StarterClient,
  id: string,
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/detections/channels/${encodeURIComponent(id)}`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
}
