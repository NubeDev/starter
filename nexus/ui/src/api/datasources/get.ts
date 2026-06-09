import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { DatasourceDetail } from "@/api/types";

// `GET /api/v1/datasources/{id}` — full detail for one datasource. The
// connection is redacted (secrets never leave the server).
export function getDatasource(
  client: StarterClient,
  id: string,
): Promise<DatasourceDetail> {
  return fetchJson<DatasourceDetail>(
    client,
    `${client.apiPrefix}/datasources/${encodeURIComponent(id)}`,
  );
}
