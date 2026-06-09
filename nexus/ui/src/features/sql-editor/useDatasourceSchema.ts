import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { getDatasourceSchema } from "@/api/datasources/schema";
import type { DatasourceSchema } from "@/api/types";

// A datasource's introspected tables/columns, cached for autocomplete. Keyed
// by datasource id and held for 5 minutes (longer than the 60s datasource
// list — a database's shape changes far less often than its set of
// connections). The query only fires once a datasource is selected; pass
// `undefined` before then and it stays idle.
const schemaKey = (id: string) => ["nexus", "datasource-schema", id] as const;

const FIVE_MINUTES = 5 * 60_000;

export function useDatasourceSchema(
  datasourceId: string | undefined,
): UseQueryResult<DatasourceSchema> {
  const client = useStarterClient();
  return useQuery({
    queryKey: schemaKey(datasourceId ?? ""),
    queryFn: () => getDatasourceSchema(client, datasourceId!),
    enabled: !!datasourceId,
    staleTime: FIVE_MINUTES,
    // The schema is a hint for autocomplete, not data the user reads — a failed
    // introspection (e.g. no permission on information_schema) should degrade
    // to keyword-only completion, not retry-storm.
    retry: false,
  });
}
