// `devices-tab.tsx` — devices table with search/filter, rename, decommission,
// drill-in to points, and print-label.
import * as React from "react";
import {
  ChevronDown,
  ChevronRight,
  Eye,
  MapPin,
  Pencil,
  QrCode,
  Radio,
  RadioTower,
  RefreshCw,
  Search,
  Trash2,
  TriangleAlert,
  Wifi,
} from "lucide-react";
import { decommission, deviceUpdate, listDevices, listSites } from "../bc-api";
import { useRefreshKey } from "../refresh";
import { deviceHref, gotoDevice } from "../nav";
import { statusTone } from "../status";
import type { DeviceRow, SiteRow } from "../bc-types";
import { DeviceDetail } from "./device-detail";
import { LabelDialog } from "./label-dialog";
import { PlaceOnPage } from "./place-on-page";

const STATUSES = ["", "pending", "provisioned", "active", "commissioned", "decommissioned"];

// A compact KPI tile for the footer strip.
function StatTile({
  icon: Icon,
  label,
  value,
  tone,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: number | string;
  tone: string;
}): React.ReactElement {
  return (
    <div className="ext-glass flex items-center gap-3 p-3">
      <div className={"flex size-10 shrink-0 items-center justify-center rounded-lg " + tone}>
        <Icon className="size-5" />
      </div>
      <div className="min-w-0">
        <div className="ext-eyebrow">{label}</div>
        <div className="ext-num text-2xl font-semibold leading-tight text-foreground">{value}</div>
      </div>
    </div>
  );
}

