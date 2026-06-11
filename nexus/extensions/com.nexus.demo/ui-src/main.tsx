// `main.tsx` — the page body for `com.nexus.demo` (`slot: main`).
//
// Mounted by the host's `/x/:extId/*` route into the main content area, with
// the path tail forwarded as the slot route. `useExtensionRoute()` returns that
// tail so this component dispatches its own sub-pages (Overview / Readings /
// About) — the rubix `MainRouter` pattern.
//
// Styling: built from REAL shadcn/ui primitives vendored under `components/ui/`
// (Card, Button, Badge, Separator). The extension ships its OWN scoped Tailwind
// v4 bundle (app.css + vite.config.ts), scanned against its own source and
// scoped to `[data-ext-id]`, so every shadcn token class resolves against the
// host theme. Every page is wrapped in `<div data-ext-id="com.nexus.demo">` so
// the scoped CSS matches. Component name `Main` MUST match the manifest expose.

import * as React from "react";
import {
  Activity,
  Bell,
  Building2,
  Info,
  RefreshCw,
} from "lucide-react";
import { fetchJson } from "@nube/starter-client-ts";
import { BlockShell, useExtensionRoute, useHostClient } from "@nube/starter-ext-sdk-ts";

import "./app.css";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "./components/ui/card";
import { Button } from "./components/ui/button";
import { Badge } from "./components/ui/badge";
import { Separator } from "./components/ui/separator";

const EXTENSION_ID = "com.nexus.demo";

interface PingResponse {
  rows: Array<{ greeting?: string; server_time?: string }>;
}

export default function Main(): React.ReactElement {
  return (
    <BlockShell>
      {/* `data-ext-id` is what the scoped Tailwind bundle keys off. */}
      <div data-ext-id={EXTENSION_ID} className="mx-auto flex max-w-5xl flex-col gap-6 p-1">
        <MainRouter />
      </div>
    </BlockShell>
  );
}

function MainRouter(): React.ReactElement {
  const route = useExtensionRoute();
  const page =
    route === "readings" || route?.startsWith("readings")
      ? "readings"
      : route === "about" || route?.startsWith("about")
        ? "about"
        : "overview";

  return (
    <div className="flex flex-col gap-6">
      <Header page={page} />
      {page === "overview" ? <OverviewPage /> : null}
      {page === "readings" ? <ReadingsPage /> : null}
      {page === "about" ? <AboutPage /> : null}
    </div>
  );
}

function Header({ page }: { page: string }) {
  const titles: Record<string, string> = {
    overview: "Overview",
    readings: "Readings",
    about: "About",
  };
  return (
    <div className="flex flex-col gap-1">
      <p className="text-sm text-muted-foreground">Nexus Demo extension</p>
      <h1 className="text-2xl font-semibold tracking-tight">{titles[page]}</h1>
      <nav className="mt-2 flex gap-1 border-b border-border">
        <Tab to="/x/com.nexus.demo" active={page === "overview"}>
          Overview
        </Tab>
        <Tab to="/x/com.nexus.demo/readings" active={page === "readings"}>
          Readings
        </Tab>
        <Tab to="/x/com.nexus.demo/about" active={page === "about"}>
          About
        </Tab>
      </nav>
    </div>
  );
}

function Tab({
  to,
  active,
  children,
}: {
  to: string;
  active: boolean;
  children: React.ReactNode;
}) {
  return (
    <a
      href={to}
      className={
        "-mb-px border-b-2 px-3 py-2 text-sm transition-colors " +
        (active
          ? "border-primary font-medium text-foreground"
          : "border-transparent text-muted-foreground hover:text-foreground")
      }
    >
      {children}
    </a>
  );
}

// --- Overview: KPI stat cards + a live ping from the contributed kind ----------

