// `DashboardPage` — Energy & Water Overview orchestrator.
//
// Layout (bento, glassmorphic, dark-mode-first):
//
//   ┌──────────────── Filter rail (glass) ───────────────────┐
//   ├─ Map (hero, 7 cols) ─┬─ KPI stack (3 cols) ────────────┤
//   ├─ Region rollup ────────────────────────────────────────┤
//   ├─ Site tiles strip (sparkline + last reading) ──────────┤
//   ├─ Main time-series uPlot (per-host overlay) ────────────┤
//   ├─ Weekday × hour heatmap ───────────────────────────────┤
//   ├─ Top meters leaderboard + meter table ─────────────────┤
//   └────────────────────────────────────────────────────────┘
//
// Backed by warehouse templates:
//   • com.nubeio.rubixos.meters_list
//   • com.nubeio.rubixos.usage_site_totals
//   • com.nubeio.rubixos.usage_bucketed
//   • com.nubeio.rubixos.usage_per_meter
//   • com.nubeio.rubixos.histories_summary (range anchor)

import * as React from "react";

import { EXTENSION_ID, asEpochMs, asNumber } from "../types";
import type {
  HistoriesSummaryRow,
  MeterRow,
  UsageBucketRow,
  UsagePerMeterRow,
  UsageSiteTotalRow,
} from "../types";
import { fetchTemplate } from "../api";
import { geoForHost } from "../sites-geo";
import { SiteMap, type SiteMarker } from "../site-map";

import { KINDS, RANGES, SERIES_TOP_N, TILES_MAX } from "./presets";
import { fmtBig, inferUnit } from "./helpers";
import { Empty, LoadingToast, PillBtn, SectionHeader } from "./prims";
import { KpiCard } from "./kpi-card";
import { SiteTile } from "./site-tile";
import { FilterRail } from "./filter-rail";
import { UsageTimeSeries } from "./usage-time-series";
import { UsageHeatmap } from "./usage-heatmap";
import { TopMeters } from "./top-meters";
import { MeterTable } from "./meter-table";
import { RegionRollup, buildRegions } from "./region-rollup";
import { PortfolioTable, type PortfolioRow } from "./portfolio-table";
import { exportChartPng, exportUsageCsv } from "./csv-export";
import { stateOf } from "./helpers";
import { ROLE_ACCENT, IconBolt, IconClock, IconGrid, IconList, IconMap, IconMapPin, IconRegion, IconTrend, IconWave, IconAlert, IconGauge, IconHash, IconLayers } from "./icons";
import { RadialKpi } from "./radial-kpi";
import { readUrlState, useUrlSync } from "./url-state";

