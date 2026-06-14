// `panel.tsx` — the `main`-slot page for `com.nexus.hello`, built from REAL
// shadcn/ui primitives (Card, Button, Badge, Separator) vendored under
// `components/ui/`. It's a small dashboard: a header with a live status badge, a
// row of KPI stat cards, and a primary card that runs the extension's own
// contributed query-kind (`com.nexus.hello.ping`) through the host client.
//
// Styling: the extension ships its own scoped Tailwind v4 bundle (app.css +
// vite.config.ts), so every shadcn token class resolves against the host theme.
// The whole page is wrapped in `<div data-ext-id="com.nexus.hello">` so the
// scoped CSS matches.

import * as React from "react";
import { Activity, CheckCircle2, Clock, RefreshCw, Zap } from "lucide-react";
import { fetchJson } from "@nube/starter-client-ts";
import { BlockShell, useHostClient } from "@nube/starter-ext-sdk-ts";

import "./app.css";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "./components/ui/card";
import { Button, buttonVariants } from "./components/ui/button";
import { Badge } from "./components/ui/badge";
import { Separator } from "./components/ui/separator";

const EXTENSION_ID = "com.nexus.hello";
const KIND = "com.nexus.hello.ping";

interface PingResponse {
  rows: Array<{ greeting?: string; server_time?: string }>;
}

export default function HelloPanel(): React.ReactElement {
  return (
    <BlockShell>
      {/* `data-ext-id` is what the scoped Tailwind bundle keys off. */}
      <div
        data-ext-id={EXTENSION_ID}
        className="mx-auto flex max-w-4xl flex-col gap-6 p-1"
      >
        <PageInner />
      </div>
    </BlockShell>
  );
}

function PageInner(): React.ReactElement {
  const client = useHostClient();
  const [result, setResult] = React.useState<PingResponse | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(false);
  const [lastRun, setLastRun] = React.useState<number | null>(null);

  const runPing = React.useCallback(() => {
    setLoading(true);
    setError(null);
    fetchJson<PingResponse>(client, `${client.apiPrefix}/query`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ sql: "", kind: KIND }),
    })
      .then((r) => {
        setResult(r);
        setLastRun(performance.now());
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, [client]);

  React.useEffect(() => {
    runPing();
  }, [runPing]);

  const row = result?.rows?.[0];
  const ok = !error && !!row;

  return (
    <>
      {/* Header */}
      <div className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1.5">
          <p className="text-sm text-muted-foreground">Nexus Hello extension</p>
          <h1 className="flex items-center gap-2 text-2xl font-semibold tracking-tight">
            <span aria-hidden>👋</span> Overview
          </h1>
        </div>
        {error ? (
          <Badge variant="destructive">Offline</Badge>
        ) : ok ? (
          <Badge variant="success">
            <CheckCircle2 /> Live
          </Badge>
        ) : (
          <Badge variant="secondary">…</Badge>
        )}
      </div>

      {/* KPI row */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <StatCard
          icon={<Activity />}
          label="Status"
          value={error ? "error" : ok ? "live" : "—"}
          hint="contributed query-kind"
        />
        <StatCard
          icon={<Zap />}
          label="Source"
          value="3rd"
          hint="dispatcher source"
        />
        <StatCard
          icon={<Clock />}
          label="Latency"
          value={lastRun ? "fresh" : "—"}
          hint="host round-trip"
        />
      </div>

      {/* Main card */}
      <Card>
        <CardHeader>
          <CardTitle>Live ping</CardTitle>
          <CardDescription>
            Result of <code className="font-mono">{KIND}</code> run through the host
            client — proves federation, cookie auth, and third-source kind dispatch.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <Separator />
          {error ? (
            <p className="text-sm text-destructive">kind query failed: {error}</p>
          ) : row ? (
            <dl className="grid grid-cols-[7rem_1fr] items-center gap-x-6 gap-y-3 text-sm">
              <dt className="text-muted-foreground">Greeting</dt>
              <dd className="font-mono">{row.greeting}</dd>
              <dt className="text-muted-foreground">Server time</dt>
              <dd className="font-mono tabular-nums">{row.server_time}</dd>
            </dl>
          ) : (
            <p className="text-sm text-muted-foreground">
              running <code className="font-mono">{KIND}</code>…
            </p>
          )}
        </CardContent>
        <CardFooter className="gap-2">
          <Button size="sm" onClick={runPing} disabled={loading}>
            <RefreshCw className={loading ? "animate-spin" : undefined} />
            {loading ? "Running…" : "Run ping"}
          </Button>
          {/* shadcn's `asChild` needs Radix Slot, which this slim Button omits,
              so style the anchor directly via `buttonVariants`. */}
          <a
            href="/extensions"
            className={buttonVariants({ variant: "outline", size: "sm" })}
          >
            Manage extension
          </a>
        </CardFooter>
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
