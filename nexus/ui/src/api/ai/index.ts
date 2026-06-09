// AI assist binding — synchronous, task-typed assistance (vs. streaming agent
// sessions). One round-trip: a plain-English intent plus optional grounding
// (datasource schema, current SQL) in, a structured artifact out (SQL string,
// panel spec, or dashboard spec). Backs the query editor's "write SQL for me"
// and the dashboard builder's "suggest panels".
import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { AssistRequest, AssistResponse } from "@/api/types";

/** `POST /api/v1/ai/assist` — one structured AI completion for the given task. */
export function aiAssist(
  client: StarterClient,
  request: AssistRequest,
): Promise<AssistResponse> {
  return fetchJson<AssistResponse>(client, `${client.apiPrefix}/ai/assist`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
