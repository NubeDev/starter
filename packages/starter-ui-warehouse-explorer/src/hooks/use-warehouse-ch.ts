// Typed React Query hooks against the warehouse-ch sub-router served
// by `starter-warehouse` at `/api/warehouse/ch/*`.
//
// These wrap `fetchJson(starter, …)` from `@nube/starter-client-ts`,
// reusing the host's `StarterClient` (base URL, credentials, error
// envelope, CSRF). The transport surface intentionally matches the
// old `@nube/starter-ui-ch-explorer/api.ts` 1:1; only the hook
// wrapper is new.
//
// Naming follows the rubix-client-react convention so these can be
// promoted into `@nube/rubix-client-react` later as a single move —
// the public hook names won't change.

import {
  useQuery,
  useInfiniteQuery,
  type UseQueryResult,
  type UseInfiniteQueryResult,
} from "@tanstack/react-query";
import {
  fetchJson,
  readCsrfHeader,
  type StarterError,
} from "@nube/starter-client-ts";
import { useStarterClient } from "@nube/starter-client-react";

import type {
  Overview,
  Tables,
  Table,
  TableData,
  Query,
  Autocomplete,
  Erd,
} from "../api";

const API_ROOT = "/api/warehouse/ch";
const KEY = ["warehouse", "ch"] as const;

// The wire format for `created` / `modified` is an ISO-8601 string or
// null. Upstream views call `.toUTCString()` on them, so revive to
// Date here to keep view code unchanged.
// Wire shape from the rubix backend differs from upstream sql-studio:
//   * `size_on_disk` (was `db_size`)
//   * `sqlite_version` dropped
//   * `created` / `modified` are ISO-8601 strings (or null)
// The reviver normalises to the upstream-shaped `Overview` type so
// view code stays untouched.
interface OverviewWire
  extends Omit<Overview, "created" | "modified" | "sqlite_version" | "db_size"> {
  created: string | null;
  modified: string | null;
  size_on_disk: string;
}

function reviveOverview(w: OverviewWire): Overview {
  const { size_on_disk, created, modified, ...rest } = w;
  return {
    ...rest,
    sqlite_version: null,
    db_size: size_on_disk,
    created: created ? new Date(created) : null,
    modified: modified ? new Date(modified) : null,
  };
}

export function useWarehouseStatus(): UseQueryResult<Overview, StarterError> {
  const starter = useStarterClient();
  return useQuery<Overview, StarterError>({
    queryKey: [...KEY, "overview"],
    queryFn: async () => {
      const body = await fetchJson<OverviewWire>(starter, `${API_ROOT}/overview`);
      return reviveOverview(body);
    },
  });
}

export function useClickhouseTables(): UseQueryResult<Tables, StarterError> {
  const starter = useStarterClient();
  return useQuery<Tables, StarterError>({
    queryKey: [...KEY, "tables"],
    queryFn: () => fetchJson<Tables>(starter, `${API_ROOT}/tables`),
  });
}

export function useClickhouseTable(
  name: string,
): UseQueryResult<Table, StarterError> {
  const starter = useStarterClient();
  return useQuery<Table, StarterError>({
    queryKey: [...KEY, "table", name],
    queryFn: () =>
      fetchJson<Table>(
        starter,
        `${API_ROOT}/tables/${encodeURIComponent(name)}`,
      ),
    enabled: !!name,
  });
}

export function useClickhouseTableData(
  name: string,
): UseInfiniteQueryResult<{ pages: TableData[]; pageParams: number[] }, StarterError> {
  const starter = useStarterClient();
  return useInfiniteQuery<
    TableData,
    StarterError,
    { pages: TableData[]; pageParams: number[] },
    readonly (string | number)[],
    number
  >({
    queryKey: [...KEY, "table-data", name],
    queryFn: ({ pageParam }) =>
      fetchJson<TableData>(
        starter,
        `${API_ROOT}/tables/${encodeURIComponent(name)}/data?page=${pageParam}`,
      ),
    initialPageParam: 1,
    getNextPageParam: (lastPage, _all, lastParam) =>
      lastPage.rows.length === 0 ? undefined : lastParam + 1,
    enabled: !!name,
  });
}

export function useClickhouseQuery(
  sql: string,
  options?: { enabled?: boolean },
): UseQueryResult<Query, StarterError> {
  const starter = useStarterClient();
  return useQuery<Query, StarterError>({
    queryKey: [...KEY, "query", sql],
    queryFn: () =>
      fetchJson<Query>(starter, `${API_ROOT}/query`, {
        method: "POST",
        headers: { "content-type": "application/json", ...readCsrfHeader() },
        body: JSON.stringify({ sql }),
      }),
    enabled: options?.enabled ?? true,
    retry: false,
  });
}

export function useClickhouseErd(): UseQueryResult<Erd, StarterError> {
  const starter = useStarterClient();
  return useQuery<Erd, StarterError>({
    queryKey: [...KEY, "erd"],
    queryFn: () => fetchJson<Erd>(starter, `${API_ROOT}/erd`),
  });
}

export function useClickhouseAutocomplete(): UseQueryResult<Autocomplete, StarterError> {
  const starter = useStarterClient();
  return useQuery<Autocomplete, StarterError>({
    queryKey: [...KEY, "autocomplete"],
    queryFn: () => fetchJson<Autocomplete>(starter, `${API_ROOT}/autocomplete`),
  });
}
