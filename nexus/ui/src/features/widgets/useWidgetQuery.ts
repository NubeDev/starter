import { useQuery } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { queryDatasource } from "@/api/datasources/query";
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
      tick,
      interval,
      varRevision,
      insightId,
      insightParams,
    ],
    queryFn: () =>
      (datasourceId
        ? queryDatasource(client, datasourceId, request)
        : runQuery(client, request)
      ).then(toWidgetData),
    // A panel without SQL hasn't been authored yet — don't fire a query.
    enabled: sql.trim().length > 0,
  });

  if (result.isPending) return { status: "loading" };
  if (result.isError) {
    return {
      status: "error",
      message: result.error instanceof Error ? result.error.message : undefined,
    };
  }
  return { status: "ready", data: result.data };
}
