// `loadtest/page.tsx` — diagnostics / load-test harness for the
// rubixos warehouse templates.
//
// Purpose: exercise the real data set (all sites / hosts / meters) at
// controllable scale and observe (a) page-load latency under
// concurrency and (b) cache behaviour (cold vs warm), without fighting
// the production dashboard's hardcoded caps.
//
// How it measures honestly:
//  - Every fired request uses `callTool(..., { fresh: true })`, which
//    BYPASSES the frontend in-flight coalescing in `api.ts`. Otherwise
//    the harness would measure the client dedup map, not the backend.
//    Backend single-flight + the cache layer are still exercised.
//  - "Cold" runs append a unique `_n` nonce to the params so the cache
//    key is guaranteed fresh (a real miss); "warm" runs repeat the
//    exact same params so the second hit should be served from cache.
//    The cold→warm latency drop is the visible proof the cache works
//    end-to-end (the backend also logs decision="hit"/"miss" under
//    RUST_LOG=starter_cache::access=debug).

import * as React from "react";
import { EXTENSION_ID } from "../types";
import { callTool } from "../api";
import { useSlotContext, useHostTheme } from "@nube/starter-ext-sdk-ts";

// ----------------------------------------------------------------------
// Types
// ----------------------------------------------------------------------

interface HostRow {
  host_uuid: string;
  host_name: string | null;
  point_count: number;
}
interface MeterRow {
  uuid: string;
  host_uuid: string;
  host_name: string | null;
}
interface WarehouseResp<R> {
  rows: ReadonlyArray<R>;
  count?: number;
}

interface CallResult {
  ok: boolean;
  status: number | "err";
  ms: number;
  rows?: number;
  error?: string;
}

// One template the harness can fire, plus how to build its params for a
// given point-set + window.
interface TemplateChoice {
  id: string;
  label: string;
  needsPoints: boolean;
  build: (ctx: BuildCtx) => Record<string, unknown>;
}

interface BuildCtx {
  pointUuidsCsv: string;
  from: string;
  to: string;
  bucket: string;
}

// NOTE on forcing a cold cache key: the templates' params schemas use
// `additionalProperties: false`, so we CANNOT smuggle a nonce param —
// the backend validator rejects unknown keys. Instead the cold/warm
// test varies a *valid* param (`to`, by a few ms) to get a fresh cache
// key. `histories_summary` takes no params, so its key is constant —
// for it, "cold" means the first call after a cache flush/restart.
const TEMPLATES: ReadonlyArray<TemplateChoice> = [
  {
    id: "histories_summary",
    label: "histories_summary (KPI)",
    needsPoints: false,
    build: () => ({}),
  },
  {
    id: "usage_site_totals",
    label: "usage_site_totals",
    needsPoints: true,
    build: ({ pointUuidsCsv, from, to }) => ({
      point_uuids: pointUuidsCsv,
      from,
      to,
    }),
  },
  {
    id: "usage_bucketed",
    label: "usage_bucketed",
    needsPoints: true,
    build: ({ pointUuidsCsv, from, to, bucket }) => ({
      point_uuids: pointUuidsCsv,
      from,
      to,
      bucket,
    }),
  },
  {
    id: "usage_per_meter",
    label: "usage_per_meter",
    needsPoints: true,
    build: ({ pointUuidsCsv, from, to }) => ({
      point_uuids: pointUuidsCsv,
      from,
      to,
      limit: 50,
    }),
  },
];

// Whether a template's cache key can be made unique per-run by nudging
// `to`. `histories_summary` (no params) can't, so cold/warm there only
// works against a freshly-flushed cache.
function canForceCold(t: TemplateChoice): boolean {
  return t.needsPoints; // these all carry `to`
}

// ----------------------------------------------------------------------
// Timed call — always `fresh` so we measure the backend, not the
// frontend coalescing map.
// ----------------------------------------------------------------------

