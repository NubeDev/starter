import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { FlowDetail, FlowExport } from "@/api/types";

// `GET /api/v1/flows/{id}/export` — the portable flow model (name + ArkFlow
// config), with secrets redacted. The caller serialises it to a file. The
// `redacted` flag tells the UI whether credentials were removed.
export function exportFlow(
  client: StarterClient,
  id: string,
): Promise<FlowExport> {
  return fetchJson<FlowExport>(
    client,
    `${client.apiPrefix}/flows/${encodeURIComponent(id)}/export`,
  );
}

// `POST /api/v1/flows/import` — re-create a flow from a previously exported
// model. The imported flow always lands stopped (`enabled: false`). 400 on an
// unknown `schema_version`. Returns the new flow's detail.
export function importFlow(
  client: StarterClient,
  model: FlowExport,
): Promise<FlowDetail> {
  return fetchJson<FlowDetail>(client, `${client.apiPrefix}/flows/import`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(model),
  });
}
