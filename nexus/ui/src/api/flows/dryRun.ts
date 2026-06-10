import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { DryRunRequest, DryRunResponse } from "@/api/types";

// `POST /api/v1/flows/dry-run` — validate a flow's input + pipeline and run a
// bounded sample, without persisting it or writing to its real output. The
// editor's "Test" button. A build/runtime failure comes back as a 200 with
// `error` set, so the caller renders it inline rather than throwing.
export function dryRunFlow(
  client: StarterClient,
  request: DryRunRequest,
): Promise<DryRunResponse> {
  return fetchJson<DryRunResponse>(client, `${client.apiPrefix}/flows/dry-run`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
