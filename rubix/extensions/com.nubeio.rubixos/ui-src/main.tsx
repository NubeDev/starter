// `main.tsx` — Main panel for `com.nubeio.rubixos`.
//
// Renders a BMS dashboard on top of the warehouse-template results:
//
//  - KPI cards (sample count, point count, earliest/latest sample)
//  - Hosts / Networks / Devices overview tables
//  - Point picker + time-range selector + bucketed history chart
//
// All data flows through `com.nubeio.rubixos.warehouse_query` →
// `ctx.warehouse_read().query(template, params)` on the agent side.

import * as React from "react";
import "./app.css";

import {
  BlockShell,
  useExtensionRoute,
  useHostTheme,
  useSlotContext,
} from "@nube/starter-ext-sdk-ts";

import {
  EXTENSION_ID,
  asNumber,
  type DeviceOverviewRow,
  type ExtensionDetail,
  type HistoriesSummaryRow,
  type HistoryBucketRow,
  type HostOverviewRow,
  type NetworkOverviewRow,
  type PointRow,
} from "./types";
import { fetchTemplate } from "./api";
import { fetchExtensionDetail, invalidateExtensionDetail } from "./detail";
import { HistoryLineChart } from "./chart";
import { DashboardPage } from "./dashboard";

export default function Main(): React.ReactElement {
  return (
    <BlockShell>
      <MainRouter />
    </BlockShell>
  );
}

function MainRouter(): React.ReactElement {
  const route = useExtensionRoute();
  if (route === "hosts")    return <HostsPage />;
  if (route === "networks") return <NetworksPage />;
  if (route === "devices")  return <DevicesPage />;
  if (route === "history" || route?.startsWith("history/")) return <HistoryPage />;
  if (route === "usage" || route?.startsWith("usage/")) return <UsagePage />;
  return <OverviewPage />;
}

/* ============================== Usage =============================== */

// Thin wrapper that frames `<DashboardPage />` inside the same
// `Page`/`Header` chrome the other routes use, so it inherits the
// theme + slot data-attrs and the refresh affordance (no-op here
// since the dashboard manages its own data lifecycle).
function UsagePage(): React.ReactElement {
  const slot = useSlotContext();
  const theme = useHostTheme();
  return (
    <Page
      slot={slot.slotId}
      theme={theme.mode}
      header={
        <div>
          <h3 className="text-lg font-semibold tracking-tight">Energy &amp; Water Overview</h3>
          <p className="text-sm text-muted-foreground">
            {EXTENSION_ID}.meters_list · usage_site_totals · usage_bucketed
          </p>
        </div>
      }
      error={null}
    >
      <DashboardPage />
    </Page>
  );
}

/* ============================== Overview =============================== */

function OverviewPage(): React.ReactElement {
  const slot = useSlotContext();
  const theme = useHostTheme();

  const [detail, setDetail] = React.useState<ExtensionDetail | null>(null);
  const [summary, setSummary] = React.useState<HistoriesSummaryRow | null>(null);
  const [hosts, setHosts] = React.useState<ReadonlyArray<HostOverviewRow>>([]);
  const [error, setError] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(false);
  const [tick, setTick] = React.useState(0);

  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    Promise.all([
      fetchExtensionDetail(),
      fetchTemplate<HistoriesSummaryRow>(`${EXTENSION_ID}.histories_summary`, {}),
      fetchTemplate<HostOverviewRow>(`${EXTENSION_ID}.hosts_overview`, { limit: 25 }),
    ])
      .then(([d, s, h]) => {
        if (cancelled) return;
        setDetail(d);
        setSummary(s[0] ?? null);
        setHosts(h);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => { cancelled = true; };
  }, [tick]);

  const totalRows = summary ? Number(summary.sample_count) : null;

  return (
    <Page
      slot={slot.slotId}
      theme={theme.mode}
      header={
        <Header
          subtitle="Nube-iO Rubix-OS BMS — devices · points · histories"
          version={detail?.manifest?.version}
          onRefresh={() => { invalidateExtensionDetail(); setTick((t) => t + 1); }}
          loading={loading}
        />
      }
      error={error}
    >
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <Kpi label="Samples" value={fmtInt(totalRows)} />
        <Kpi label="Points (with history)" value={fmtInt(summary?.point_count ?? null)} />
        <Kpi label="Earliest" value={fmtTs(summary?.earliest ?? null)} />
        <Kpi label="Latest"   value={fmtTs(summary?.latest ?? null)} />
      </div>

      <Card title="Hosts" description={`${EXTENSION_ID}.hosts_overview`}>
        {hosts.length === 0 ? (
          <Empty>No hosts. Run <code>scripts/load-dump.sh</code> to ingest a dump.</Empty>
        ) : (
          <Table headers={["Host", "Networks", "Devices", "Points"]}>
            {hosts.map((h) => (
              <tr key={h.host_uuid} className="border-t border-border/60">
                <Td>
                  <a className="text-primary hover:underline"
                     href={`/extensions/${EXTENSION_ID}/networks?host=${encodeURIComponent(h.host_uuid)}`}>
                    {h.host_name ?? <Mono>{h.host_uuid}</Mono>}
                  </a>
                  {h.host_description ? (
                    <span className="text-muted-foreground text-xs"> · {h.host_description}</span>
                  ) : null}
                </Td>
                <Td align="right">{fmtInt(h.network_count)}</Td>
                <Td align="right">{fmtInt(h.device_count)}</Td>
                <Td align="right">{fmtInt(h.point_count)}</Td>
              </tr>
            ))}
          </Table>
        )}
      </Card>

      <Card title="Contributions" description="declared in block.yaml">
        <ContribGrid detail={detail} />
      </Card>
    </Page>
  );
}

