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

  const range = useTimeStore((s) => s.range);
  const now = useTimeStore((s) => s.now);
  const tick = useTimeStore((s) => s.tick);

  const resolved = resolveTimeRange(range, now);
  const interval = intervalSecs(resolved);
  const request: QueryRequest = {
    sql,
    time_range: { from: resolved.from.toISOString(), to: resolved.to.toISOString() },
    interval_secs: interval,
  };

  const result = useQuery({
    // `tick` snaps the cache to the refresh tick (C3): the resolved instants
    // change every render for a relative range, so keying on them directly
    // would bust cache constantly; keying on `tick` busts once per refresh.
    queryKey: ["nexus", "query", datasourceId, sql, tick, interval],
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
