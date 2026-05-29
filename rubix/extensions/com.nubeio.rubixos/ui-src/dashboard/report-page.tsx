// `ReportPage` — printable executive report.
//
// One-page A4 layout intended for "email the PDF to the boss on
// Monday". Single column, light, generous whitespace, big radial
// KPI up top, region table, leaderboard, time-series snapshot.
//
// Re-uses the warehouse fetches and the `RadialKpi` /
// `RegionRollup` / `TopMeters` / `UsageTimeSeries` components from
// the operator dashboard so any data correction or feature added
// to the dashboard automatically shows up here.

import * as React from "react";

import { EXTENSION_ID, asNumber } from "../types";
import type {
  HistoriesSummaryRow,
  MeterRow,
  UsageBucketRow,
  UsagePerMeterRow,
  UsageSiteTotalRow,
} from "../types";
import { fetchTemplate } from "../api";

import { KINDS, RANGES, SERIES_TOP_N } from "./presets";
import { fmtBig, inferUnit } from "./helpers";
import { ROLE_ACCENT, IconBolt, IconPrint, IconRegion, IconTrend, IconAlert, IconWave } from "./icons";
import { LoadingToast } from "./prims";
import { RadialKpi } from "./radial-kpi";
import { RegionRollup, buildRegions } from "./region-rollup";
import { TopMeters } from "./top-meters";
import { UsageTimeSeries } from "./usage-time-series";
import { readUrlState, useUrlSync } from "./url-state";

export function ReportPage(): React.ReactElement {
  // Two-channel report: we fetch BOTH elec and water in parallel
  // so the printed page tells the whole story (most BMS reports
  // get emailed as a single deck).
  //
  // `?range=…` is honoured so links like
  //   /extensions/com.nubeio.rubixos/report?range=1y
  // open straight to the right window. `kind` is ignored here —
  // the report always shows both channels.
  const urlInit = React.useRef(readUrlState()).current;
  const [rangeIdx, setRangeIdx] = React.useState(urlInit.rangeIdx ?? 5); // default 1y for annual reports
  useUrlSync(null, rangeIdx);
  const range = RANGES[rangeIdx]!;

  const [latestSampleMs, setLatestSampleMs] = React.useState<number | null>(null);
  React.useEffect(() => {
    let cancelled = false;
    fetchTemplate<HistoriesSummaryRow>(`${EXTENSION_ID}.histories_summary`, {})
      .then((rs) => {
        if (cancelled) return;
        const t = rs[0]?.latest ? Date.parse(rs[0]!.latest!) : NaN;
        setLatestSampleMs(Number.isFinite(t) ? t : null);
      })
      .catch(() => { /* fall back to wall clock */ });
    return () => { cancelled = true; };
  }, []);

  const win = React.useMemo(() => {
    // Floor to the hour so cache keys are stable across reloads / mounts.
    // The warehouse_query cache spec uses a 1h time_series bucket; aligning
    // here means a returning user within TTL hits cache instead of cold SQL.
    const HOUR = 3_600_000;
    const rawTo = latestSampleMs ?? Date.now();
    const toMs = Math.floor(rawTo / HOUR) * HOUR;
    const from = new Date(toMs - range.hours * HOUR).toISOString();
    const to   = new Date(toMs).toISOString();
    return { from, to, bucket: range.bucket };
  }, [range.hours, range.bucket, latestSampleMs]);

  const prevWin = React.useMemo(() => {
    const toMs = Date.parse(win.from);
    if (!Number.isFinite(toMs)) return null;
    // Skip the prior-window fetch for long ranges. At 6m/1y the
    // extra round-trip costs ~3s on elec just to compute one
    // delta number — not worth the wait. The KPI shows "no prior
    // period data" in that case, which is honest.
    if (range.hours > 2160) return null;
    const fromMs = toMs - range.hours * 3600_000;
    return { from: new Date(fromMs).toISOString(), to: new Date(toMs).toISOString() };
  }, [win.from, range.hours]);

  // Channel coordination: at portfolio scale (~672 elec meters,
  // ~77 water meters) Postgres handles two concurrent aggregate
  // queries comfortably — measurements show each channel runs in
  // ~12s elec / ~2s water warm, regardless of whether the other
  // is in flight. Running them in parallel halves total wall
  // time vs. strict serialisation. Inside each channel queries
  // are still chained so the page paints progressively.
  const [elecDone, setElecDone] = React.useState(false);
  const [waterDone, setWaterDone] = React.useState(false);
  const [loadDismissed, setLoadDismissed] = React.useState(false);
  const isLoading = !elecDone || !waterDone;
  React.useEffect(() => {
    if (isLoading) setLoadDismissed(false);
  }, [isLoading]);
  // Reset gates when the window changes (range or anchor moves).
  React.useEffect(() => {
    setElecDone(false);
    setWaterDone(false);
  }, [win.from, win.to, win.bucket]);

  // Stable callbacks so ReportChannel's effect dep array doesn't
  // re-trigger on every render of the parent.
  const onWaterDone = React.useCallback(() => setWaterDone(true), []);
  const onElecDone = React.useCallback(() => setElecDone(true), []);

  return (
    <div className="ext-report flex flex-col gap-6 print:gap-3 max-w-[920px] mx-auto py-4 print:py-0">
      <ReportCover
        latestSampleMs={latestSampleMs}
        rangeLabel={range.label}
        rangeIdx={rangeIdx}
        onRangeChange={setRangeIdx}
      />
      <LoadingToast show={isLoading && !loadDismissed} onClose={() => setLoadDismissed(true)} />
      {/* Both channels start immediately and run concurrently —
          water finishes first (small dataset) and paints, elec
          follows. */}
      <ReportChannel
        kind="water"
        win={win} prevWin={prevWin}
        bucket={range.bucket} rangeLabel={range.label}
        ready
        onDone={onWaterDone}
      />
      <ReportChannel
        kind="elec"
        win={win} prevWin={prevWin}
        bucket={range.bucket} rangeLabel={range.label}
        ready
        onDone={onElecDone}
      />
      <ReportFooter />
    </div>
  );
}