export function DashboardPage(): React.ReactElement {
  // Initial state honours `?kind=…&range=…` so links like
  //   /extensions/com.nubeio.rubixos/usage?kind=water&range=6m
  // are shareable. Subsequent changes are mirrored back to the URL
  // by `useUrlSync` below.
  const urlInit = React.useRef(readUrlState()).current;
  const [kindIdx, setKindIdx] = React.useState(urlInit.kindIdx ?? 0);
  const [rangeIdx, setRangeIdx] = React.useState(urlInit.rangeIdx ?? 1); // default 7d
  useUrlSync(kindIdx, rangeIdx);
  const [selectedHosts, setSelectedHosts] = React.useState<ReadonlyArray<string>>([]);
  // Site list view + filter state (scale to 100+ buildings).
  const [siteQuery, setSiteQuery] = React.useState("");
  const [showAllSeries, setShowAllSeries] = React.useState(false);

  const kindPreset = KINDS[kindIdx]!;
  const range = RANGES[rangeIdx]!;

  // --- Time window: anchor to latest sample in warehouse so the
  // bundled static dump (ends ~2025-12-06) is visible without a
  // manual range pick.
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
    const to = new Date(toMs);
    const from = new Date(toMs - range.hours * HOUR);
    return { from: from.toISOString(), to: to.toISOString(), bucket: range.bucket };
  }, [rangeIdx, range.hours, range.bucket, latestSampleMs]);

  // Prior identical window — used for the period-over-period delta
  // on the headline KPI. Anchored to `win.from` so it ends exactly
  // where the current window starts (no gap, no overlap).
  //
  // Skipped at 6m / 1y: the extra usage_site_totals round-trip
  // costs ~3s on elec just to compute one badge. The KPI cleanly
  // hides the delta when prevWin is null.
  const prevWin = React.useMemo(() => {
    const toMs = Date.parse(win.from);
    if (!Number.isFinite(toMs)) return null;
    if (range.hours > 2160) return null;
    const fromMs = toMs - range.hours * 3600_000;
    return { from: new Date(fromMs).toISOString(), to: new Date(toMs).toISOString() };
  }, [win.from, range.hours]);

  // --- 1. Meter catalog for the current channel.
  const [meters, setMeters] = React.useState<ReadonlyArray<MeterRow>>([]);
  const [metersLoading, setMetersLoading] = React.useState(false);
  const [metersError, setMetersError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    setMetersLoading(true);
    setMetersError(null);
    fetchTemplate<MeterRow>(`${EXTENSION_ID}.meters_list`, {
      kind: kindPreset.kind,
      secondary_tag: kindPreset.secondaryTag,
      limit: 2000,
    })
      .then((rs) => { if (!cancelled) setMeters(rs); })
      .catch((e) => { if (!cancelled) setMetersError(e instanceof Error ? e.message : String(e)); })
      .finally(() => { if (!cancelled) setMetersLoading(false); });
    return () => { cancelled = true; };
  }, [kindPreset.kind, kindPreset.secondaryTag]);

  // --- Hosts present in the current meter set.
  const allHosts = React.useMemo(() => {
    const seen = new Map<string, string>();
    for (const m of meters) {
      if (!seen.has(m.host_uuid)) seen.set(m.host_uuid, m.host_name ?? m.host_uuid);
    }
    return Array.from(seen, ([uuid, name]) => ({ uuid, name })).sort(
      (a, b) => a.name.localeCompare(b.name),
    );
  }, [meters]);

  // Default-select all when the host set arrives.
  React.useEffect(() => {
    if (allHosts.length === 0) { setSelectedHosts([]); return; }
    setSelectedHosts(allHosts.map((h) => h.uuid));
  }, [allHosts]);

  const pointUuidsCsv = React.useMemo(() => {
    const hosts = new Set(selectedHosts);
    return meters.filter((m) => hosts.has(m.host_uuid)).map((m) => m.uuid).join(",");
  }, [meters, selectedHosts]);

  // --- 2/3/4. Aggregates — fire together when selection changes.
  const [siteTotals, setSiteTotals] = React.useState<ReadonlyArray<UsageSiteTotalRow>>([]);
  const [bucketRows, setBucketRows] = React.useState<ReadonlyArray<UsageBucketRow>>([]);
  const [perMeter,   setPerMeter]   = React.useState<ReadonlyArray<UsagePerMeterRow>>([]);
  const [prevTotals, setPrevTotals] = React.useState<ReadonlyArray<UsageSiteTotalRow>>([]);
  const [aggLoading, setAggLoading] = React.useState(false);
  const [aggError,   setAggError]   = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!pointUuidsCsv) {
      setSiteTotals([]); setBucketRows([]); setPerMeter([]); setPrevTotals([]);
      return;
    }
    let cancelled = false;
    setAggLoading(true);
    setAggError(null);

    // Sequenced (not Promise.all): warehouse aggregates over long
    // ranges (6m, 1y) are heavy and concurrent queries thrash the
    // DB. Running them one after another keeps each query fast,
    // lets the user see KPIs + map first, and avoids HTTP/2 stream
    // contention on slow links. The cancelled-flag check between
    // steps means a kind/range/host change aborts mid-chain.
    (async () => {
      try {
        // 1) Site totals — fastest, drives KPI + map immediately.
        const tot = await fetchTemplate<UsageSiteTotalRow>(
          `${EXTENSION_ID}.usage_site_totals`,
          { point_uuids: pointUuidsCsv, from: win.from, to: win.to },
        );
        if (cancelled) return;
        setSiteTotals(tot);

        // 2) Bucketed series — drives the main chart + heatmap.
        const buc = await fetchTemplate<UsageBucketRow>(
          `${EXTENSION_ID}.usage_bucketed`,
          { point_uuids: pointUuidsCsv, from: win.from, to: win.to, bucket: win.bucket },
        );
        if (cancelled) return;
        setBucketRows(buc);

        // 3) Per-meter leaderboard.
        const pm = await fetchTemplate<UsagePerMeterRow>(
          `${EXTENSION_ID}.usage_per_meter`,
          { point_uuids: pointUuidsCsv, from: win.from, to: win.to, limit: 50 },
        );
        if (cancelled) return;
        setPerMeter(pm);

        // 4) Prior-window totals for the KPI delta. Failure here
        //    is non-fatal — we just skip the delta badge.
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
      } catch (e) {
        if (!cancelled) setAggError(e instanceof Error ? e.message : String(e));
      } finally {
        if (!cancelled) setAggLoading(false);
      }
    })();

    return () => { cancelled = true; };
  }, [pointUuidsCsv, win.from, win.to, win.bucket, prevWin]);

  const unit = inferUnit(meters) ?? kindPreset.unitHint;
  const hostName = (uuid: string): string => allHosts.find((h) => h.uuid === uuid)?.name ?? uuid;

  // --- Loading toast: small dismissible indicator, auto re-shows
  // whenever a new fetch round begins.
  const isLoading = metersLoading || aggLoading;
  const [loadDismissed, setLoadDismissed] = React.useState(false);
  React.useEffect(() => {
    if (isLoading) setLoadDismissed(false);
  }, [isLoading]);
  const toggleHost = (uuid: string) =>
    setSelectedHosts((sel) => sel.includes(uuid) ? sel.filter((s) => s !== uuid) : [...sel, uuid]);

  // ---- Derived rollups for KPI cards & tiles ------------------------
  const totalsByHost = React.useMemo(() => {
    const map = new Map<string, number>();
    for (const r of siteTotals) {
      const v = asNumber(r.total_value);
      if (v !== null) map.set(r.host_uuid, v);
    }
    return map;
  }, [siteTotals]);

  const grandTotal = Array.from(totalsByHost.values()).reduce((s, v) => s + v, 0);
  // Prior-window grand total — restricted to the SAME selected
  // hosts, so the delta isn't skewed by sites the user toggled off.
  const prevGrandTotal = React.useMemo(() => {
    const sel = new Set(selectedHosts);
    let s = 0;
    for (const r of prevTotals) {
      if (!sel.has(r.host_uuid)) continue;
      const v = asNumber(r.total_value);
      if (v !== null) s += v;
    }
    return s;
  }, [prevTotals, selectedHosts]);
  const deltaPct: number | null =
    prevGrandTotal > 0 ? ((grandTotal - prevGrandTotal) / prevGrandTotal) * 100 : null;
  const filteredMeterCount = meters.filter((m) => selectedHosts.includes(m.host_uuid)).length;
  const totalSamples = bucketRows.reduce((s, r) => s + (Number(r.sample_count) || 0), 0);
  const latestBucket = bucketRows.length
    ? Math.max(...bucketRows.map((r) => asEpochMs(r.bucket) ?? 0))
    : 0;

  // Map markers — one per selected site, sized by share of total.
  const markers: ReadonlyArray<SiteMarker> = React.useMemo(() => {
    return allHosts
      .map((h) => {
        const geo = geoForHost(h.uuid);
        if (!geo) return null;
        return {
          site: geo,
          value: totalsByHost.get(h.uuid) ?? 0,
          selected: selectedHosts.includes(h.uuid),
        };
      })
      .filter((m): m is SiteMarker => m !== null);
  }, [allHosts, totalsByHost, selectedHosts]);

  // Per-host sparkline data (for the strip below the map).
  const sparkByHost = React.useMemo(() => {
    const byHost = new Map<string, Array<{ t: number; v: number | null }>>();
    for (const r of bucketRows) {
      const t = asEpochMs(r.bucket);
      if (t === null) continue;
      const v = asNumber(r.avg_value);
      const list = byHost.get(r.host_uuid) ?? [];
      list.push({ t, v });
      byHost.set(r.host_uuid, list);
    }
    for (const list of byHost.values()) list.sort((a, b) => a.t - b.t);
    return byHost;
  }, [bucketRows]);

  // Regional rollup (by Australian state from `sites-geo.ts`).
  const regions = React.useMemo(
    () => buildRegions(allHosts, totalsByHost, bucketRows, selectedHosts, grandTotal),
    [allHosts, totalsByHost, bucketRows, selectedHosts, grandTotal],
  );

  // Flat per-site rows used by the portfolio table view.
  const portfolioRows: ReadonlyArray<PortfolioRow> = React.useMemo(() => {
    return allHosts.map((h) => {
      const points = sparkByHost.get(h.uuid) ?? [];
      const last = points.length ? points[points.length - 1]!.v : null;
      const total = totalsByHost.get(h.uuid) ?? 0;
      return {
        host_uuid: h.uuid,
        name: h.name,
        region: geoForHost(h.uuid)?.locality ?? "—",
        total,
        last,
        share: grandTotal > 0 ? total / grandTotal : 0,
        spark: points.map((p) => p.v),
      };
    });
  }, [allHosts, sparkByHost, totalsByHost, grandTotal]);

  const useTable = allHosts.length > TILES_MAX;

  // Region drill-in: when set, narrows the site list (tiles /
  // portfolio table) and the map markers to that state's hosts.
  // KPIs and chart continue to follow `selectedHosts` so users
  // can still compare a focused region against everything they
  // had selected before drilling in.
  const [focusRegion, setFocusRegion] = React.useState<string | null>(null);
  // Clear focus if the region disappears (e.g. kind switch).
  React.useEffect(() => {
    if (focusRegion && !regions.some((r) => r.state === focusRegion)) {
      setFocusRegion(null);
    }
  }, [regions, focusRegion]);

  const inFocus = React.useCallback((uuid: string): boolean => {
    if (!focusRegion) return true;
    return stateOf(geoForHost(uuid)?.locality) === focusRegion;
  }, [focusRegion]);

  const visibleHosts = React.useMemo(
    () => focusRegion ? allHosts.filter((h) => inFocus(h.uuid)) : allHosts,
    [allHosts, focusRegion, inFocus],
  );
  const visiblePortfolioRows = React.useMemo(
    () => focusRegion ? portfolioRows.filter((r) => inFocus(r.host_uuid)) : portfolioRows,
    [portfolioRows, focusRegion, inFocus],
  );
  const visibleMarkers = React.useMemo(
    () => focusRegion ? markers.filter((m) => inFocus(m.site.host_uuid)) : markers,
    [markers, focusRegion, inFocus],
  );

  // Bulk region selectors (used by RegionRollup).
  const selectRegion = React.useCallback((uuids: ReadonlyArray<string>) => {
    setSelectedHosts((sel) => {
      const set = new Set(sel);
      for (const u of uuids) set.add(u);
      return Array.from(set);
    });
  }, []);
  const clearRegion = React.useCallback((uuids: ReadonlyArray<string>) => {
    const drop = new Set(uuids);
    setSelectedHosts((sel) => sel.filter((u) => !drop.has(u)));
  }, []);

  return (
    <div className="ext-dash-shell flex flex-col gap-4">
      {/* ─── Filter rail ─────────────────────────────────────── */}
      <FilterRail
        kindIdx={kindIdx} setKindIdx={setKindIdx}
        rangeIdx={rangeIdx} setRangeIdx={setRangeIdx}
        allHosts={allHosts}
        selectedHosts={selectedHosts} setSelectedHosts={setSelectedHosts}
        latestSampleMs={latestSampleMs}
      />

      {(metersError || aggError) ? (
        <div role="alert" className="ext-glass px-3 py-2 text-sm text-destructive">
          {metersError ?? aggError}
        </div>
      ) : null}

      <LoadingToast show={isLoading && !loadDismissed} onClose={() => setLoadDismissed(true)} />

      {/* ─── Hero: radial KPI + map + secondary stats ───────── */}
      {/*
        Three-column bento with strong asymmetry:
        ┌─────────┬───────────────────────┬────────┐
        │  RADIAL │         MAP           │  meta  │
        │   KPI   │   (geographic hero)   │ stack  │
        └─────────┴───────────────────────┴────────┘
        At lg+ the radial gauge anchors the eye first, the map
        provides spatial context, and the meta stack provides
        scan-ready counts. On smaller screens it collapses
        gracefully to a single column.
      */}
      <div className="grid grid-cols-1 lg:grid-cols-[minmax(260px,320px)_1fr_minmax(220px,260px)] gap-4">
        <RadialKpi
          value={fmtBig(grandTotal)}
          unit={unit}
          deltaPct={deltaPct}
          deltaLabel={`vs prior ${range.label}`}
          accent={ROLE_ACCENT[kindPreset.kind]}
          periodLabel={range.label}
        />

        <section className="ext-glass p-2 relative">
          <div className="absolute top-3 left-3 z-10 flex items-center gap-1.5 ext-eyebrow text-muted-foreground bg-background/40 backdrop-blur-sm px-2 py-0.5 rounded">
            <IconMapPin size={12} />
            <span>{visibleMarkers.length} sites mapped</span>
          </div>
          <SiteMap markers={visibleMarkers} unit={unit} onToggleSite={toggleHost} height={380} />
        </section>

        <section className="grid grid-rows-3 gap-3">
          <KpiCard
            icon={<IconGauge size={12} />}
            eyebrow="Meters"
            value={filteredMeterCount.toLocaleString()}
            sub={`of ${meters.length.toLocaleString()} in catalog`}
          />
          <KpiCard
            icon={<IconLayers size={12} />}
            eyebrow="Sites"
            value={`${selectedHosts.length} / ${allHosts.length}`}
            sub={`${regions.length} region${regions.length === 1 ? "" : "s"}`}
          />
          <KpiCard
            icon={<IconClock size={12} />}
            eyebrow="Latest sample"
            value={latestBucket ? new Date(latestBucket).toLocaleDateString() : "—"}
            sub={latestBucket
              ? `${new Date(latestBucket).toLocaleTimeString()} · ${totalSamples >= 1000 ? `${(totalSamples / 1000).toFixed(totalSamples >= 10000 ? 0 : 1)}k` : totalSamples.toLocaleString()} samples`
              : undefined}
          />
        </section>
      </div>

      {/* ─── Regional rollup (scales to any site count) ─────── */}
      {regions.length > 0 ? (
        <section>
          <SectionHeader
            icon={<IconRegion size={14} />}
            title="By region"
            subtitle={`${regions.length} state${regions.length === 1 ? "" : "s"} · ${kindPreset.label.toLowerCase()} · ${range.label}`}
          />
          <RegionRollup
            regions={regions}
            unit={unit}
            focusRegion={focusRegion}
            onFocusRegion={setFocusRegion}
            onSelectRegion={selectRegion}
            onClearRegion={clearRegion}
          />
        </section>
      ) : null}

      {/* ─── Sites: tiles (small portfolio) or table (scaled) ── */}
      <section>
        <SectionHeader
          icon={<IconGrid size={14} />}
          title={focusRegion ? `Sites — ${focusRegion}` : "Sites"}
          subtitle={`${kindPreset.label.toLowerCase()} · ${range.label} window · ${visibleHosts.length}${focusRegion ? ` of ${allHosts.length}` : " total"}`}
          right={
            <div className="flex items-center gap-2">
              {focusRegion ? (
                <PillBtn onClick={() => setFocusRegion(null)}>✕ clear focus</PillBtn>
              ) : null}
              <span className="ext-eyebrow">
                {useTable ? "click row to toggle" : "click tile to toggle"}
              </span>
            </div>
          }
        />
        {useTable ? (
          <PortfolioTable
            rows={visiblePortfolioRows}
            unit={unit}
            selectedHosts={selectedHosts}
            onToggleHost={toggleHost}
            query={siteQuery}
            setQuery={setSiteQuery}
          />
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3">
            {visibleHosts.map((h) => {
              const total = totalsByHost.get(h.uuid) ?? 0;
              const points = sparkByHost.get(h.uuid) ?? [];
              const last = points.length ? points[points.length - 1]!.v : null;
              const max = points.reduce((m, p) => Math.max(m, p.v ?? 0), 0);
              const pct = max > 0 ? Math.min(100, ((last ?? 0) / max) * 100) : 0;
              const on = selectedHosts.includes(h.uuid);
              return (
                <SiteTile
                  key={h.uuid}
                  name={h.name}
                  locality={geoForHost(h.uuid)?.locality ?? null}
                  total={total}
                  last={last}
                  pct={pct}
                  spark={points.map((p) => p.v)}
                  unit={unit}
                  selected={on}
                  onClick={() => toggleHost(h.uuid)}
                />
              );
            })}
          </div>
        )}
      </section>

      {/* ─── Main time-series (capped) ─────────────────────────── */}
      <section className="ext-glass p-4" data-chart="usage-ts">
        <SectionHeader
          icon={kindPreset.kind === "elec" ? <IconBolt size={14} className={ROLE_ACCENT.elec.text} /> : <IconWave size={14} className={ROLE_ACCENT.water.text} />}
          title={`${kindPreset.label} usage over time`}
          subtitle={`bucket = ${range.bucket} · ${unit ?? ""}`}
          right={
            <div className="flex items-center gap-3">
              <span className="ext-eyebrow">
                {aggLoading ? "loading…" : `${bucketRows.length.toLocaleString()} rows`}
              </span>
              {bucketRows.length > 0 ? (
                <button
                  type="button"
                  onClick={() => exportUsageCsv(
                    bucketRows, selectedHosts, hostName,
                    kindPreset.label, range.label, unit,
                  )}
                  className={
                    "px-2 py-0.5 text-xs rounded-md border cursor-pointer transition-colors " +
                    "bg-transparent text-muted-foreground border-border/40 hover:bg-accent hover:text-foreground"
                  }
                  title="Download visible series as CSV (long format)"
                  aria-label="Export time-series as CSV"
                >
                  ↓ CSV
                </button>
              ) : null}
              {bucketRows.length > 0 ? (
                <button
                  type="button"
                  onClick={() => exportChartPng(
                    `rubixos-${kindPreset.label.toLowerCase()}-${range.label}`,
                  )}
                  className={
                    "px-2 py-0.5 text-xs rounded-md border cursor-pointer transition-colors " +
                    "bg-transparent text-muted-foreground border-border/40 hover:bg-accent hover:text-foreground"
                  }
                  title="Download chart as PNG"
                  aria-label="Export chart as PNG"
                >
                  ↓ PNG
                </button>
              ) : null}
              {selectedHosts.length > SERIES_TOP_N ? (
                <button
                  type="button"
                  onClick={() => setShowAllSeries((v) => !v)}
                  className={
                    "px-2 py-0.5 text-xs rounded-md border cursor-pointer transition-colors " +
                    (showAllSeries
                      ? "bg-primary/15 text-foreground border-primary/50"
                      : "bg-transparent text-muted-foreground border-border/40 hover:bg-accent")
                  }
                  aria-pressed={showAllSeries}
                  title={showAllSeries
                    ? "Show only Total + top sites"
                    : `Show all ${selectedHosts.length} site series`}
                >
                  {showAllSeries
                    ? `all ${selectedHosts.length}`
                    : `top ${SERIES_TOP_N} + total`}
                </button>
              ) : null}
            </div>
          }
        />
        {bucketRows.length === 0 ? (
          <Empty>{aggLoading ? "loading…" : "No samples in this window."}</Empty>
        ) : (
          <UsageTimeSeries
            rows={bucketRows}
            hostUuids={selectedHosts}
            hostName={hostName}
            unit={unit}
            totalsByHost={totalsByHost}
            topN={SERIES_TOP_N}
            showAll={showAllSeries}
          />
        )}
      </section>

      {/* ─── Weekday × hour heatmap (when bucket ≤ 1h) ──────── */}
      {range.bucket === "15 minutes" || range.bucket === "1 hour" ? (
        <section className="ext-glass p-4">
          <SectionHeader
            icon={<IconClock size={14} />}
            title="Weekday × hour"
            subtitle={`avg ${kindPreset.label.toLowerCase()} usage · brighter = higher`}
            right={<span className="ext-eyebrow">local time</span>}
          />
          <UsageHeatmap rows={bucketRows} selectedHosts={selectedHosts} unit={unit} />
        </section>
      ) : null}

      {/* ─── Leaderboard + meter table ────────────────────────── */}
      <div className="grid grid-cols-1 lg:grid-cols-[1fr_2fr] gap-4">
        <section className="ext-glass p-4">
          <SectionHeader
            icon={<IconAlert size={14} />}
            title="Top meters"
            subtitle="by AVG in window · ⚠ marks z-score ≥ 2"
          />
          <TopMeters rows={perMeter.slice(0, 10)} unit={unit} hostName={hostName} allRows={perMeter} />
        </section>
        <section className="ext-glass p-4">
          <SectionHeader
            icon={<IconList size={14} />}
            title="Meters"
            subtitle={metersLoading ? "loading…" : `${filteredMeterCount} of ${meters.length}`}
          />
          <MeterTable meters={meters.filter((m) => selectedHosts.includes(m.host_uuid))} />
        </section>
      </div>
    </div>
  );
}