async function timedQuery(
  template: string,
  params: Record<string, unknown>,
): Promise<CallResult> {
  const t0 = performance.now();
  try {
    const res = await callTool<WarehouseResp<unknown>>(
      `${EXTENSION_ID}.warehouse_query`,
      // Warehouse templates are addressed by their FULLY-QUALIFIED name
      // (`com.nubeio.rubixos.<id>`); a bare id is rejected as "outside
      // this extension's namespace". Qualify here so callers pass bare
      // ids (`hosts_overview`, `usage_site_totals`, …).
      { template: `${EXTENSION_ID}.${template}`, params },
      { fresh: true },
    );
    return {
      ok: true,
      status: 200,
      ms: performance.now() - t0,
      rows: res.count ?? res.rows?.length ?? 0,
    };
  } catch (e) {
    return {
      ok: false,
      status: "err",
      ms: performance.now() - t0,
      error: e instanceof Error ? e.message : String(e),
    };
  }
}

function pct(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0;
  const i = Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length));
  return sorted[i]!;
}

function fmtMs(ms: number): string {
  return ms >= 1000 ? `${(ms / 1000).toFixed(2)}s` : `${Math.round(ms)}ms`;
}

// ----------------------------------------------------------------------
// Component
// ----------------------------------------------------------------------

