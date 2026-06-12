import { useQuery, keepPreviousData } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { queryDatasource } from "@/api/datasources/query";
import { NEXUS_DB_DATASOURCE_ID, queryNexusDb } from "@/api/nexus-db/query";
import { runQuery } from "@/api/query/run";
import { toWidgetData } from "@/api/query/toWidgetData";
import type { QueryRequest } from "@/api/types";
import type { Widget } from "@/data/types";
import type { WidgetState } from "@/features/widgets/WidgetCard";
import { useTimeStore } from "@/store/time";
import { resolveTimeRange, intervalSecs } from "@/store/time";
import { useVariableStore, toQueryVariables } from "@/store/variables";

// Runs a panel's query and adapts the TanStack Query result into the
// `WidgetState` the card renders (loading / error / ready). This is the
// data subscription seam that keeps the widget itself pure (F6): the
// panel never fetches; this hook does, one level up.
//
// The query body carries the panel's SQL plus the dashboard's resolved
// global time range and `$__interval` (WS-01): the server-side binder
// substitutes `$__timeFilter`/`$__timeGroup` against them. A panel whose SQL
// uses no time macro is unaffected — the binder ignores an unreferenced range
// — so this is backwards-compatible. The resolved window is frozen against
// the time store's per-tick `now`, so every panel in one refresh shares a
// single instant, and the `tick` in the query key busts cache exactly once
// per refresh interval rather than on every render.
export function useWidgetQuery(widget: Widget): WidgetState {
  const client = useStarterClient();
  const sql = widget.config.query.sql;
  const datasourceId = widget.config.query.datasourceId;
  const insightId = widget.config.query.insightId;
  const insightParams = widget.config.query.insightParams;
  // Kind-mode (WS-10): a panel can run a declarative query-kind instead of raw
  // SQL. The backend resolves the kind, validates `params`, and binds its SQL;
  // crucially the principal-bearing `POST /api/v1/query` kind path runs against
  // the control-plane DB with `$caller_tenant_id`/`$caller_team_ids` bound — the
  // only read a non-admin has into an extension-owned table (e.g. a per-user
  // "My devices" panel over `com.acme.devices.devices_list`).
  const kind = widget.config.query.kind;
  const kindParams = widget.config.query.kindParams;

  const range = useTimeStore((s) => s.range);
  const now = useTimeStore((s) => s.now);
  const tick = useTimeStore((s) => s.tick);

  // Resolved dashboard variables (WS-02): the bar's current selections,
  // shaped as `QueryVariable[]` the server-side binder expands ($var /
  // ${var:csv} / $__sqlIn). `revision` bumps once per selection change, so
  // keying on it re-queries exactly the affected panels (item 7 / C3)
  // without busting cache on every render. A panel whose SQL references no
  // variable is unaffected — the binder ignores unreferenced values.
  const resolvedVars = useVariableStore((s) => s.resolved);
  const varRevision = useVariableStore((s) => s.revision);
  const variables = toQueryVariables(resolvedVars);

  const resolved = resolveTimeRange(range, now);
  const interval = intervalSecs(resolved);
  const request: QueryRequest = {
    sql,
    time_range: { from: resolved.from.toISOString(), to: resolved.to.toISOString() },
    interval_secs: interval,
    ...(variables.length > 0 ? { variables } : {}),
    // Kind-mode: when set the backend ignores `sql` and runs the named kind.
    ...(kind ? { kind, ...(kindParams ? { params: kindParams } : {}) } : {}),
    // RW-06: a panel-attached insight runs server-side after the query and
    // before serialization. The panel owns the SQL/datasource; the insight is
    // the transform on top. Absent when no insight is attached, so the field is
    // purely additive and a panel without one is unaffected.
    ...(insightId
      ? {
          insight: {
            insight_id: insightId,
            ...(insightParams !== undefined ? { params: insightParams } : {}),
          },
        }
      : {}),
  };

  const result = useQuery({
    // `tick` snaps the cache to the refresh tick (C3): the resolved instants
    // change every render for a relative range, so keying on them directly
    // would bust cache constantly; keying on `tick` busts once per refresh.
    // `varRevision` does the same for variable selections. The insight id +
    // params join the key so changing the attached insight re-queries.
    queryKey: [
      "nexus",
      "query",
      datasourceId,
      sql,
      kind,
      kindParams,
      tick,
      interval,
      varRevision,
      insightId,
      insightParams,
    ],
    queryFn: () =>
      // The Nexus control-plane DB isn't a registered datasource — it lives
      // behind its own `POST /nexus-db/query` and takes only raw `{ sql }`.
      // A panel selects it via the `NEXUS_DB_DATASOURCE_ID` sentinel; here we
      // route to that endpoint, which means time-range/variable/insight macros
      // in `request` are intentionally NOT applied (the endpoint ignores them).
      // Admin-only + tenant-RLS server-side, so a non-admin viewer's panel
      // surfaces a 403 as an error state.
      //
      // Kind-mode panels carry no datasource id (the kind names its own table),
      // so they fall through to `runQuery` → `POST /api/v1/query`, which binds
      // the caller's tenant/teams and resolves the kind against the metadata DB.
      (datasourceId === NEXUS_DB_DATASOURCE_ID
        ? queryNexusDb(client, sql)
        : datasourceId
          ? queryDatasource(client, datasourceId, request)
          : runQuery(client, request)
      ).then(toWidgetData),
    // Fire when the panel has either authored SQL or a kind to run. A kind-mode
    // panel has empty `sql` but is fully authored.
    enabled: sql.trim().length > 0 || (kind?.trim().length ?? 0) > 0,
    // The refresh tick changes the query key every interval (see above), so
    // without this React Query would treat each tick as a fresh query with no
    // cached data and drop the panel to its loading state — the whole grid
    // "flashes" spinners every few seconds. `keepPreviousData` keeps the prior
    // tick's data on screen while the next one fetches, so the panel only ever
    // swaps in fresh rows. `isPlaceholderData` lets callers tint stale data if
    // they want; here we just render it as-is.
    placeholderData: keepPreviousData,
  });

  if (result.isPending) return { status: "loading" };
  if (result.isError) {
    return {
      status: "error",
      message: result.error instanceof Error ? result.error.message : undefined,
    };
  }
  // A disabled/idle query (e.g. a panel whose SQL isn't authored yet) is
  // neither pending nor error but has no `data`. Treat that as loading
  // rather than handing renderers an undefined `points` to read (F0).
  if (!result.data) return { status: "loading" };
  return { status: "ready", data: result.data };
}
