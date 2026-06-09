import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { TestDatasourceResponse } from "@/api/types";

// `POST /api/v1/datasources/{id}/test` — probe a saved datasource's
// connection. Returns `{ ok, latency_ms, message }` — `ok:false` with a
// message when the connect/auth fails. Takes no body; it tests the stored
// (sealed) credentials, so the secret never leaves the server.
export function testDatasource(
  client: StarterClient,
  id: string,
): Promise<TestDatasourceResponse> {
  return fetchJson<TestDatasourceResponse>(
    client,
    `${client.apiPrefix}/datasources/${encodeURIComponent(id)}/test`,
    { method: "POST", headers: readCsrfHeader() },
  );
}