/* ============================ cover =============================== */

function ReportCover({
  latestSampleMs, rangeLabel, rangeIdx, onRangeChange,
}: {
  latestSampleMs: number | null;
  rangeLabel: string;
  rangeIdx: number;
  onRangeChange: (n: number) => void;
}): React.ReactElement {
  const now = latestSampleMs ? new Date(latestSampleMs) : new Date();
  const periodFrom = new Date(now.getTime() - (RANGES[rangeIdx]!.hours * 3600_000));
  return (
    <header className="ext-report-cover flex items-end justify-between gap-6 pb-4 border-b border-border print:border-slate-300">
      <div>
        <div className="ext-eyebrow text-muted-foreground">Rubix-OS · Portfolio report</div>
        <h1 className="mt-1 text-3xl font-semibold tracking-tight text-foreground">
          Energy &amp; Water — {rangeLabel}
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">
          {periodFrom.toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" })}
          {" → "}
          {now.toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" })}
        </p>
      </div>
      <div className="flex items-center gap-2 print:hidden">
        <div className="flex gap-1">
          {RANGES.map((r, i) => (
            <button
              key={r.label}
              type="button"
              onClick={() => onRangeChange(i)}
              className={
                "px-3 py-1 text-xs rounded-md border cursor-pointer transition-colors " +
                (i === rangeIdx
                  ? "bg-primary text-primary-foreground border-primary"
                  : "bg-card text-foreground border-border hover:bg-accent")
              }
            >
              {r.label}
            </button>
          ))}
        </div>
        <button
          type="button"
          onClick={() => window.print()}
          className={
            "inline-flex items-center gap-1.5 px-3 py-1 text-xs rounded-md border cursor-pointer " +
            "bg-card text-foreground border-border hover:bg-accent"
          }
        >
          <IconPrint size={14} /> Print / PDF
        </button>
      </div>
    </header>
  );
}

/* ========================= per-channel section ========================= */
//
// Self-contained block (KPI + region rollup + leaderboard +
// chart). Two of these stack to form the full report.

function ReportChannel({
  kind, win, prevWin, bucket, rangeLabel, ready, onDone,
}: {
  kind: "elec" | "water";
  win: { from: string; to: string; bucket: string };
  prevWin: { from: string; to: string } | null;
  bucket: string;
  rangeLabel: string;
  /** When false, the channel waits before firing aggregate queries
   *  (used to serialise elec after water so the DB isn't thrashed). */
  ready: boolean;
  /** Fired once the channel has finished loading (or failed). */
  onDone: () => void;
}): React.ReactElement {
  const kindPreset = KINDS.find((k) => k.kind === kind)!;
  const accent = ROLE_ACCENT[kind];

  const [meters, setMeters] = React.useState<ReadonlyArray<MeterRow>>([]);
  const [siteTotals, setSiteTotals] = React.useState<ReadonlyArray<UsageSiteTotalRow>>([]);
  const [bucketRows, setBucketRows] = React.useState<ReadonlyArray<UsageBucketRow>>([]);
  const [perMeter,   setPerMeter]   = React.useState<ReadonlyArray<UsagePerMeterRow>>([]);
  const [prevTotals, setPrevTotals] = React.useState<ReadonlyArray<UsageSiteTotalRow>>([]);

  // Catalog
  React.useEffect(() => {
    let cancelled = false;
    fetchTemplate<MeterRow>(`${EXTENSION_ID}.meters_list`, {
      kind: kindPreset.kind,
      secondary_tag: kindPreset.secondaryTag,
      limit: 2000,
    })
      .then((rs) => { if (!cancelled) setMeters(rs); })
      .catch(() => { /* report should degrade gracefully */ });
    return () => { cancelled = true; };
  }, [kindPreset.kind, kindPreset.secondaryTag]);

  const allHosts = React.useMemo(() => {
    const seen = new Map<string, string>();
    for (const m of meters) {
      if (!seen.has(m.host_uuid)) seen.set(m.host_uuid, m.host_name ?? m.host_uuid);
    }
    return Array.from(seen, ([uuid, name]) => ({ uuid, name })).sort(
      (a, b) => a.name.localeCompare(b.name),
    );
  }, [meters]);

  // Aggregates — always for the WHOLE portfolio (no user filter
  // in the report — it's a board-level view).
  const pointUuidsCsv = React.useMemo(
    () => meters.map((m) => m.uuid).join(","),
    [meters],
  );

  React.useEffect(() => {
    if (!pointUuidsCsv) {
      setSiteTotals([]); setBucketRows([]); setPerMeter([]); setPrevTotals([]);
      onDone();
      return;
    }
    if (!ready) return; // wait for the previous channel to finish

    let cancelled = false;
    // Sequenced (not Promise.all): each warehouse aggregate over
    // ~672 elec meters × 30d is ~1s on the DB. Running them one
    // after another keeps each query fast and lets the page paint
    // progressively (KPI → chart → leaderboard). The
    // cancelled-flag check between steps aborts cleanly if the
    // range/anchor changes mid-chain.
    (async () => {
      try {
        const tot = await fetchTemplate<UsageSiteTotalRow>(
          `${EXTENSION_ID}.usage_site_totals`,
          { point_uuids: pointUuidsCsv, from: win.from, to: win.to },
        );
        if (cancelled) return;
        setSiteTotals(tot);

        const buc = await fetchTemplate<UsageBucketRow>(
          `${EXTENSION_ID}.usage_bucketed`,
          { point_uuids: pointUuidsCsv, from: win.from, to: win.to, bucket: win.bucket },
        );
        if (cancelled) return;
        setBucketRows(buc);

        const pm = await fetchTemplate<UsagePerMeterRow>(
          `${EXTENSION_ID}.usage_per_meter`,
          { point_uuids: pointUuidsCsv, from: win.from, to: win.to, limit: 50 },
        );
        if (cancelled) return;
        setPerMeter(pm);

        // Prior-window totals last — it's only used for the delta
        // badge, so the rest of the report is interactive sooner.
        if (prevWin) {
          const prev = await fetchTemplate<UsageSiteTotalRow>(
            `${EXTENSION_ID}.usage_site_totals`,
            { point_uuids: pointUuidsCsv, from: prevWin.from, to: prevWin.to },
          ).catch(() => [] as ReadonlyArray<UsageSiteTotalRow>);
          if (cancelled) return;
          setPrevTotals(prev);
        } else {
          setPrevTotals([]);
        }
      } catch {
        /* swallow — partial report is better than none */
      } finally {
        if (!cancelled) onDone();
      }
    })();

    return () => { cancelled = true; };
  }, [pointUuidsCsv, win.from, win.to, win.bucket, prevWin, ready, onDone]);

  const totalsByHost = React.useMemo(() => {
    const map = new Map<string, number>();
    for (const r of siteTotals) {
      const v = asNumber(r.total_value);
      if (v !== null) map.set(r.host_uuid, v);
    }
    return map;
  }, [siteTotals]);

  const grandTotal = Array.from(totalsByHost.values()).reduce((s, v) => s + v, 0);
  const prevGrandTotal = React.useMemo(() => {
    let s = 0;
    for (const r of prevTotals) {
      const v = asNumber(r.total_value);
      if (v !== null) s += v;
    }
    return s;
  }, [prevTotals]);
  const deltaPct: number | null =
    prevGrandTotal > 0 ? ((grandTotal - prevGrandTotal) / prevGrandTotal) * 100 : null;

  const allSelected = React.useMemo(() => allHosts.map((h) => h.uuid), [allHosts]);
  const regions = React.useMemo(
    () => buildRegions(allHosts, totalsByHost, bucketRows, allSelected, grandTotal),
    [allHosts, totalsByHost, bucketRows, allSelected, grandTotal],
  );

  const unit = inferUnit(meters) ?? kindPreset.unitHint;
  const hostName = (uuid: string): string => allHosts.find((h) => h.uuid === uuid)?.name ?? uuid;
  const Icon = kind === "elec" ? IconBolt : IconWave;

  return (
    <section className="ext-report-channel flex flex-col gap-4 print:break-inside-avoid">
      <h2 className={"flex items-center gap-2 text-xl font-semibold tracking-tight " + accent.text}>
        <Icon size={20} />
        {accent.label}
        <span className="ml-2 text-xs font-normal text-muted-foreground tracking-normal">
          {allHosts.length} sites · {meters.length} meters
        </span>
      </h2>

      {/* Headline row: big radial + 30-day trend chart side-by-side */}
      <div className="grid grid-cols-1 md:grid-cols-[280px_1fr] gap-4">
        <RadialKpi
          value={fmtBig(grandTotal)}
          unit={unit}
          deltaPct={deltaPct}
          deltaLabel={`vs prior ${rangeLabel}`}
          accent={accent}
          periodLabel={rangeLabel}
        />
        <div className="ext-glass p-3 print:!shadow-none print:!bg-white print:border-slate-200">
          <div className="flex items-center gap-1.5 ext-eyebrow text-muted-foreground mb-1">
            <IconTrend size={12} /> Trend · bucket {bucket}
          </div>
          {bucketRows.length === 0 ? (
            <div className="text-sm text-muted-foreground italic p-6">Loading data…</div>
          ) : (
            <UsageTimeSeries
              rows={bucketRows}
              hostUuids={allSelected}
              hostName={hostName}
              unit={unit}
              totalsByHost={totalsByHost}
              topN={SERIES_TOP_N}
              showAll={false}
            />
          )}
        </div>
      </div>

      {/* Region rollup — pure read-only in the report (no focus/select buttons). */}
      {regions.length > 0 ? (
        <div>
          <h3 className="flex items-center gap-1.5 text-sm font-semibold tracking-tight text-foreground mb-2">
            <IconRegion size={14} className={accent.text} /> Regional breakdown
          </h3>
          <RegionRollup
            regions={regions}
            unit={unit}
            focusRegion={null}
            onFocusRegion={() => { /* report is read-only */ }}
            onSelectRegion={() => { /* report is read-only */ }}
            onClearRegion={() => { /* report is read-only */ }}
          />
        </div>
      ) : null}

      {/* Top meters with z-score outlier flags. */}
      {perMeter.length > 0 ? (
        <div className="print:break-inside-avoid">
          <h3 className="flex items-center gap-1.5 text-sm font-semibold tracking-tight text-foreground mb-2">
            <IconAlert size={14} className={accent.text} /> Top meters by AVG
          </h3>
          <div className="ext-glass p-3 print:!shadow-none print:!bg-white print:border-slate-200">
            <TopMeters
              rows={perMeter.slice(0, 8)}
              unit={unit}
              hostName={hostName}
              allRows={perMeter}
            />
          </div>
        </div>
      ) : null}
    </section>
  );
}

function ReportFooter(): React.ReactElement {
  return (
    <footer className="pt-4 border-t border-border text-xs text-muted-foreground print:text-[10px]">
      Generated {new Date().toLocaleString()} · Rubix-OS · {EXTENSION_ID}
      <span className="float-right">Source: warehouse_query · usage_site_totals · usage_bucketed · usage_per_meter</span>
    </footer>
  );
}
