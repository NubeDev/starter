// Rubix overlay — surfaces the W11 dimension freshness and W16
// ingest read-after-write bound at the top of the explorer.
//
// Reads `GET /api/warehouse/status` through the host's typed
// `StarterClient` (`fetchJson` from `@nube/starter-client-ts`); no
// raw `fetch` and no per-package transport. Rendered nothing when
// the endpoint returns 404 or the host hasn't mounted the rubix
// warehouse REST router.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

import { useQuery } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import { fetchJson, StarterError } from "@nube/starter-client-ts";
import { Card, CardContent, CardHeader, CardTitle } from "@nube/starter-ui-kit";
import { AlertTriangle, CheckCircle2, Clock, RefreshCw } from "lucide-react";
import { z } from "zod";

const dictFreshnessSchema = z.object({
  status: z.enum([
    "ok",
    "stale",
    "refreshing",
    "failed_refresh",
    "never_refreshed",
  ]),
  last_successful_refresh: z.string().nullable().optional(),
  rows: z.number().optional(),
});

const warehouseStatusSchema = z.object({
  dimensions: z.object({
    entities_dict: dictFreshnessSchema,
  }),
  ingest: z.object({
    async_insert_oldest_age_ms: z.number(),
    async_insert_backlog: z.number(),
  }),
});

export type WarehouseStatus = z.infer<typeof warehouseStatusSchema>;

const STATUS_PRESENTATION: Record<
  WarehouseStatus["dimensions"]["entities_dict"]["status"],
  { defaultLabel: string; icon: typeof CheckCircle2; tone: string }
> = {
  ok: { defaultLabel: "OK", icon: CheckCircle2, tone: "text-emerald-500" },
  stale: { defaultLabel: "STALE", icon: Clock, tone: "text-amber-500" },
  refreshing: {
    defaultLabel: "REFRESHING",
    icon: RefreshCw,
    tone: "text-sky-500",
  },
  failed_refresh: {
    defaultLabel: "FAILED REFRESH",
    icon: AlertTriangle,
    tone: "text-red-500",
  },
  never_refreshed: {
    defaultLabel: "NEVER REFRESHED",
    icon: Clock,
    tone: "text-[color:var(--color-muted)]",
  },
};

function formatMs(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)} s`;
  if (ms < 3_600_000) return `${(ms / 60_000).toFixed(1)} min`;
  return `${(ms / 3_600_000).toFixed(1)} h`;
}

export interface FreshnessTilesMessages {
  /** Label shown above the dictionary status pill. */
  entitiesTitle: string;
  /** Label shown above the W16 oldest-async-insert age tile. */
  ingestLagTitle: string;
  /** Label shown above the W16 backlog tile. */
  ingestBacklogTitle: string;
  /** Subtitle when no refresh has happened yet. */
  awaitingFirstRefresh: string;
  /** Subtitle for the lag tile. */
  ingestLagSubtitle: string;
  /** Subtitle for the backlog tile. */
  ingestBacklogSubtitle: string;
  /** Per-status labels (used in the big pill). */
  statusLabels?: Partial<
    Record<WarehouseStatus["dimensions"]["entities_dict"]["status"], string>
  >;
}

const DEFAULT_FRESHNESS_MESSAGES: FreshnessTilesMessages = {
  entitiesTitle: "ENTITIES DICTIONARY",
  ingestLagTitle: "INGEST LAG (W16)",
  ingestBacklogTitle: "INGEST BACKLOG",
  awaitingFirstRefresh: "Awaiting first refresh.",
  ingestLagSubtitle: "Oldest async-insert part awaiting flush.",
  ingestBacklogSubtitle: "Bytes pending in system.asynchronous_inserts.",
};

export interface FreshnessTilesProps {
  /** Optional message override. Defaults to English. */
  messages?: Partial<FreshnessTilesMessages>;
}

export function FreshnessTiles({ messages }: FreshnessTilesProps = {}) {
  const starter = useStarterClient();
  const m: FreshnessTilesMessages = { ...DEFAULT_FRESHNESS_MESSAGES, ...messages };

  const { data, error } = useQuery({
    queryKey: ["warehouse-explorer", "warehouse-status"],
    queryFn: async (): Promise<WarehouseStatus | null> => {
      try {
        const body = await fetchJson<unknown>(starter, "/api/warehouse/status");
        return warehouseStatusSchema.parse(body);
      } catch (e) {
        // `404` → endpoint not mounted (e.g. explorer-only demo);
        // `503` → W11 "dictionary failed_refresh" code, body still
        // valid — but the typed client treats it as an error so we
        // surface null for both. Renders nothing when null.
        if (e instanceof StarterError && (e.status === 404 || e.status === 503)) {
          return null;
        }
        throw e;
      }
    },
    refetchInterval: 60_000,
    retry: false,
  });

  if (error || data == null) return null;

  const dict = data.dimensions.entities_dict;
  const { defaultLabel, icon: StatusIcon, tone } = STATUS_PRESENTATION[dict.status];
  const label = m.statusLabels?.[dict.status] ?? defaultLabel;

  return (
    <div className="grid gap-4 md:grid-cols-2 md:gap-8 lg:grid-cols-3">
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{m.entitiesTitle}</CardTitle>
          <StatusIcon className={`h-4 w-4 ${tone}`} />
        </CardHeader>
        <CardContent>
          <div className={`text-2xl font-bold ${tone}`}>{label}</div>
          <p className="text-xs text-[color:var(--color-muted)]">
            {dict.last_successful_refresh
              ? `Last refresh ${new Date(dict.last_successful_refresh).toUTCString()}`
              : m.awaitingFirstRefresh}
            {typeof dict.rows === "number"
              ? ` · ${dict.rows.toLocaleString()} rows`
              : null}
          </p>
        </CardContent>
      </Card>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">{m.ingestLagTitle}</CardTitle>
          <Clock className="h-4 w-4 text-[color:var(--color-muted)]" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            {formatMs(data.ingest.async_insert_oldest_age_ms)}
          </div>
          <p className="text-xs text-[color:var(--color-muted)]">
            {m.ingestLagSubtitle}
          </p>
        </CardContent>
      </Card>
      <Card>
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm font-medium">
            {m.ingestBacklogTitle}
          </CardTitle>
          <Clock className="h-4 w-4 text-[color:var(--color-muted)]" />
        </CardHeader>
        <CardContent>
          <div className="text-2xl font-bold">
            {data.ingest.async_insert_backlog.toLocaleString()}
          </div>
          <p className="text-xs text-[color:var(--color-muted)]">
            {m.ingestBacklogSubtitle}
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