/* =============================== Hosts ================================= */

function HostsPage(): React.ReactElement {
  const rows = useTemplate<HostOverviewRow>(`${EXTENSION_ID}.hosts_overview`, { limit: 200 });
  return (
    <SimplePage title="Hosts" template={`${EXTENSION_ID}.hosts_overview`} rows={rows}>
      {(data) => (
        <Table headers={["Host", "Networks", "Devices", "Points"]}>
          {data.map((h) => (
            <tr key={h.host_uuid} className="border-t border-border/60">
              <Td>{h.host_name ?? <Mono>{h.host_uuid}</Mono>}</Td>
              <Td align="right">{fmtInt(h.network_count)}</Td>
              <Td align="right">{fmtInt(h.device_count)}</Td>
              <Td align="right">{fmtInt(h.point_count)}</Td>
            </tr>
          ))}
        </Table>
      )}
    </SimplePage>
  );
}

/* ============================= Networks ================================ */

function NetworksPage(): React.ReactElement {
  const rows = useTemplate<NetworkOverviewRow>(`${EXTENSION_ID}.networks_overview`, { limit: 200 });
  return (
    <SimplePage title="Networks" template={`${EXTENSION_ID}.networks_overview`} rows={rows}>
      {(data) => (
        <Table headers={["Network", "Host", "Devices", "Points"]}>
          {data.map((n) => (
            <tr key={n.network_uuid} className="border-t border-border/60">
              <Td>
                {n.network_name ?? <Mono>{n.network_uuid}</Mono>}
                {n.network_description ? (
                  <span className="text-muted-foreground text-xs"> · {n.network_description}</span>
                ) : null}
              </Td>
              <Td>{n.host_name ?? <Mono>{n.host_uuid ?? ""}</Mono>}</Td>
              <Td align="right">{fmtInt(n.device_count)}</Td>
              <Td align="right">{fmtInt(n.point_count)}</Td>
            </tr>
          ))}
        </Table>
      )}
    </SimplePage>
  );
}

/* ============================== Devices ================================ */

function DevicesPage(): React.ReactElement {
  const rows = useTemplate<DeviceOverviewRow>(`${EXTENSION_ID}.devices_overview`, { limit: 200 });
  return (
    <SimplePage title="Devices" template={`${EXTENSION_ID}.devices_overview`} rows={rows}>
      {(data) => (
        <Table headers={["Device", "Network", "Host", "Points"]}>
          {data.map((d) => (
            <tr key={d.device_uuid} className="border-t border-border/60">
              <Td>
                <a className="text-primary hover:underline"
                   href={`/extensions/${EXTENSION_ID}/history?device=${encodeURIComponent(d.device_uuid)}`}>
                  {d.device_name ?? <Mono>{d.device_uuid}</Mono>}
                </a>
                {d.device_description ? (
                  <span className="text-muted-foreground text-xs"> · {d.device_description}</span>
                ) : null}
              </Td>
              <Td>{d.network_name ?? <Mono>{d.network_uuid ?? ""}</Mono>}</Td>
              <Td>{d.host_name ?? <Mono>{d.host_uuid ?? ""}</Mono>}</Td>
              <Td align="right">{fmtInt(d.point_count)}</Td>
            </tr>
          ))}
        </Table>
      )}
    </SimplePage>
  );
}

/* ============================== History ================================ */

