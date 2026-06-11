import { fetchJson, fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type {
  CreateDetectionRequest,
  DetectionDetail,
  DetectionStats,
  UpdateDetectionRequest,
} from "@/api/types";

// Detections — a saved insight run on a schedule that emits findings (WS-15).
// `GET/POST /api/v1/detections`, `GET/PUT/DELETE …/{id}`, plus `…/{id}/run`
// to fire one off-schedule.
export function listDetections(
  client: StarterClient,
): Promise<DetectionDetail[]> {
  return fetchJson<DetectionDetail[]>(client, `${client.apiPrefix}/detections`);
}

export function createDetection(
  client: StarterClient,
  request: CreateDetectionRequest,
): Promise<DetectionDetail> {
  return fetchJson<DetectionDetail>(client, `${client.apiPrefix}/detections`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}

export async function updateDetection(
  client: StarterClient,
  id: string,
  request: UpdateDetectionRequest,
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/detections/${encodeURIComponent(id)}`,
    {
      method: "PUT",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}

export async function removeDetection(
  client: StarterClient,
  id: string,
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/detections/${encodeURIComponent(id)}`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
}

// Run a detection now, outside its schedule — the deterministic seam for
// "create it, run it, see findings" without waiting an interval.
export async function runDetection(
  client: StarterClient,
  id: string,
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/detections/${encodeURIComponent(id)}/run`,
    { method: "POST", headers: readCsrfHeader() },
  );
}

// Run stats: next run time + findings-by-status counts. Glanceable per-row.
export function detectionStats(
  client: StarterClient,
  id: string,
): Promise<DetectionStats> {
  return fetchJson<DetectionStats>(
    client,
    `${client.apiPrefix}/detections/${encodeURIComponent(id)}/stats`,
  );
}