export function DevicesTab(): React.ReactElement {
  const [rows, setRows] = React.useState<ReadonlyArray<DeviceRow>>([]);
  const [sites, setSites] = React.useState<ReadonlyArray<SiteRow>>([]);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [q, setQ] = React.useState("");
  const [site, setSite] = React.useState("");
  const [status, setStatus] = React.useState("");
  const [open, setOpen] = React.useState<string | null>(null);
  const [editing, setEditing] = React.useState<string | null>(null);
  const [editName, setEditName] = React.useState("");
  const [label, setLabel] = React.useState<string | null>(null);
  const [placing, setPlacing] = React.useState<string | null>(null);
  const refresh = useRefreshKey();

  const load = React.useCallback(() => {
    setLoading(true);
    setError(null);
    listDevices({ site_id: site || undefined, status: status || undefined, limit: 500 })
      .then(setRows)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, [site, status]);

  // Silent background refresh for the live poll: same query as `load`
  // but without flipping the `loading` spinner (so the refresh icon
  // doesn't blink every tick) and swallowing transient errors (a single
  // dropped poll shouldn't paint an error banner over good data).
  const poll = React.useCallback(() => {
    listDevices({ site_id: site || undefined, status: status || undefined, limit: 500 })
      .then(setRows)
      .catch(() => undefined);
  }, [site, status]);

  React.useEffect(load, [load, refresh]);
  React.useEffect(() => {
    listSites().then(setSites).catch(() => setSites([]));
  }, [refresh]);

  // Live updates via polling (the platform has no per-extension data SSE
  // channel today; a true push feed would need a host-side route). Poll
  // every 5s, but never while the tab is hidden (no point refetching
  // off-screen) nor mid-interaction (`editing`/`placing` own the row —
  // a refetch underneath them would be jarring). The toolbar refresh
  // button stays for an on-demand pull. On regaining focus we refetch
  // once immediately so a backgrounded tab catches up at once.
  React.useEffect(() => {
    const busy = () => document.hidden || editing !== null || placing !== null;
    const id = window.setInterval(() => {
      if (!busy()) poll();
    }, 5000);
    const onVisible = () => {
      if (!busy()) poll();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      window.clearInterval(id);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [poll, editing, placing]);

  // Decommission is a soft-delete (status flips, history retained —
  // BARCODE.md §7), so the row is still returned by the list query. Hide
  // those tombstones from the default view so "decommission" reads as a
  // delete; they remain reachable by explicitly selecting the
  // "decommissioned" status filter (which sends it server-side, so this
  // client guard is a no-op then and never hides a deliberate view).
  const visible = rows.filter((r) => status !== "" || r.status.toLowerCase() !== "decommissioned");

  const filtered = visible.filter((r) => {
    if (!q.trim()) return true;
    const hay = `${r.name ?? ""} ${r.device_id} ${r.template} ${r.address ?? ""}`.toLowerCase();
    return hay.includes(q.trim().toLowerCase());
  });

  // Footer KPIs derived from the visible result set (search-independent,
  // so totals don't shift as the user types) — decommissioned tombstones
  // are excluded to match what the table shows.
  const kpis = React.useMemo(() => {
    let connected = 0;
    let syncing = 0;
    let alerts = 0;
    for (const r of visible) {
      const t = statusTone(r.status);
      if (t.dot === "bg-emerald-500") connected += 1;
      else if (t.dot === "bg-amber-500") syncing += 1;
      else if (t.dot === "bg-rose-500") alerts += 1;
    }
    return { connected, syncing, alerts };
  }, [visible]);

  const saveRename = (id: string) => {
    deviceUpdate({ device_id: id, name: editName.trim() })
      .then(() => {
        setEditing(null);
        load();
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  const remove = (id: string, hard: boolean) => {
    if (!window.confirm(hard ? "Hard-delete this device permanently?" : "Decommission this device?")) return;
    decommission([id], hard)
      .then(load)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <div className="relative min-w-[14rem] flex-1">
          <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Search name / id / address"
            aria-label="Search devices"
            className="w-full rounded-lg border border-border/60 bg-background py-2 pl-9 pr-3 text-sm text-foreground outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/30"
          />
        </div>
        <select value={site} onChange={(e) => setSite(e.target.value)} aria-label="Filter by site" className="cursor-pointer rounded-lg border border-border/60 bg-background px-3 py-2 text-sm text-foreground transition-colors hover:border-border focus:border-primary focus:outline-none">
          <option value="">All sites</option>
          {sites.map((s) => (
            <option key={s.site_id} value={s.site_id}>
              {s.name}
            </option>
          ))}
        </select>
        <select value={status} onChange={(e) => setStatus(e.target.value)} aria-label="Filter by status" className="cursor-pointer rounded-lg border border-border/60 bg-background px-3 py-2 text-sm text-foreground transition-colors hover:border-border focus:border-primary focus:outline-none">
          {STATUSES.map((s) => (
            <option key={s} value={s}>
              {s || "All statuses"}
            </option>
          ))}
        </select>
        <button
          type="button"
          onClick={load}
          aria-label="Refresh devices"
          title="Refresh"
          className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-border/60 bg-background px-3 py-2 text-sm text-muted-foreground transition-colors hover:border-border hover:text-foreground"
        >
          <RefreshCw className={"size-4 " + (loading ? "animate-spin" : "")} />
        </button>
      </div>

      {error ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      <div className="ext-glass overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border/60 text-left">
              <th className="w-8 px-3 py-2.5" />
              <th className="px-3 py-2.5 font-medium"><span className="ext-eyebrow">Device</span></th>
              <th className="px-3 py-2.5 font-medium"><span className="ext-eyebrow">Template</span></th>
              <th className="px-3 py-2.5 font-medium"><span className="ext-eyebrow">Network / Addr</span></th>
              <th className="px-3 py-2.5 font-medium"><span className="ext-eyebrow">Status</span></th>
              <th className="px-3 py-2.5 text-right font-medium"><span className="ext-eyebrow">Actions</span></th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              <tr><td colSpan={6} className="px-3 py-6 text-center text-sm italic text-muted-foreground">Loading devices…</td></tr>
            ) : filtered.length === 0 ? (
              <tr><td colSpan={6} className="px-3 py-6 text-center text-sm italic text-muted-foreground">No devices match.</td></tr>
            ) : (
              filtered.map((r) => {
                const tone = statusTone(r.status);
                const isOpen = open === r.device_id;
                return (
                <React.Fragment key={r.device_id}>
                  <tr className="border-t border-border/40 transition-colors hover:bg-muted/30">
                    <td className="px-3 py-2">
                      <button type="button" aria-label="Toggle points" onClick={() => setOpen(isOpen ? null : r.device_id)} className="cursor-pointer rounded p-0.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground">
                        {isOpen ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
                      </button>
                    </td>
                    <td className="px-3 py-2">
                      {editing === r.device_id ? (
                        <span className="flex items-center gap-1">
                          <input value={editName} onChange={(e) => setEditName(e.target.value)} aria-label="Device name" className="rounded-md border border-border/60 bg-background px-2 py-1 text-sm outline-none focus:border-primary" />
                          <button type="button" onClick={() => saveRename(r.device_id)} className="cursor-pointer rounded-md bg-primary px-2 py-1 text-xs font-medium text-primary-foreground">save</button>
                        </span>
                      ) : (
                        <div className="flex items-center gap-2.5">
                          <span className="flex size-8 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
                            <Radio className="size-4" />
                          </span>
                          <div className="min-w-0">
                            <a
                              href={deviceHref(r.device_id)}
                              onClick={(e) => { e.preventDefault(); gotoDevice(r.device_id); }}
                              className="block cursor-pointer truncate text-left font-medium text-foreground transition-colors hover:text-primary"
                            >
                              {r.name ?? r.device_id}
                            </a>
                            <div className="font-mono text-[11px] uppercase tracking-wide text-muted-foreground">{r.device_id}</div>
                          </div>
                        </div>
                      )}
                    </td>
                    <td className="px-3 py-2 font-mono text-xs text-muted-foreground">{r.template}</td>
                    <td className="px-3 py-2 text-muted-foreground">
                      <span className="font-mono text-xs">{r.network ?? "—"}</span>
                      <span className="px-1 text-muted-foreground/50">/</span>
                      <span className="font-mono text-xs">{r.address ?? "—"}</span>
                    </td>
                    <td className="px-3 py-2">
                      <span className={"inline-flex items-center gap-1.5 text-xs font-medium " + tone.text}>
                        <span className={"inline-block size-1.5 rounded-full " + tone.dot} />
                        {r.status}
                      </span>
                    </td>
                    <td className="px-3 py-2">
                      <div className="flex items-center justify-end gap-1">
                        {r.status.toLowerCase().includes("pend") || !r.page_id ? (
                          <button type="button" aria-label="Place on page" title="Place on page" onClick={() => setPlacing(placing === r.device_id ? null : r.device_id)} className="cursor-pointer rounded-md px-2 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-amber-500/10">
                            <span className="flex items-center gap-1"><MapPin className="size-4" /> Place on page</span>
                          </button>
                        ) : null}
                        <a href={deviceHref(r.device_id)} aria-label="See device" title="See" onClick={(e) => { e.preventDefault(); gotoDevice(r.device_id); }} className="cursor-pointer rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground">
                          <Eye className="size-4" />
                        </a>
                        <button type="button" aria-label="Rename device" title="Rename" onClick={() => { setEditing(r.device_id); setEditName(r.name ?? ""); }} className="cursor-pointer rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground">
                          <Pencil className="size-4" />
                        </button>
                        <button type="button" aria-label="Print label" title="Print label" onClick={() => setLabel(r.device_id)} className="cursor-pointer rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground">
                          <QrCode className="size-4" />
                        </button>
                        <button type="button" aria-label="Decommission" title="Decommission" onClick={() => remove(r.device_id, false)} className="cursor-pointer rounded-md p-1.5 text-muted-foreground transition-colors hover:bg-rose-500/10 hover:text-rose-400">
                          <Trash2 className="size-4" />
                        </button>
                      </div>
                    </td>
                  </tr>
                  {placing === r.device_id ? (
                    <tr className="border-t border-border/30 bg-muted/20">
                      <td colSpan={6} className="p-3"><PlaceOnPage device={r} onClose={() => setPlacing(null)} /></td>
                    </tr>
                  ) : null}
                  {isOpen ? (
                    <tr className="border-t border-border/30 bg-muted/20">
                      <td colSpan={6} className="p-3"><DeviceDetail deviceId={r.device_id} /></td>
                    </tr>
                  ) : null}
                </React.Fragment>
                );
              })
            )}
          </tbody>
        </table>
      </div>

      {/* Fleet KPI strip — derived from the loaded result set. */}
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <StatTile icon={Wifi} label="Connected" value={kpis.connected} tone="bg-emerald-500/10 text-emerald-400" />
        <StatTile icon={RefreshCw} label="Syncing" value={kpis.syncing} tone="bg-amber-500/10 text-amber-400" />
        <StatTile icon={TriangleAlert} label="Alerts" value={kpis.alerts} tone="bg-rose-500/10 text-rose-400" />
        <StatTile icon={RadioTower} label="Devices" value={visible.length} tone="bg-primary/10 text-primary" />
      </div>

      {label ? <LabelDialog deviceId={label} onClose={() => setLabel(null)} /> : null}
    </div>
  );
}
