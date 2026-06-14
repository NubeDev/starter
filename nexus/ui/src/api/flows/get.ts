import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { FlowDetail } from "@/api/types";

// `GET /api/v1/flows/{id}` — one flow with its full ArkFlow config
// (input / pipeline / output are opaque JSON the backend owns).
export function getFlow(client: StarterClient, id: string): Promise<FlowDetail> {
  return fetchJson<FlowDetail>(
    client,
    `${client.apiPrefix}/flows/${encodeURIComponent(id)}`,
  );
}