const RANGES = [
  { label: "1h",   hours: 1,    bucket: "1 minute"  },
  { label: "6h",   hours: 6,    bucket: "5 minutes" },
  { label: "24h",  hours: 24,   bucket: "15 minutes"},
  { label: "7d",   hours: 168,  bucket: "1 hour"    },
  { label: "30d",  hours: 720,  bucket: "6 hours"   },
  { label: "1y",   hours: 8760, bucket: "1 day"     },
] as const;

function HistoryPage(): React.ReactElement {
  const slot = useSlotContext();
  const theme = useHostTheme();

  const params = React.useMemo(() => new URLSearchParams(window.location.search), []);
  const deviceFilter = params.get("device") ?? "";

  const [points, setPoints] = React.useState<ReadonlyArray<PointRow>>([]);
  const [pointsLoading, setPointsLoading] = React.useState(false);
  const [pointsError, setPointsError] = React.useState<string | null>(null);

  const [selected, setSelected] = React.useState<string | null>(null);
  const [rangeIdx, setRangeIdx] = React.useState(2); // 24h
  const [buckets, setBuckets] = React.useState<ReadonlyArray<HistoryBucketRow>>([]);
  const [chartLoading, setChartLoading] = React.useState(false);
  const [chartError, setChartError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    setPointsLoading(true);
    const tpl = deviceFilter
      ? { template: `${EXTENSION_ID}.points_by_device`, params: { device_uuid: deviceFilter, limit: 500 } }
      : { template: `${EXTENSION_ID}.points_list`,      params: { limit: 200, offset: 0 } };
    fetchTemplate<PointRow>(tpl.template, tpl.params)
      .then((rs) => {
        if (cancelled) return;
        setPoints(rs);
        if (rs.length > 0 && !selected) setSelected(rs[0]!.uuid);
      })
      .catch((e: unknown) =>
        !cancelled && setPointsError(e instanceof Error ? e.message : String(e)),
      )
      .finally(() => !cancelled && setPointsLoading(false));
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [deviceFilter]);

  React.useEffect(() => {
    if (!selected) {
      setBuckets([]);
      return;
    }
    let cancelled = false;
    setChartLoading(true);
    setChartError(null);
    const r = RANGES[rangeIdx]!;
    const to = new Date();
    const from = new Date(to.getTime() - r.hours * 3600_000);
    fetchTemplate<HistoryBucketRow>(`${EXTENSION_ID}.history_bucketed`, {
      point_uuid: selected,
      from: from.toISOString(),
      to: to.toISOString(),
      bucket: r.bucket,
    })
      .then((rs) => !cancelled && setBuckets(rs))
      .catch((e: unknown) =>
        !cancelled && setChartError(e instanceof Error ? e.message : String(e)),
      )
      .finally(() => !cancelled && setChartLoading(false));
    return () => { cancelled = true; };
  }, [selected, rangeIdx]);

  const selectedPoint = points.find((p) => p.uuid === selected) ?? null;

  return (
    <Page
      slot={slot.slotId}
      theme={theme.mode}
      header={
        <div className="flex items-center justify-between gap-4">
          <div>
            <h3 className="text-lg font-semibold tracking-tight">History</h3>
            <p className="text-sm text-muted-foreground">
              {EXTENSION_ID}.history_bucketed · Timescale {`time_bucket()`} aggregate
              {deviceFilter ? <> · filtered to device <Mono>{deviceFilter}</Mono></> : null}
            </p>
          </div>
        </div>
      }
      error={pointsError ?? chartError}
    >
      <div className="grid grid-cols-1 md:grid-cols-[260px_1fr] gap-4">
        {/* Point picker */}
        <Card title="Points" description={pointsLoading ? "loading…" : `${points.length} points`}>
          <div className="max-h-[60vh] overflow-y-auto -mx-1">
            {points.map((p) => (
              <button
                key={p.uuid}
                type="button"
                onClick={() => setSelected(p.uuid)}
                className={
                  "w-full text-left px-2 py-1.5 rounded text-sm hover:bg-accent " +
                  (p.uuid === selected ? "bg-accent text-accent-foreground" : "text-foreground/85")
                }
              >
                <div className="truncate">{p.name ?? p.uuid}</div>
                {p.device_name ? (
                  <div className="truncate text-xs text-muted-foreground">{p.device_name}</div>
                ) : null}
              </button>
            ))}
            {points.length === 0 && !pointsLoading ? (
              <Empty>No points. Ingest the dump first.</Empty>
            ) : null}
          </div>
        </Card>

        {/* Chart */}
        <Card
          title={selectedPoint?.name ?? "Select a point"}
          description={
            selectedPoint ? (
              <>
                <Mono>{selectedPoint.uuid}</Mono>
                {selectedPoint.device_name ? <> · {selectedPoint.device_name}</> : null}
                {selectedPoint.network_name ? <> · {selectedPoint.network_name}</> : null}
              </>
            ) : (
              "pick a point from the list to chart its history"
            )
          }
        >
          <div className="flex items-center gap-1 mb-3">
            {RANGES.map((r, i) => (
              <button
                key={r.label}
                type="button"
                onClick={() => setRangeIdx(i)}
                className={
                  "px-2 py-1 text-xs rounded border transition-colors " +
                  (i === rangeIdx
                    ? "bg-primary text-primary-foreground border-primary"
                    : "bg-transparent text-foreground border-border/60 hover:bg-accent")
                }
              >
                {r.label}
              </button>
            ))}
            <span className="ml-3 text-xs text-muted-foreground">
              bucket = {RANGES[rangeIdx]!.bucket}
            </span>
          </div>
          {chartLoading ? (
            <Empty>loading…</Empty>
          ) : buckets.length === 0 ? (
            <Empty>No samples in the selected range.</Empty>
          ) : (
            <div className="text-primary">
              <HistoryLineChart rows={buckets} />
              <div className="grid grid-cols-4 gap-2 mt-3 text-xs text-muted-foreground">
                <KpiSm label="buckets" value={String(buckets.length)} />
                <KpiSm label="min" value={fmtNum(Math.min(...bucketAvgs(buckets)))} />
                <KpiSm label="max" value={fmtNum(Math.max(...bucketAvgs(buckets)))} />
                <KpiSm label="avg" value={fmtNum(mean(bucketAvgs(buckets)))} />
              </div>
            </div>
          )}
        </Card>
      </div>
    </Page>
  );
}

