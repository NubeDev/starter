import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type {
  PreviewInsightRequest,
  PreviewInsightResponse,
} from "@/api/types";

// `POST /api/v1/insights/preview` — run an inline Rhai script against a sample
// of rows the client already holds, without saving anything. Rows-in /
// rows-out: the body carries the script, the sample `rows`, and optional
// `params`; the response is an untagged union keyed by `ok`.
//
// IMPORTANT: a *script* error (compile/runtime/limit) comes back as HTTP 200
// with `ok: false` — it is NOT a failed request. The promise resolves; callers
// must branch on `response.ok` (and the presence of `result` vs `error`) rather
// than relying on a rejection. A rejection here means a transport/HTTP failure,
// not a bad script.
export function previewInsight(
  client: StarterClient,
  body: PreviewInsightRequest,
): Promise<PreviewInsightResponse> {
  return fetchJson<PreviewInsightResponse>(
    client,
    `${client.apiPrefix}/insights/preview`,
    {
      method: "POST",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(body),
    },
  );
}
