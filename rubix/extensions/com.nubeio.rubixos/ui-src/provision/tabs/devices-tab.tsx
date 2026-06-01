// `devices-tab.tsx` — devices table with search/filter, rename, decommission,
// drill-in to points, and print-label.
import * as React from "react";
import { ChevronDown, ChevronRight, QrCode, Trash2 } from "lucide-react";
import { decommission, deviceUpdate, listDevices, listSites } from "../bc-api";
import type { DeviceRow, SiteRow } from "../bc-types";
import { DeviceDetail } from "./device-detail";
import { LabelDialog } from "./label-dialog";

const STATUSES = ["", "active", "commissioned", "decommissioned"];

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

  const load = React.useCallback(() => {
    setLoading(true);
    setError(null);
    listDevices({ site_id: site || undefined, status: status || undefined, limit: 500 })
      .then(setRows)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, [site, status]);

  React.useEffect(load, [load]);
  React.useEffect(() => {
    listSites().then(setSites).catch(() => setSites([]));
  }, []);

  const filtered = rows.filter((r) => {
    if (!q.trim()) return true;
    const hay = `${r.name ?? ""} ${r.device_id} ${r.template} ${r.address ?? ""}`.toLowerCase();
    return hay.includes(q.trim().toLowerCase());
  });

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
      <div className="flex flex-wrap items-end gap-2">
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="Search name / id / address"
          aria-label="Search devices"
          className="flex-1 rounded-md border border-border/60 bg-background px-3 py-1.5 text-sm text-foreground outline-none focus:border-primary"
        />
        <select value={site} onChange={(e) => setSite(e.target.value)} aria-label="Filter by site" className="rounded-md border border-border/60 bg-background px-2 py-1.5 text-sm text-foreground">
          <option value="">All sites</option>
          {sites.map((s) => (
            <option key={s.site_id} value={s.site_id}>
              {s.name}
            </option>
          ))}
        </select>
        <select value={status} onChange={(e) => setStatus(e.target.value)} aria-label="Filter by status" className="rounded-md border border-border/60 bg-background px-2 py-1.5 text-sm text-foreground">
          {STATUSES.map((s) => (
            <option key={s} value={s}>
              {s || "All statuses"}
            </option>
          ))}
        </select>
      </div>

      {error ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      <div className="overflow-x-auto rounded-lg border border-border/60 bg-card">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-xs text-muted-foreground">
              <th className="px-2 py-2 font-medium" />
              <th className="px-2 py-2 font-medium">Device</th>
              <th className="px-2 py-2 font-medium">Template</th>
              <th className="px-2 py-2 font-medium">Network/Addr</th>
              <th className="px-2 py-2 font-medium">Status</th>
              <th className="px-2 py-2 text-right font-medium">Actions</th>
            </tr>
          </thead>
          <tbody>
            {loading ? (
              <tr><td colSpan={6} className="px-2 py-3 text-sm italic text-muted-foreground">loading…</td></tr>
            ) : filtered.length === 0 ? (
              <tr><td colSpan={6} className="px-2 py-3 text-sm italic text-muted-foreground">No devices match.</td></tr>
            ) : (
              filtered.map((r) => (
                <React.Fragment key={r.device_id}>
                  <tr className="border-t border-border/40">
                    <td className="px-2 py-1.5">
                      <button type="button" aria-label="Toggle points" onClick={() => setOpen(open === r.device_id ? null : r.device_id)} className="rounded p-0.5 hover:bg-accent">
                        {open === r.device_id ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
                      </button>
                    </td>
                    <td className="px-2 py-1.5">
                      {editing === r.device_id ? (
                        <span className="flex gap-1">
                          <input value={editName} onChange={(e) => setEditName(e.target.value)} aria-label="Device name" className="rounded border border-border/60 bg-background px-1.5 py-0.5 text-sm" />
                          <button type="button" onClick={() => saveRename(r.device_id)} className="text-xs text-primary hover:underline">save</button>
                        </span>
                      ) : (
                        <button type="button" onClick={() => { setEditing(r.device_id); setEditName(r.name ?? ""); }} className="text-left text-foreground hover:underline">
                          {r.name ?? r.device_id}
                        </button>
                      )}
                    </td>
                    <td className="px-2 py-1.5 text-muted-foreground">{r.template}</td>
                    <td className="px-2 py-1.5 text-muted-foreground">{r.network ?? "—"} / {r.address ?? "—"}</td>
                    <td className="px-2 py-1.5 text-muted-foreground">{r.status}</td>
                    <td className="px-2 py-1.5">
                      <div className="flex items-center justify-end gap-2">
                        <button type="button" aria-label="Print label" title="Print label" onClick={() => setLabel(r.device_id)} className="rounded p-1 hover:bg-accent">
                          <QrCode className="size-4" />
                        </button>
                        <button type="button" aria-label="Decommission" title="Decommission" onClick={() => remove(r.device_id, false)} className="rounded p-1 text-destructive hover:bg-accent">
                          <Trash2 className="size-4" />
                        </button>
                      </div>
                    </td>
                  </tr>
                  {open === r.device_id ? (
                    <tr className="border-t border-border/30 bg-muted/30">
                      <td colSpan={6} className="p-2"><DeviceDetail deviceId={r.device_id} /></td>
                    </tr>
                  ) : null}
                </React.Fragment>
              ))
            )}
          </tbody>
        </table>
      </div>

      {label ? <LabelDialog deviceId={label} onClose={() => setLabel(null)} /> : null}
    </div>
  );
}
