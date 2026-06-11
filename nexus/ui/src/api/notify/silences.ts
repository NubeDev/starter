import { fetchJson, fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { CreateSilenceRequest, SilenceDetail } from "@/api/types";

// Silences — time windows that mute a detection (or all detections when
// `detection_id` is null). `GET/POST /api/v1/detections/silences`,
// `DELETE …/{id}`.
export function listSilences(client: StarterClient): Promise<SilenceDetail[]> {
  return fetchJson<SilenceDetail[]>(
    client,
    `${client.apiPrefix}/detections/silences`,
  );
}

export function createSilence(
  client: StarterClient,
  request: CreateSilenceRequest,
): Promise<SilenceDetail> {
  return fetchJson<SilenceDetail>(
    client,
    `${client.apiPrefix}/detections/silences`,
    {
      method: "POST",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}

export async function removeSilence(
  client: StarterClient,
  id: string,
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/detections/silences/${encodeURIComponent(id)}`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
}