function bucketAvgs(rows: ReadonlyArray<HistoryBucketRow>): number[] {
  return rows.map((r) => asNumber(r.avg_value)).filter((n): n is number => n !== null);
}
function mean(xs: number[]): number {
  if (xs.length === 0) return NaN;
  return xs.reduce((a, b) => a + b, 0) / xs.length;
}

/* ============================ small UI prims =========================== */

function Page({
  slot,
  theme,
  header,
  error,
  children,
}: {
  slot: string;
  theme: string;
  header: React.ReactNode;
  error: string | null;
  children: React.ReactNode;
}): React.ReactElement {
  return (
    <div
      data-ext-id={EXTENSION_ID}
      data-ext-slot={slot}
      data-ext-theme={theme}
      className="flex flex-col gap-4 p-4"
    >
      {header}
      {error ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 text-destructive px-3 py-2 text-sm">
          {error}
        </div>
      ) : null}
      {children}
    </div>
  );
}

function Header({
  subtitle,
  version,
  onRefresh,
  loading,
}: {
  subtitle: string;
  version?: string;
  onRefresh: () => void;
  loading: boolean;
}): React.ReactElement {
  return (
    <div className="flex items-center justify-between gap-4">
      <div>
        <h3 className="text-lg font-semibold tracking-tight">
          Rubix-OS
          {version ? <span className="text-muted-foreground font-normal ml-2 text-sm">v{version}</span> : null}
        </h3>
        <p className="text-sm text-muted-foreground">{subtitle}</p>
      </div>
      <button
        type="button"
        onClick={onRefresh}
        disabled={loading}
        className="text-sm px-3 py-1 rounded border border-border/60 hover:bg-accent disabled:opacity-50"
      >
        {loading ? "loading…" : "refresh"}
      </button>
    </div>
  );
}

function Card({
  title,
  description,
  children,
}: {
  title: React.ReactNode;
  description?: React.ReactNode;
  children: React.ReactNode;
}): React.ReactElement {
  return (
    <section className="rounded-lg border border-border/60 bg-card text-card-foreground">
      <header className="px-3 py-2 border-b border-border/60">
        <div className="text-sm font-medium">{title}</div>
        {description ? <div className="text-xs text-muted-foreground">{description}</div> : null}
      </header>
      <div className="p-3">{children}</div>
    </section>
  );
}

function Kpi({ label, value }: { label: string; value: string }): React.ReactElement {
  return (
    <div className="rounded-lg border border-border/60 bg-card text-card-foreground p-3">
      <div className="text-xs uppercase tracking-wide text-muted-foreground">{label}</div>
      <div className="text-xl font-semibold tabular-nums">{value}</div>
    </div>
  );
}

function KpiSm({ label, value }: { label: string; value: string }): React.ReactElement {
  return (
    <div>
      <div className="uppercase tracking-wide text-[0.65rem] opacity-75">{label}</div>
      <div className="text-sm font-medium tabular-nums text-foreground">{value}</div>
    </div>
  );
}

