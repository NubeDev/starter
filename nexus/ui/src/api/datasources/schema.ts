import { fetchJson } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { DatasourceSchema } from "@/api/types";

// `GET /api/v1/datasources/{id}/schema` — the datasource's tables and
// columns, introspected from its `information_schema` under the read-only
// query guards. Feeds the SQL editor's autocomplete; metadata only, never
// row data.
export function getDatasourceSchema(
  client: StarterClient,
  id: string,
): Promise<DatasourceSchema> {
  return fetchJson<DatasourceSchema>(
    client,
    `${client.apiPrefix}/datasources/${encodeURIComponent(id)}/schema`,
  );
}
