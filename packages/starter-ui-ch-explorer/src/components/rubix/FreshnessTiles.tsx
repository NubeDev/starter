// PR 4 — rubix overlays. First slice: surface the W11 dimension
// freshness + W16 ingest read-after-write bound at the top of the
// explorer index. Pure read; no rubix verb dispatch yet.
//
// The component renders nothing when `/api/warehouse/status` 404s,
// which is the case for the `examples/ch-explorer` demo binary
// (it only mounts `starter_warehouse::explorer::routes`, not the
// full `starter_warehouse::rest::router`). A rubix-agent deployment
// mounts both, so the tiles appear automatically there.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import { useQuery } from "@tanstack/react-query";
import { CheckCircle2, AlertTriangle, RefreshCw, Clock } from "lucide-react";

import { fetchWarehouseStatus, WarehouseStatus } from "@/api";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

const STATUS_PRESENTATION: Record<
  WarehouseStatus["dimensions"]["entities_dict"]["status"],
  { label: string; icon: typeof CheckCircle2; tone: string }
> = {
  ok: {
    label: "OK",
    icon: CheckCircle2,
    tone: "text-emerald-500",
  },
  stale: {
    label: "STALE",
    icon: Clock,
    tone: "text-amber-500",
  },
  refreshing: {
    label: "REFRESHING",
    icon: RefreshCw,
    tone: "text-sky-500",
  },
  failed_refresh: {
    label: "FAILED REFRESH",
    icon: AlertTriangle,
    tone: "text-red-500",
  },
  never_refreshed: {
    label: "NEVER REFRESHED",
    icon: Clock,
    tone: "text-muted-foreground",
  },
};

function formatMs(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)} s`;
  if (ms < 3_600_000) return `${(ms / 60_000).toFixed(1)} min`;
  return `${(ms / 3_600_000).toFixed(1)} h`;
}

export function FreshnessTiles() {
  const { data, error } = useQuery({
    queryKey: ["warehouse-status"],
    queryFn: fetchWarehouseStatus,
    // Refresh once a minute; the W11 envelope updates on the same
    // cadence as the dictionary materialised-view refresh.
    refetchInterval: 60_000,
    // Don't retry — the absence of the endpoint is itself the
    // signal that the rubix overlay is disabled.
    retry: false,
  });

  // Endpoint not mounted (404 → null) or hard error → no overlay.
  if (error || data == null) return null;

  const dict = data.dimensions.entities_dict;
  const { label, icon: StatusIcon, tone } = STATUS_PRESENTATION[dict.status];

  return (
    <div className="grid gap-4 md:grid-cols-2 md:gap-8 lg:grid-cols-3">
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">
            ENTITIES DICTIONARY
          </CardTitle>
          <StatusIcon className={`h-4 w-4 ${tone}`} />
        </CardHeader>
        <CardContent>
          <div className={`text-2xl font-bold ${tone}`}>{label}</div>
          <p className="text-xs text-muted-foreground">
            {dict.last_successful_refresh
              ? `Last refresh ${new Date(dict.last_successful_refresh).toUTCString()}`
              : "Awaiting first refresh."}
            {typeof dict.rows === "number"
              ? ` · ${dict.rows.toLocaleString()} rows`
              : null}
          </p>
        </CardContent>
      </Card>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">
            INGEST LAG (W16)
          </CardTitle>
          <Clock className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            {formatMs(data.ingest.async_insert_oldest_age_ms)}
          </div>
          <p className="text-xs text-muted-foreground">
            Oldest async-insert part awaiting flush.
          </p>
        </CardContent>
      </Card>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">
            INGEST BACKLOG
          </CardTitle>
          <Clock className="h-4 w-4 text-muted-foreground" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            {data.ingest.async_insert_backlog.toLocaleString()}
          </div>
          <p className="text-xs text-muted-foreground">
            Bytes pending in <code>system.asynchronous_inserts</code>.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
