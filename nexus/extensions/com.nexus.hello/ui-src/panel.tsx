// `panel.tsx` — the `main`-slot page for `com.nexus.hello`, built from REAL
// shadcn/ui primitives (`Card`, `Button`) vendored under `components/ui/`.
//
// It exercises the whole WS-14 loop: it runs the extension's *own contributed
// query-kind* (`com.nexus.hello.ping`, the dispatcher's third source) through
// the host's client and renders the result inside a shadcn Card. If this page
// renders styled and shows a greeting + server time, then federation load,
// singleton negotiation, slot mounting, cookie auth, kind dispatch, AND the
// extension's own scoped Tailwind/shadcn bundle are all working end to end.
//
// Styling: the extension ships its own Tailwind v4 bundle (see app.css +
// vite.config.ts), scanned against its own source and scoped to
// `[data-ext-id]`, so every shadcn token class resolves against the host theme.
// The whole page is wrapped in `<div data-ext-id="com.nexus.hello">` so the
// scoped CSS matches.

import * as React from "react";
import { RefreshCw } from "lucide-react";
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
import { Button, buttonVariants } from "./components/ui/button";

const EXTENSION_ID = "com.nexus.hello";
const KIND = "com.nexus.hello.ping";

/** The slice of nexus's `QueryResponse` this page reads. */
interface PingResponse {
  rows: Array<{ greeting?: string; server_time?: string }>;
}

export default function HelloPanel(): React.ReactElement {
  return (
    <BlockShell>
      {/* The `data-ext-id` wrapper is what the scoped Tailwind bundle keys off —
          without it none of the extension's utility classes apply. */}
      <div data-ext-id={EXTENSION_ID} className="mx-auto flex max-w-3xl flex-col gap-6 p-1">
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

  const runPing = React.useCallback(() => {
    setLoading(true);
    setError(null);
    // Kind-mode query: the server resolves `kind` against its registries
    // (file pack → extension-contributed → tenant overlay) — `sql` is ignored.
    fetchJson<PingResponse>(client, `${client.apiPrefix}/query`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ sql: "", kind: KIND }),
    })
      .then((r) => setResult(r))
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, [client]);

  React.useEffect(() => {
    runPing();
  }, [runPing]);

  const row = result?.rows?.[0];

  return (
    <>
      <div className="flex flex-col gap-1">
        <p className="text-sm text-muted-foreground">Nexus Hello extension</p>
        <h1 className="text-2xl font-semibold tracking-tight">👋 Overview</h1>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Live ping (contributed kind)</CardTitle>
          <CardDescription>
            Result of <span className="font-mono">{KIND}</span> run through the host
            client — proves federation, auth, and third-source kind dispatch.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {error ? (
            <p className="text-sm text-destructive">kind query failed: {error}</p>
          ) : row ? (
            <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm">
              <dt className="text-muted-foreground">Greeting</dt>
              <dd className="font-mono">{row.greeting}</dd>
              <dt className="text-muted-foreground">Server time</dt>
              <dd className="font-mono tabular-nums">{row.server_time}</dd>
            </dl>
          ) : (
            <p className="text-sm text-muted-foreground">
              running <span className="font-mono">{KIND}</span>…
            </p>
          )}

          <div className="mt-5 flex items-center gap-2">
            <Button size="sm" onClick={runPing} disabled={loading}>
              <RefreshCw className={loading ? "animate-spin" : undefined} />
              {loading ? "Running…" : "Run ping"}
            </Button>
            {/* A styled link: shadcn's `asChild` needs Radix Slot, which this
                slimmed-down Button omits, so apply the variants to the anchor
                directly via `buttonVariants` (the canonical no-Slot pattern). */}
            <a
              href="/extensions"
              className={buttonVariants({ variant: "outline", size: "sm" })}
            >
              Manage extension
            </a>
          </div>
        </CardContent>
      </Card>
    </>
  );
}
