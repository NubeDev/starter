import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { CreateDatasourceRequest, DatasourceDetail } from "@/api/types";

// `POST /api/v1/datasources` — register a datasource under the caller's
// tenant. The password is write-only: stored as ciphertext and absent
// from every response.
export function createDatasource(
  client: StarterClient,
  request: CreateDatasourceRequest,
): Promise<DatasourceDetail> {
  return fetchJson<DatasourceDetail>(client, `${client.apiPrefix}/datasources`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}
