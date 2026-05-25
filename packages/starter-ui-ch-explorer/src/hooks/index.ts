// React Query hooks for the ClickHouse explorer read surfaces.
// Each hook reads the host's ambient `StarterClient` (via
// `useStarterClient` from `@nube/starter-client-react`) and the
// ambient `QueryClient` — the library never constructs either.
// Mutating verbs (`rubix.clickhouse.*`) flow through typed
// `@nube/rubix-client-react` hooks consumed by the overlay
// components under `./rubix/*`.
//
// Query keys are namespaced under `["ch-explorer", …]` so a single
// `queryClient.invalidateQueries({ queryKey: ["ch-explorer"] })`
// flushes the explorer cache after coarse changes.

import {
  useInfiniteQuery,
  useQuery,
  type UseInfiniteQueryResult,
  type UseQueryResult,
} from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import { useEffect, useState } from "react";
import {
  fetchAutocomplete,
  fetchErd,
  fetchOverview,
  fetchQuery,
  fetchTable,
  fetchTableData,
  fetchTables,
  type Autocomplete,
  type Erd,
  type Overview,
  type QueryResult,
  type TableData,
  type TableMeta,
  type TablesList,
} from "../api.js";

export const chExplorerKeys = {
  all: ["ch-explorer"] as const,
  overview: () => ["ch-explorer", "overview"] as const,
  tables: () => ["ch-explorer", "tables"] as const,
  table: (name: string) => ["ch-explorer", "tables", name] as const,
  tableData: (name: string) =>
    ["ch-explorer", "tables", name, "data"] as const,
  query: (sql: string) => ["ch-explorer", "query", sql] as const,
  autocomplete: () => ["ch-explorer", "autocomplete"] as const,
  erd: () => ["ch-explorer", "erd"] as const,
};

export function useChOverview(): UseQueryResult<Overview, Error> {
  const starter = useStarterClient();
  return useQuery({
    queryKey: chExplorerKeys.overview(),
    queryFn: () => fetchOverview(starter),
  });
}

export function useChTables(): UseQueryResult<TablesList, Error> {
  const starter = useStarterClient();
  return useQuery({
    queryKey: chExplorerKeys.tables(),
    queryFn: () => fetchTables(starter),
  });
}

export function useChTable(name: string): UseQueryResult<TableMeta, Error> {
  const starter = useStarterClient();
  return useQuery({
    queryKey: chExplorerKeys.table(name),
    queryFn: () => fetchTable(starter, name),
    enabled: !!name,
  });
}

export function useChTableData(
  name: string,
): UseInfiniteQueryResult<{ pages: TableData[]; pageParams: number[] }, Error> {
  const starter = useStarterClient();
  return useInfiniteQuery({
    queryKey: chExplorerKeys.tableData(name),
    queryFn: ({ pageParam }) => fetchTableData(starter, name, pageParam),
    initialPageParam: 1,
    getNextPageParam: (lastPage, _, lastPageParams) => {
      if (lastPage.rows.length === 0) return undefined;
      return lastPageParams + 1;
    },
    enabled: !!name,
  });
}

export interface UseChQueryOptions {
  /** SQL text to send. */
  sql: string;
  /** When false, the hook is idle until `refetch` is called. */
  enabled?: boolean;
}

export function useChQuery({
  sql,
  enabled = true,
}: UseChQueryOptions): UseQueryResult<QueryResult, Error> {
  const starter = useStarterClient();
  return useQuery({
    queryKey: chExplorerKeys.query(sql),
    queryFn: () => fetchQuery(starter, sql),
    enabled,
    retry: false,
  });
}

export function useChAutocomplete(): UseQueryResult<Autocomplete, Error> {
  const starter = useStarterClient();
  return useQuery({
    queryKey: chExplorerKeys.autocomplete(),
    queryFn: () => fetchAutocomplete(starter),
  });
}

export function useChErd(): UseQueryResult<Erd, Error> {
  const starter = useStarterClient();
  return useQuery({
    queryKey: chExplorerKeys.erd(),
    queryFn: () => fetchErd(starter),
  });
}

// ---------------------------------------------------------------- theme

export type ExplorerTheme = "dark" | "light";

function readTheme(): ExplorerTheme {
  if (typeof document === "undefined") return "light";
  return document.documentElement.classList.contains("dark") ? "dark" : "light";
}

/** Read the host shell's active theme by sniffing the `dark` class
 * on `<html>`. Both the rubix shell and the demo host toggle that
 * class so the library doesn't need a `ThemeProvider` of its own. */
export function useResolvedTheme(): ExplorerTheme {
  const [theme, setTheme] = useState<ExplorerTheme>(() => readTheme());
  useEffect(() => {
    if (typeof document === "undefined") return;
    const target = document.documentElement;
    const update = () => setTheme(readTheme());
    const observer = new MutationObserver(update);
    observer.observe(target, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);
  return theme;
}

// ---------------------------------------------------------------- sql state

const SQL_STORAGE_KEY = "ch-explorer.sql";
const DEFAULT_SQL = "select 1 + 1;";

/** Persisted SQL text for the Query view. Mirrors the upstream
 * `SqlProvider` reducer — local component state + a write-through
 * `localStorage` cache so the editor restores across remounts. */
export function useSqlState(): [string, (next: string) => void] {
  const [sql, setSqlState] = useState<string>(() => {
    if (typeof localStorage === "undefined") return DEFAULT_SQL;
    return localStorage.getItem(SQL_STORAGE_KEY) ?? DEFAULT_SQL;
  });
  const setSql = (next: string) => {
    setSqlState(next);
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(SQL_STORAGE_KEY, next);
    }
  };
  return [sql, setSql];
}
