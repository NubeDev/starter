// `main.tsx` — the page body for `com.nexus.demo` (`slot: main`).
//
// Mounted by the host's `/x/:extId/*` route into the main content area, with
// the path tail forwarded as the slot route. `useExtensionRoute()` returns that
// tail so this component dispatches its own sub-pages (Overview / Readings /
// About) — the rubix `MainRouter` pattern.
//
// Styling: the host shares only React via the importmap, NOT its shadcn/ui
// component library. So this page is built from plain elements + the host's
// Tailwind design tokens (`bg-card`, `text-muted-foreground`, `border`, …),
// which resolve against the host theme — the shadcn *look* without importing
// the host's components. Component name `Main` MUST match the manifest expose.

import * as React from "react";
import { fetchJson } from "@nube/starter-client-ts";
import { BlockShell, useExtensionRoute, useHostClient } from "@nube/starter-ext-sdk-ts";

const EXTENSION_ID = "com.nexus.demo";

interface PingResponse {
  rows: Array<{ greeting?: string; server_time?: string }>;
}

export default function Main(): React.ReactElement {
  return (
    <BlockShell>
      <MainRouter />
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
    <div className="mx-auto flex max-w-5xl flex-col gap-6">
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
      <nav className="mt-2 flex gap-1 border-b">
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

// --- Overview: KPI cards + a live ping from the contributed kind --------------

function OverviewPage(): React.ReactElement {
  const client = useHostClient();
  const [ping, setPing] = React.useState<PingResponse | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    fetchJson<PingResponse>(client, `${client.apiPrefix}/query`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ sql: "", kind: `${EXTENSION_ID}.ping` }),
    })
      .then((r) => !cancelled && setPing(r))
      .catch((e: unknown) =>
        !cancelled
          ? setError(e instanceof Error ? e.message : String(e))
          : undefined,
      );
    return () => {
      cancelled = true;
    };
  }, [client]);

  const row = ping?.rows?.[0];

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <Card title="Status" value={error ? "error" : row ? "live" : "…"} />
        <Card title="Sites" value="12" hint="across 3 regions" />
        <Card title="Open alerts" value="2" hint="1 critical" />
      </div>

      <Panel title="Live ping (contributed kind)">
        {error ? (
          <p className="text-sm text-destructive">{error}</p>
        ) : row ? (
          <dl className="grid grid-cols-2 gap-2 text-sm">
            <dt className="text-muted-foreground">Greeting</dt>
            <dd className="font-mono">{row.greeting}</dd>
            <dt className="text-muted-foreground">Server time</dt>
            <dd className="font-mono">{row.server_time}</dd>
          </dl>
        ) : (
          <p className="text-sm text-muted-foreground">Loading…</p>
        )}
      </Panel>
    </div>
  );
}

// --- Readings: a simple shadcn-styled table -----------------------------------

const READINGS: Array<{ site: string; metric: string; value: string; trend: string }> = [
  { site: "HQ — Roof", metric: "Power", value: "42.1 kW", trend: "▲ 3%" },
  { site: "HQ — Floor 2", metric: "Temp", value: "21.4 °C", trend: "▼ 1%" },
  { site: "Depot A", metric: "Water", value: "118 L/min", trend: "▲ 8%" },
  { site: "Depot B", metric: "Power", value: "9.7 kW", trend: "—" },
];

function ReadingsPage(): React.ReactElement {
  return (
    <Panel title="Latest readings">
      <div className="overflow-hidden rounded-lg border">
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
              <tr key={`${r.site}-${r.metric}`} className="border-t">
                <td className="px-3 py-2">{r.site}</td>
                <td className="px-3 py-2 text-muted-foreground">{r.metric}</td>
                <td className="px-3 py-2 font-mono">{r.value}</td>
                <td className="px-3 py-2">{r.trend}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Panel>
  );
}

function AboutPage(): React.ReactElement {
  return (
    <Panel title="About this extension">
      <div className="flex flex-col gap-3 text-sm leading-relaxed text-muted-foreground">
        <p>
          <span className="font-medium text-foreground">com.nexus.demo</span> is
          a worked WS-14 example: it contributes a sidebar nav group and a full
          page rendered into the host's content area, plus two query-kinds and
          an insight on the backend.
        </p>
        <p>
          The page you're reading is the extension's own federated UI (the{" "}
          <code className="rounded bg-muted px-1 py-0.5">main</code> slot),
          mounted by the host route{" "}
          <code className="rounded bg-muted px-1 py-0.5">/x/:extId/*</code>. The
          tabs above change the slot route; the extension dispatches its own
          sub-pages — the host registers no routes for it.
        </p>
      </div>
    </Panel>
  );
}

// --- Small shadcn-styled primitives (host design tokens) ----------------------

function Card({
  title,
  value,
  hint,
}: {
  title: string;
  value: string;
  hint?: string;
}) {
  return (
    <div className="rounded-xl border bg-card p-4 text-card-foreground shadow-sm">
      <p className="text-sm text-muted-foreground">{title}</p>
      <p className="mt-1 text-2xl font-semibold tracking-tight">{value}</p>
      {hint ? <p className="mt-1 text-xs text-muted-foreground">{hint}</p> : null}
    </div>
  );
}

function Panel({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-xl border bg-card p-5 text-card-foreground shadow-sm">
      <h2 className="mb-3 text-sm font-medium">{title}</h2>
      {children}
    </section>
  );
}
