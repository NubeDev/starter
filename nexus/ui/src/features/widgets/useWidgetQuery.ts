import { useQuery } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";

import { queryDatasource } from "@/api/datasources/query";
import { runQuery } from "@/api/query/run";
import { toWidgetData } from "@/api/query/toWidgetData";
import type { Widget } from "@/data/types";
import type { WidgetState } from "@/features/widgets/WidgetCard";

// Runs a panel's query and adapts the TanStack Query result into the
// `WidgetState` the card renders (loading / error / ready). This is the
// data subscription seam that keeps the widget itself pure (F6): the
// panel never fetches; this hook does, one level up.
//
// The query body is the panel's SQL only — the server applies every
// safety bound and (today) targets its configured datasource; the panel
// config's `datasourceId` routes the query to that datasource; a panel
// with no datasource falls back to the server's default query route.
export function useWidgetQuery(widget: Widget): WidgetState {
  const client = useStarterClient();
  const sql = widget.config.query.sql;
  const datasourceId = widget.config.query.datasourceId;

  const result = useQuery({
    queryKey: ["nexus", "query", datasourceId, sql],
    queryFn: () =>
      (datasourceId
        ? queryDatasource(client, datasourceId, { sql })
        : runQuery(client, { sql })
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
