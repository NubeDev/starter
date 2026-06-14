import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { TestConnectionRequest, TestDatasourceResponse } from "@/api/types";

// `POST /api/v1/datasources/test` — probe a *raw* connection config before the
// datasource is saved. Returns `{ ok, latency_ms, message }`; `ok:false` with a
// sanitized message when the connect/auth fails. The secret is sent only to open
// the probe and is never stored. This is what the "Test connection" button in the
// create form calls, so a user can validate credentials before committing them.
export function testConnection(
  client: StarterClient,
  body: TestConnectionRequest,
): Promise<TestDatasourceResponse> {
  return fetchJson<TestDatasourceResponse>(
    client,
    `${client.apiPrefix}/datasources/test`,
    { method: "POST", headers: readCsrfHeader(), body: JSON.stringify(body) },
  );
}