export function LoadTestPage(): React.ReactElement {
  const slot = useSlotContext();
  const theme = useHostTheme();

  // --- Inventory (all sites / meters, NOT capped to the dashboard's 2000)
  const [hosts, setHosts] = React.useState<ReadonlyArray<HostRow>>([]);
  const [meters, setMeters] = React.useState<ReadonlyArray<MeterRow>>([]);
  const [invLoading, setInvLoading] = React.useState(false);
  const [invError, setInvError] = React.useState<string | null>(null);

  // --- Test config
  const [kind, setKind] = React.useState<"elec" | "water">("elec");
  const [templateId, setTemplateId] = React.useState("usage_site_totals");
  const [meterCount, setMeterCount] = React.useState(500);
  const [rangeDays, setRangeDays] = React.useState(7);
  const [bucket, setBucket] = React.useState("1 hour");
  const [concurrency, setConcurrency] = React.useState(5);
  const [iterations, setIterations] = React.useState(1);

  // --- Results
  const [running, setRunning] = React.useState(false);
  const [results, setResults] = React.useState<CallResult[]>([]);
  const [coldWarm, setColdWarm] = React.useState<{ cold: number; warm: number } | null>(null);
  const [log, setLog] = React.useState<string[]>([]);

  const pushLog = (s: string) =>
    setLog((l) => [`${new Date().toLocaleTimeString()}  ${s}`, ...l].slice(0, 100));

  // Load inventory once (and on kind change for meters).
  const loadInventory = React.useCallback(async () => {
    setInvLoading(true);
    setInvError(null);
    try {
      const [h, m] = await Promise.all([
        callTool<WarehouseResp<HostRow>>(
          `${EXTENSION_ID}.warehouse_query`,
          { template: `${EXTENSION_ID}.hosts_overview`, params: { limit: 1000 } },
          { fresh: true },
        ),
        callTool<WarehouseResp<MeterRow>>(
          `${EXTENSION_ID}.warehouse_query`,
          {
            // High limit: we WANT the full catalog here — this page's
            // whole point is to see beyond the dashboard's 2000 cap.
            template: `${EXTENSION_ID}.meters_list`,
            params: { kind, secondary_tag: "", limit: 50000 },
          },
          { fresh: true },
        ),
      ]);
      setHosts(h.rows);
      setMeters(m.rows);
      pushLog(
        `inventory: ${h.rows.length} hosts, ${m.rows.length} ${kind} meters, ` +
          `${new Set(m.rows.map((r) => r.host_uuid)).size} sites with ${kind} meters`,
      );
    } catch (e) {
      setInvError(e instanceof Error ? e.message : String(e));
    } finally {
      setInvLoading(false);
    }
  }, [kind]);

  React.useEffect(() => {
    void loadInventory();
  }, [loadInventory]);

  const sitesWithMeters = React.useMemo(
    () => new Set(meters.map((m) => m.host_uuid)).size,
    [meters],
  );

  // Build a point-set of the requested size from the loaded meters.
  const pointUuidsCsv = React.useMemo(
    () => meters.slice(0, meterCount).map((m) => m.uuid).join(","),
    [meters, meterCount],
  );

  const window = React.useMemo(() => {
    const to = new Date();
    const from = new Date(to.getTime() - rangeDays * 86400_000);
    return { from: from.toISOString(), to: to.toISOString() };
  }, [rangeDays]);

  const chosen = TEMPLATES.find((t) => t.id === templateId)!;

  // --- Run a burst: `concurrency` requests in parallel, `iterations` times.
  const runBurst = React.useCallback(async () => {
    setRunning(true);
    setResults([]);
    pushLog(
      `BURST: ${chosen.id} × ${concurrency} concurrent × ${iterations} iter ` +
        `(${chosen.needsPoints ? `${meterCount} meters` : "no points"}, ${rangeDays}d)`,
    );
    const all: CallResult[] = [];
    try {
      for (let it = 0; it < iterations; it++) {
        const batch = Array.from({ length: concurrency }, () =>
          timedQuery(
            chosen.id,
            chosen.build({ pointUuidsCsv, from: window.from, to: window.to, bucket }),
          ),
        );
        const res = await Promise.all(batch);
        all.push(...res);
        setResults([...all]);
        const fails = res.filter((r) => !r.ok).length;
        pushLog(
          `  iter ${it + 1}: ${res.length} calls, ${fails} failed, ` +
            `slowest ${fmtMs(Math.max(...res.map((r) => r.ms)))}`,
        );
      }
    } finally {
      setRunning(false);
    }
  }, [chosen, concurrency, iterations, meterCount, rangeDays, pointUuidsCsv, window, bucket]);

  // --- Cold vs warm: one cold (fresh cache key), then an immediate warm
  // repeat (identical params → cache hit). Proves caching end-to-end.
  //
  // To guarantee the cold call is a real miss we nudge `to` by a unique
  // few-ms offset (a *valid* param — unlike a nonce, which the schema
  // would reject). For `histories_summary` (no params) we can't force a
  // fresh key, so "cold" there is only meaningful right after a restart;
  // we flag that in the log.
  const runColdWarm = React.useCallback(async () => {
    setRunning(true);
    setColdWarm(null);
    let coldParams: Record<string, unknown>;
    if (canForceCold(chosen)) {
      // Unique `to` → guaranteed fresh cache key for this run.
      const uniqueTo = new Date(Date.parse(window.to) - Math.floor(Math.random() * 1000)).toISOString();
      coldParams = chosen.build({ pointUuidsCsv, from: window.from, to: uniqueTo, bucket });
      pushLog(`COLD/WARM: ${chosen.id} — cold (fresh key via unique 'to') then warm (repeat)`);
    } else {
      coldParams = chosen.build({ pointUuidsCsv, from: window.from, to: window.to, bucket });
      pushLog(`COLD/WARM: ${chosen.id} — no params to vary; cold is only real after a cache flush/restart`);
    }
    try {
      const cold = await timedQuery(chosen.id, coldParams);
      // Warm: identical params → should hit cache.
      const warm = await timedQuery(chosen.id, coldParams);
      setColdWarm({ cold: cold.ms, warm: warm.ms });
      const ratio = warm.ms > 0 ? (cold.ms / warm.ms).toFixed(1) : "∞";
      pushLog(
        `  cold ${fmtMs(cold.ms)} → warm ${fmtMs(warm.ms)} ` +
          `(${ratio}× faster warm — cache ${warm.ms < cold.ms * 0.6 ? "WORKING ✓" : "no clear hit ✗"})`,
      );
    } finally {
      setRunning(false);
    }
  }, [chosen, pointUuidsCsv, window, bucket]);

  // --- Derived stats
  const stats = React.useMemo(() => {
    const oks = results.filter((r) => r.ok);
    const sorted = oks.map((r) => r.ms).sort((a, b) => a - b);
    return {
      n: results.length,
      ok: oks.length,
      fail: results.length - oks.length,
      p50: pct(sorted, 50),
      p95: pct(sorted, 95),
      max: sorted.length ? sorted[sorted.length - 1]! : 0,
      min: sorted.length ? sorted[0]! : 0,
    };
  }, [results]);

  const card = "rounded-xl border border-black/10 dark:border-white/10 bg-white/60 dark:bg-white/5 p-4";
  const label = "text-xs font-medium uppercase tracking-wide text-slate-500";
  const btn =
    "rounded-lg px-3 py-1.5 text-sm font-medium border border-black/10 dark:border-white/15 " +
    "hover:bg-black/5 dark:hover:bg-white/10 disabled:opacity-40 disabled:cursor-not-allowed";
  const btnPrimary =
    "rounded-lg px-4 py-2 text-sm font-semibold bg-teal-500 text-white hover:bg-teal-600 " +
    "disabled:opacity-40 disabled:cursor-not-allowed";

  return (
    <div
      data-ext-id={EXTENSION_ID}
      data-ext-slot={slot.slotId}
      data-ext-theme={theme.mode}
      className="flex flex-col gap-4 p-4"
    >
      <div>
        <h1 className="text-lg font-semibold">Warehouse load-test &amp; cache diagnostics</h1>
        <p className="text-sm text-slate-500">
          Exercise all sites / meters at controllable scale; measure latency under
          concurrency and cold-vs-warm cache behaviour. Requests bypass the frontend
          coalescing so timing reflects the backend.
        </p>
      </div>

      {invError ? (
        <div role="alert" className="rounded-md border border-red-400/40 bg-red-400/10 text-red-600 px-3 py-2 text-sm">
          inventory error: {invError}
        </div>
      ) : null}

      {/* Inventory */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <div className={card}>
          <div className={label}>Hosts (all)</div>
          <div className="text-2xl font-bold tabular-nums">{hosts.length}</div>
        </div>
        <div className={card}>
          <div className={label}>{kind} meters</div>
          <div className="text-2xl font-bold tabular-nums">{meters.length.toLocaleString()}</div>
        </div>
        <div className={card}>
          <div className={label}>Sites with {kind} meters</div>
          <div className="text-2xl font-bold tabular-nums">{sitesWithMeters}</div>
        </div>
        <div className={card}>
          <div className={label}>Inventory</div>
          <button className={btn} onClick={() => void loadInventory()} disabled={invLoading}>
            {invLoading ? "loading…" : "reload"}
          </button>
        </div>
      </div>

      {/* Config */}
      <div className={card}>
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-4">
          <Field label="Kind">
            <select className="w-full bg-transparent border rounded px-2 py-1 text-sm" value={kind} onChange={(e) => setKind(e.target.value as "elec" | "water")}>
              <option value="elec">elec</option>
              <option value="water">water</option>
            </select>
          </Field>
          <Field label="Template">
            <select className="w-full bg-transparent border rounded px-2 py-1 text-sm" value={templateId} onChange={(e) => setTemplateId(e.target.value)}>
              {TEMPLATES.map((t) => (
                <option key={t.id} value={t.id}>{t.label}</option>
              ))}
            </select>
          </Field>
          <Field label={`Meters (${meterCount} / ${meters.length})`}>
            <input
              type="range" min={1} max={Math.max(1, meters.length)} step={1}
              value={Math.min(meterCount, Math.max(1, meters.length))}
              onChange={(e) => setMeterCount(Number(e.target.value))}
              disabled={!chosen.needsPoints}
              className="w-full"
            />
          </Field>
          <Field label="Range (days)">
            <select className="w-full bg-transparent border rounded px-2 py-1 text-sm" value={rangeDays} onChange={(e) => setRangeDays(Number(e.target.value))}>
              {[1, 7, 30, 90, 180, 365].map((d) => <option key={d} value={d}>{d}d</option>)}
            </select>
          </Field>
          <Field label="Concurrency">
            <input type="number" min={1} max={50} value={concurrency} onChange={(e) => setConcurrency(Math.max(1, Number(e.target.value)))} className="w-full bg-transparent border rounded px-2 py-1 text-sm" />
          </Field>
          <Field label="Iterations">
            <input type="number" min={1} max={50} value={iterations} onChange={(e) => setIterations(Math.max(1, Number(e.target.value)))} className="w-full bg-transparent border rounded px-2 py-1 text-sm" />
          </Field>
        </div>
        <div className="mt-4 flex flex-wrap gap-2 items-center">
          <button className={btnPrimary} onClick={() => void runBurst()} disabled={running || invLoading}>
            {running ? "running…" : `Run burst (${concurrency}×${iterations})`}
          </button>
          <button className={btn} onClick={() => void runColdWarm()} disabled={running || invLoading}>
            Cold vs warm (cache proof)
          </button>
          {chosen.needsPoints ? (
            <span className="text-xs text-slate-500">
              point-set: {pointUuidsCsv ? pointUuidsCsv.split(",").length : 0} uuids
            </span>
          ) : (
            <span className="text-xs text-slate-500">no point-set (KPI template)</span>
          )}
        </div>
      </div>

      {/* Cold vs warm */}
      {coldWarm ? (
        <div className={card}>
          <div className={label}>Cold vs warm (same params)</div>
          <div className="flex gap-6 items-end mt-2">
            <div>
              <div className="text-xs text-slate-500">cold (miss)</div>
              <div className="text-2xl font-bold tabular-nums text-amber-500">{fmtMs(coldWarm.cold)}</div>
            </div>
            <div>
              <div className="text-xs text-slate-500">warm (hit)</div>
              <div className="text-2xl font-bold tabular-nums text-teal-500">{fmtMs(coldWarm.warm)}</div>
            </div>
            <div className="text-sm">
              {coldWarm.warm < coldWarm.cold * 0.6
                ? <span className="text-teal-500 font-medium">cache working — {(coldWarm.cold / Math.max(1, coldWarm.warm)).toFixed(1)}× faster warm</span>
                : <span className="text-amber-500 font-medium">no clear cache hit (warm not much faster)</span>}
            </div>
          </div>
        </div>
      ) : null}

      {/* Burst stats */}
      {stats.n > 0 ? (
        <div className="grid grid-cols-3 sm:grid-cols-6 gap-3">
          <Stat label="calls" value={`${stats.ok}/${stats.n}`} />
          <Stat label="failed" value={String(stats.fail)} bad={stats.fail > 0} />
          <Stat label="min" value={fmtMs(stats.min)} />
          <Stat label="p50" value={fmtMs(stats.p50)} />
          <Stat label="p95" value={fmtMs(stats.p95)} />
          <Stat label="max" value={fmtMs(stats.max)} />
        </div>
      ) : null}

      {/* Per-call results */}
      {results.length > 0 ? (
        <div className={card}>
          <div className={label}>Per-call ({results.length})</div>
          <div className="mt-2 flex flex-wrap gap-1">
            {results.map((r, i) => (
              <span
                key={i}
                title={r.error ?? `${r.rows ?? 0} rows`}
                className={
                  "inline-flex items-center rounded px-1.5 py-0.5 text-[0.7rem] tabular-nums " +
                  (r.ok
                    ? r.ms > 10000
                      ? "bg-amber-400/20 text-amber-600"
                      : "bg-teal-400/20 text-teal-700 dark:text-teal-300"
                    : "bg-red-400/20 text-red-600")
                }
              >
                {r.ok ? fmtMs(r.ms) : `ERR`}
              </span>
            ))}
          </div>
        </div>
      ) : null}

      {/* Log */}
      <div className={card}>
        <div className={label}>Log</div>
        <pre className="mt-2 text-[0.7rem] leading-relaxed whitespace-pre-wrap font-mono text-slate-600 dark:text-slate-300 max-h-64 overflow-auto">
          {log.join("\n") || "—"}
        </pre>
        <p className="mt-2 text-[0.7rem] text-slate-400">
          Backend cache hit/miss is logged under{" "}
          <code>RUST_LOG=info,starter_cache::access=debug</code> — grep the agent log for{" "}
          <code>decision="hit"</code> / <code>"miss"</code> / <code>"coalesced_hit"</code>.
        </p>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }): React.ReactElement {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs font-medium uppercase tracking-wide text-slate-500">{label}</span>
      {children}
    </label>
  );
}

function Stat({ label, value, bad }: { label: string; value: string; bad?: boolean }): React.ReactElement {
  return (
    <div className="rounded-xl border border-black/10 dark:border-white/10 bg-white/60 dark:bg-white/5 p-3">
      <div className="text-xs font-medium uppercase tracking-wide text-slate-500">{label}</div>
      <div className={"text-xl font-bold tabular-nums " + (bad ? "text-red-500" : "")}>{value}</div>
    </div>
  );
}
