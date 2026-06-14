import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { CreateStreamRequest, CreateStreamResponse } from "@/api/types";

// `POST /api/v1/streams` — register a live stream and mint its access
// token. Returns a `subscribe_url` with the signed token already embedded
// as `?token=`, because native `EventSource` can't send an Authorization
// header (F5). The caller opens an `EventSource` on that URL before
// `expires_in_secs` elapses.
export function createStream(
  client: StarterClient,
  request: CreateStreamRequest,
): Promise<CreateStreamResponse> {
  return fetchJson<CreateStreamResponse>(client, `${client.apiPrefix}/streams`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
