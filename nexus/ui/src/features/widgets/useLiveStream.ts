import { useEffect, useRef, useState } from "react";
import { streamJson } from "@nube/starter-client-ts";
import { useStarterClient } from "@nube/starter-client-react";

import { createStream } from "@/api/streams/create";
import type { StreamEvent } from "@/api/types";
import type { SeriesPoint, Widget, WidgetData } from "@/data/types";
import type { WidgetState } from "@/features/widgets/WidgetCard";
import { appendWindow } from "@/features/widgets/_shared/window";

// How many points a live panel keeps in view. Live streams are unbounded;
// a panel renders a sliding window, not the whole history (which the
// warehouse holds). Old points fall off the front as new batches arrive.
const WINDOW = 240;

// Subscribes a panel to its live stream and exposes the rolling window as
// `WidgetState` (loading → ready, or error). The flow mirrors the F5
// contract: `POST /streams` mints a token-bearing `subscribe_url`, then an
// `EventSource` reads `StreamEvent` batches from it — the token rides the
// URL because `EventSource` can't send an Authorization header. The widget
// itself stays pure (F6); this hook is the live data seam, the streaming
// twin of `useWidgetQuery`.
export function useLiveStream(widget: Widget): WidgetState {
  const client = useStarterClient();
  const streamId = widget.config.live?.streamId;
  const sql = widget.config.query.sql;
  const datasourceId = widget.config.query.datasourceId;

  const [state, setState] = useState<WidgetState>({ status: "loading" });
  // The rolling window lives in a ref so appending a batch doesn't re-run
  // the subscription effect; we publish snapshots into state per batch.
  const points = useRef<SeriesPoint[]>([]);

  useEffect(() => {
    if (!streamId || sql.trim().length === 0) return;
    const ctrl = new AbortController();
    points.current = [];
    setState({ status: "loading" });

    (async () => {
      try {
        const minted = await createStream(client, {
          datasource_id: datasourceId,
          sql,
        });
        for await (const event of streamJson<StreamEvent>(
          client,
          minted.subscribe_url,
          { signal: ctrl.signal },
        )) {
          const batch = event.rows as SeriesPoint[];
          points.current = appendWindow(points.current, batch, WINDOW);
          const data: WidgetData = { points: points.current };
          setState({ status: "ready", data });
        }
      } catch (err) {
        if (ctrl.signal.aborted) return; // unmount / re-subscribe, not an error
        setState({
          status: "error",
          message: err instanceof Error ? err.message : undefined,
        });
      }
    })();

    return () => ctrl.abort();
  }, [client, streamId, sql, datasourceId]);

  return state;
}
