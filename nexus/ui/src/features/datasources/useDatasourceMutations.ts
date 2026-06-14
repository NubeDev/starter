import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { createDatasource } from "@/api/datasources/create";
import { removeDatasource } from "@/api/datasources/remove";
import { testConnection } from "@/api/datasources/test-connection";
import type {
  CreateDatasourceRequest,
  DatasourceDetail,
  TestConnectionRequest,
  TestDatasourceResponse,
} from "@/api/types";

const DATASOURCES_KEY = ["nexus", "datasources"] as const;

// Create a datasource, then refresh the list. The password is write-only —
// it's sent here and never returned (the detail carries a redacted
// connection), so nothing sensitive lingers in the cache.
export function useCreateDatasource() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<DatasourceDetail, Error, CreateDatasourceRequest>({
    mutationFn: (body) => createDatasource(client, body),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: DATASOURCES_KEY }),
  });
}

// Probe a raw connection config before saving. Unlike the saved-datasource
// test, this carries the typed-in credentials so the user can validate them in
// the create form before committing. A failed probe is a normal result
// (`ok:false` with a message), not a mutation error, so the hook only rejects on
// a transport failure.
export function useTestConnection() {
  const client = useStarterClient();
  return useMutation<TestDatasourceResponse, Error, TestConnectionRequest>({
    mutationFn: (body) => testConnection(client, body),
  });
}

// Remove a datasource, then refresh the list.
export function useRemoveDatasource() {
  const client = useStarterClient();
  const queryClient = useQueryClient();
  return useMutation<void, Error, string>({
    mutationFn: (id) => removeDatasource(client, id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: DATASOURCES_KEY }),
  });
}
