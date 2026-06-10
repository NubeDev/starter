import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { ForgetRequest, ForgetResponse } from "@/api/types";

// `POST /api/v1/audit/forget` — GDPR right-to-erasure for a user subject
// (admin-gated). Tombstones the `before`/`after`/`patch` of every change the
// subject authored in the caller's tenant while keeping the audit fact (who,
// when, which op). Returns the number of rows scrubbed.
export function forgetSubject(
  client: StarterClient,
  request: ForgetRequest,
): Promise<ForgetResponse> {
  return fetchJson<ForgetResponse>(client, `${client.apiPrefix}/audit/forget`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