function Table({
  headers,
  children,
}: {
  headers: ReadonlyArray<string>;
  children: React.ReactNode;
}): React.ReactElement {
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="text-left text-xs text-muted-foreground">
            {headers.map((h, i) => (
              <th
                key={h}
                className={
                  "py-1.5 px-2 font-medium " + (i >= 1 && i === headers.length - 1 ? "text-right" : "")
                }
              >
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>{children}</tbody>
      </table>
    </div>
  );
}

function Td({
  children,
  align,
}: {
  children: React.ReactNode;
  align?: "left" | "right";
}): React.ReactElement {
  return <td className={"py-1.5 px-2 " + (align === "right" ? "text-right tabular-nums" : "")}>{children}</td>;
}

function Empty({ children }: { children: React.ReactNode }): React.ReactElement {
  return <p className="text-sm text-muted-foreground italic">{children}</p>;
}

function Mono({ children }: { children: React.ReactNode }): React.ReactElement {
  return <code className="text-xs font-mono">{children}</code>;
}

function fmtInt(v: number | string | null | undefined): string {
  if (v === null || v === undefined || v === "") return "—";
  const n = typeof v === "number" ? v : Number(v);
  if (!Number.isFinite(n)) return "—";
  return n.toLocaleString();
}
function fmtNum(v: number | null | undefined): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return "—";
  return v.toFixed(2);
}
function fmtTs(v: string | null | undefined): string {
  if (!v) return "—";
  const d = new Date(v);
  if (Number.isNaN(d.getTime())) return v;
  return d.toLocaleString();
}

/* ----------------------- reusable simple page ----------------------- */

function SimplePage<R>({
  title,
  template,
  rows,
  children,
}: {
  title: string;
  template: string;
  rows: AsyncState<R>;
  children: (rows: ReadonlyArray<R>) => React.ReactNode;
}): React.ReactElement {
  const slot = useSlotContext();
  const theme = useHostTheme();
  return (
    <Page
      slot={slot.slotId}
      theme={theme.mode}
      header={
        <div>
          <h3 className="text-lg font-semibold tracking-tight">{title}</h3>
          <p className="text-sm text-muted-foreground">{template}</p>
        </div>
      }
      error={rows.error}
    >
      <Card title={title} description={rows.loading ? "loading…" : `${rows.data.length} rows`}>
        {rows.data.length === 0 && !rows.loading ? (
          <Empty>No rows. Ingest the dump first.</Empty>
        ) : (
          children(rows.data)
        )}
      </Card>
    </Page>
  );
}

interface AsyncState<R> {
  data: ReadonlyArray<R>;
  loading: boolean;
  error: string | null;
}

function useTemplate<R>(template: string, params: Record<string, unknown>): AsyncState<R> {
  const [state, setState] = React.useState<AsyncState<R>>({ data: [], loading: false, error: null });
  // Stabilise params dependency by serialising
  const key = JSON.stringify(params);
  React.useEffect(() => {
    let cancelled = false;
    setState((s) => ({ ...s, loading: true, error: null }));
    fetchTemplate<R>(template, params)
      .then((rs) => !cancelled && setState({ data: rs, loading: false, error: null }))
      .catch((e: unknown) =>
        !cancelled &&
        setState({ data: [], loading: false, error: e instanceof Error ? e.message : String(e) }),
      );
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [template, key]);
  return state;
}

function ContribGrid({ detail }: { detail: ExtensionDetail | null }): React.ReactElement {
  const c = detail?.manifest?.contributes ?? {};
  const rows: Array<[string, ReadonlyArray<string>]> = [
    ["tools",              (c.tools ?? []).map((t) => t.id)],
    ["warehouse tables",   (c.warehouse_tables ?? []).map((t) => t.name)],
    ["warehouse templates",(c.warehouse_templates ?? []).map((t) => t.name)],
    ["ui slots",           (c.ui?.exposes ?? []).map((e) => e.slot)],
  ];
  return (
    <div className="flex flex-col gap-1.5">
      {rows.map(([label, items]) => (
        <div key={label} className="flex items-baseline gap-3 text-sm">
          <span className="text-muted-foreground shrink-0 w-44">{label}</span>
          <span className="flex flex-wrap gap-1">
            {items.length === 0 ? (
              <span className="text-muted-foreground/50">—</span>
            ) : (
              items.map((id, i) => (
                <code key={id + i} className="rounded bg-muted px-1.5 py-0.5 text-xs font-mono">
                  {id}
                </code>
              ))
            )}
          </span>
        </div>
      ))}
    </div>
  );
}