function OverviewPage(): React.ReactElement {
  const client = useHostClient();
  const [ping, setPing] = React.useState<PingResponse | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(false);

  const runPing = React.useCallback(() => {
    setLoading(true);
    setError(null);
    fetchJson<PingResponse>(client, `${client.apiPrefix}/query`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ sql: "", kind: `${EXTENSION_ID}.ping` }),
    })
      .then((r) => setPing(r))
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, [client]);

  React.useEffect(() => {
    runPing();
  }, [runPing]);

  const row = ping?.rows?.[0];
  const ok = !error && !!row;

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <StatCard
          icon={<Activity />}
          label="Status"
          value={error ? "error" : ok ? "live" : "—"}
          hint="contributed query-kind"
        />
        <StatCard
          icon={<Building2 />}
          label="Sites"
          value="12"
          hint="across 3 regions"
        />
        <StatCard
          icon={<Bell />}
          label="Open alerts"
          value="2"
          hint="1 critical"
        />
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between gap-3">
            <CardTitle>Live ping</CardTitle>
            {error ? (
              <Badge variant="destructive">Offline</Badge>
            ) : ok ? (
              <Badge variant="success">Live</Badge>
            ) : (
              <Badge variant="secondary">…</Badge>
            )}
          </div>
          <CardDescription>
            Result of <code className="font-mono">{EXTENSION_ID}.ping</code> run
            through the host client — proves federation, auth, and third-source
            kind dispatch.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <Separator />
          {error ? (
            <p className="text-sm text-destructive">{error}</p>
          ) : row ? (
            <dl className="grid grid-cols-[7rem_1fr] items-center gap-x-6 gap-y-3 text-sm">
              <dt className="text-muted-foreground">Greeting</dt>
              <dd className="font-mono">{row.greeting}</dd>
              <dt className="text-muted-foreground">Server time</dt>
              <dd className="font-mono tabular-nums">{row.server_time}</dd>
            </dl>
          ) : (
            <p className="text-sm text-muted-foreground">Loading…</p>
          )}
        </CardContent>
        <CardFooter>
          <Button size="sm" onClick={runPing} disabled={loading}>
            <RefreshCw className={loading ? "animate-spin" : undefined} />
            {loading ? "Running…" : "Refresh"}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}

// --- Readings: a shadcn-styled table inside a Card -----------------------------

const READINGS: Array<{ site: string; metric: string; value: string; trend: string }> = [
  { site: "HQ — Roof", metric: "Power", value: "42.1 kW", trend: "▲ 3%" },
  { site: "HQ — Floor 2", metric: "Temp", value: "21.4 °C", trend: "▼ 1%" },
  { site: "Depot A", metric: "Water", value: "118 L/min", trend: "▲ 8%" },
  { site: "Depot B", metric: "Power", value: "9.7 kW", trend: "—" },
];

function ReadingsPage(): React.ReactElement {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Latest readings</CardTitle>
        <CardDescription>Most recent value per site & metric.</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="overflow-hidden rounded-lg border border-border">
          <table className="w-full text-sm">
            <thead className="bg-muted/50 text-left text-muted-foreground">
              <tr>
                <th className="px-3 py-2 font-medium">Site</th>
                <th className="px-3 py-2 font-medium">Metric</th>
                <th className="px-3 py-2 font-medium">Value</th>
                <th className="px-3 py-2 font-medium">Trend</th>
              </tr>
            </thead>
            <tbody>
              {READINGS.map((r) => (
                <tr key={`${r.site}-${r.metric}`} className="border-t border-border">
                  <td className="px-3 py-2">{r.site}</td>
                  <td className="px-3 py-2 text-muted-foreground">{r.metric}</td>
                  <td className="px-3 py-2 font-mono">{r.value}</td>
                  <td className="px-3 py-2">{r.trend}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  );
}

function AboutPage(): React.ReactElement {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <Info className="size-4 text-muted-foreground" />
          <CardTitle>About this extension</CardTitle>
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-3 text-sm leading-relaxed text-muted-foreground">
        <p>
          <span className="font-medium text-foreground">com.nexus.demo</span> is a
          worked WS-14 example: it contributes a sidebar nav group and a full page
          rendered into the host's content area, plus two query-kinds and an
          insight on the backend.
        </p>
        <p>
          The page you're reading is the extension's own federated UI (the{" "}
          <code className="rounded bg-muted px-1 py-0.5">main</code> slot), mounted
          by the host route{" "}
          <code className="rounded bg-muted px-1 py-0.5">/x/:extId/*</code>. The
          tabs above change the slot route; the extension dispatches its own
          sub-pages — the host registers no routes for it.
        </p>
      </CardContent>
    </Card>
  );
}

// --- KPI stat card (shadcn Card + tinted icon tile) ---------------------------

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
    <Card className="gap-0 py-0">
      <CardContent className="flex items-start justify-between gap-3 p-4">
        <div className="flex flex-col gap-1">
          <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {label}
          </p>
          <p className="text-2xl font-semibold leading-none tracking-tight">
            {value}
          </p>
          <p className="text-xs text-muted-foreground">{hint}</p>
        </div>
        <span className="grid size-9 shrink-0 place-items-center rounded-lg bg-primary/15 text-primary">
          {icon}
        </span>
      </CardContent>
    </Card>
  );
}
