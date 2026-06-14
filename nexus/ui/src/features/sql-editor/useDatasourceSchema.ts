import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { getDatasourceSchema } from "@/api/datasources/schema";
import {
  NEXUS_DB_DATASOURCE_ID,
  getNexusDbSchema,
} from "@/api/nexus-db/query";
import type { DatasourceSchema } from "@/api/types";

// A datasource's introspected tables/columns/relations, cached for autocomplete
// and the schema diagram. Keyed by datasource id and held for 5 minutes (longer
// than the 60s datasource list — a database's shape changes far less often than
// its set of connections). The query only fires once a datasource is selected;
// pass `undefined` before then and it stays idle.
//
// The `NEXUS_DB_DATASOURCE_ID` sentinel resolves to the control-plane DB's own
// schema endpoint instead of `/datasources/:id/schema`, so the sidebar and
// editor browse the Nexus DB exactly like a registered datasource.
const schemaKey = (id: string) => ["nexus", "datasource-schema", id] as const;

const FIVE_MINUTES = 5 * 60_000;

export function useDatasourceSchema(
  datasourceId: string | undefined,
): UseQueryResult<DatasourceSchema> {
  const client = useStarterClient();
  return useQuery({
    queryKey: schemaKey(datasourceId ?? ""),
    queryFn: () =>
      datasourceId === NEXUS_DB_DATASOURCE_ID
        ? getNexusDbSchema(client)
        : getDatasourceSchema(client, datasourceId!),
    enabled: !!datasourceId,
    staleTime: FIVE_MINUTES,
    // The schema is a hint for autocomplete, not data the user reads — a failed
    // introspection (e.g. no permission on information_schema) should degrade
    // to keyword-only completion, not retry-storm.
    retry: false,
  });
}
