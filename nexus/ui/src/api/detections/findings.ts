import { fetchJson, fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { Finding, FindingActionRequest } from "@/api/types";

// Findings — the persistent "sparks" a detection emits, one per flagged
// target, with an open → acknowledged → resolved lifecycle (WS-15).
// `GET /api/v1/findings` (filterable), `…/{id}`, `…/{id}/ack`, `…/{id}/resolve`.

export interface FindingFilter {
  detectionId?: string;
  status?: string;
}

export function listFindings(
  client: StarterClient,
  filter: FindingFilter = {},
): Promise<Finding[]> {
  const params = new URLSearchParams();
  if (filter.detectionId) params.set("detection_id", filter.detectionId);
  if (filter.status) params.set("status", filter.status);
  const qs = params.toString();
  return fetchJson<Finding[]>(
    client,
    `${client.apiPrefix}/findings${qs ? `?${qs}` : ""}`,
  );
}

export async function ackFinding(
  client: StarterClient,
  id: string,
  body: FindingActionRequest = {},
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/findings/${encodeURIComponent(id)}/ack`,
    {
      method: "POST",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(body),
    },
  );
}

export async function resolveFinding(
  client: StarterClient,
  id: string,
  body: FindingActionRequest = {},
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/findings/${encodeURIComponent(id)}/resolve`,
    {
      method: "POST",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(body),
    },
  );
}
