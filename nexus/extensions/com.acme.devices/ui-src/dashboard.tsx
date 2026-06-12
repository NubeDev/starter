// `dashboard.tsx` — the Devices DASHBOARD page for `com.acme.devices`.
//
// Where `panel.tsx` *provisions* a device (the write/automation side), this is
// the read-only OVERVIEW of every device the viewer may see. It is a separate
// `main`-slot component (`DevicesDashboard`) with its own sidebar-nav entry, so
// the host mounts it at `/x/com.acme.devices/dashboard`.
//
// The data is the SAME persisted store as the panel's table: the extension owns
// `com_acme_devices__devices` in the nexus Postgres DB (WS-17 Wave A), and we
// read it back through the `com.acme.devices.devices_list` query-kind. The host
// binds `$caller_tenant_id` / `$caller_team_ids` from the verified session, so
// the rows — and therefore every summary tile below — are already scoped to the
// viewer's tenant and team. Nothing here is client-trusted.
//
// Styling: same scoped Tailwind v4 bundle as the panel; the whole page is
// wrapped in `<div data-ext-id="com.acme.devices">` so shadcn tokens resolve.

import * as React from "react";
import {
  Boxes,
  LayoutDashboard,
  MapPin,
  RefreshCw,
  Table2,
  Users,
} from "lucide-react";
import { fetchJson } from "@nube/starter-client-ts";
import { BlockShell, useHostClient } from "@nube/starter-ext-sdk-ts";

import "./app.css";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "./components/ui/card";
import { Button } from "./components/ui/button";
import { Badge } from "./components/ui/badge";

const EXTENSION_ID = "com.acme.devices";

/** One row of `com.acme.devices.devices_list` (the read API over the owned table). */
type DeviceRow = {
  device_id?: string;
  barcode?: string;
  location?: string;
  owner?: string;
  team?: string;
  created_at?: string;
};

export default function DevicesDashboard(): React.ReactElement {
  return (
    <BlockShell>
      <div
        data-ext-id={EXTENSION_ID}
        className="mx-auto flex max-w-5xl flex-col gap-5 p-1"
      >
        <DashboardInner />
      </div>
    </BlockShell>
  );
}

function DashboardInner(): React.ReactElement {
  const client = useHostClient();
  const [rows, setRows] = React.useState<DeviceRow[]>([]);
  const [loading, setLoading] = React.useState(false);
  const [err, setErr] = React.useState<string | null>(null);

  // Read the persisted devices from the extension-owned table via the
  // `devices_list` query-kind. No params — the scope IS the caller's identity
  // (the host binds `$caller_tenant_id` / `$caller_team_ids`), so this returns
  // exactly the rows the viewer's tenant + team may see.
  const load = React.useCallback(() => {
    setLoading(true);
    setErr(null);
    fetchJson<{ rows: DeviceRow[] }>(client, `${client.apiPrefix}/query`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ kind: "com.acme.devices.devices_list", params: {} }),
    })
      .then((r) => setRows(Array.isArray(r.rows) ? r.rows : []))
      .catch((e: unknown) => setErr(friendlyError(e)))
      .finally(() => setLoading(false));
  }, [client]);

  React.useEffect(() => {
    load();
  }, [load]);

  const stats = React.useMemo(() => summarize(rows), [rows]);

  return (
    <>
      {/* Header */}
      <div className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1.5">
          <p className="text-sm text-muted-foreground">
            Acme Devices · Fleet overview
          </p>
          <h1 className="flex items-center gap-2 text-2xl font-semibold tracking-tight">
            <LayoutDashboard className="size-6" /> Devices dashboard
          </h1>
        </div>
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={load}
          disabled={loading}
          title="Reload from the server"
        >
          <RefreshCw className={loading ? "animate-spin" : ""} /> Refresh
        </Button>
      </div>

      {/* Summary tiles — every number is already tenant/team-scoped by the host. */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <StatCard
          icon={<Boxes className="size-4" />}
          label="Devices"
          value={String(stats.total)}
          hint="Persisted in the nexus DB"
        />
        <StatCard
          icon={<Users className="size-4" />}
          label="Teams"
          value={String(stats.teams.length)}
          hint={
            stats.teams.length
              ? stats.teams.join(", ")
              : "tenant-wide only"
          }
        />
        <StatCard
          icon={<MapPin className="size-4" />}
          label="Locations"
          value={String(stats.locations.length)}
          hint={
            stats.locations.length
              ? `${stats.locations.length} distinct`
              : "none recorded"
          }
        />
      </div>

      {/* Full table */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Table2 className="size-4" /> All devices
          </CardTitle>
          <CardDescription>
            Read from the extension's own nexus table
            (<code className="font-mono">com_acme_devices__devices</code>) via the{" "}
            <code className="font-mono">devices_list</code> kind — scoped to your
            tenant &amp; team by the host's un-spoofable identity tokens.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {err ? (
            <div className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {err}
            </div>
          ) : rows.length === 0 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              {loading
                ? "Loading…"
                : "No devices yet — provision one on the “Provision device” page."}
            </p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full border-collapse text-sm">
                <thead>
                  <tr className="border-b text-left text-xs text-muted-foreground">
                    <th className="py-2 pr-4 font-medium">device_id</th>
                    <th className="py-2 pr-4 font-medium">barcode</th>
                    <th className="py-2 pr-4 font-medium">location</th>
                    <th className="py-2 pr-4 font-medium">owner</th>
                    <th className="py-2 pr-4 font-medium">team</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((d, i) => (
                    <tr key={d.device_id ?? i} className="border-b last:border-0">
                      <td className="py-2 pr-4 font-mono text-xs">
                        {d.device_id ?? "—"}
                      </td>
                      <td className="py-2 pr-4 font-mono text-xs">
                        {d.barcode ?? "—"}
                      </td>
                      <td className="py-2 pr-4">{d.location || "—"}</td>
                      <td className="py-2 pr-4 font-mono text-xs">
                        {d.owner ? `${d.owner.slice(0, 8)}…` : "—"}
                      </td>
                      <td className="py-2 pr-4">
                        {d.team ? (
                          <Badge variant="secondary">{d.team}</Badge>
                        ) : (
                          <span className="text-muted-foreground">tenant-wide</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>
    </>
  );
}

function StatCard({
  icon,
  label,
  value,
  hint,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  hint: string;
}): React.ReactElement {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardDescription className="flex items-center gap-1.5">
          {icon} {label}
        </CardDescription>
        <CardTitle className="text-3xl tabular-nums">{value}</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="truncate text-xs text-muted-foreground" title={hint}>
          {hint}
        </p>
      </CardContent>
    </Card>
  );
}

function summarize(rows: DeviceRow[]): {
  total: number;
  teams: string[];
  locations: string[];
} {
  const teams = new Set<string>();
  const locations = new Set<string>();
  for (const r of rows) {
    if (r.team) teams.add(r.team);
    if (r.location) locations.add(r.location);
  }
  return {
    total: rows.length,
    teams: [...teams].sort(),
    locations: [...locations].sort(),
  };
}

// Surface the most common failure — the team/tenant scope returning nothing, or
// a CSRF/session error — in language a demo viewer can act on.
function friendlyError(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e);
  if (/403|forbidden|csrf/i.test(msg)) {
    return `${msg} — if this is a CSRF error, reload to refresh your session token.`;
  }
  return msg;
}
