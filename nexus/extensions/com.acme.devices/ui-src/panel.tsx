// `panel.tsx` — the `main`-slot "Provision device" page for
// `com.acme.devices`, built from REAL shadcn/ui primitives (Card, Button,
// Badge, Separator) vendored under `components/ui/`.
//
// It drives the Setup / Automation Builder barcode story end to end against the
// host's `/setup` REST surface:
//   1. type/scan a barcode + location → POST /setup/templates/{id}/run → 202
//      { run_id } (instant launch — DOCS §7),
//   2. open the SSE stream GET /setup/runs/{run_id}/events and render per-step
//      progress as it ticks,
//   3. if a step fails, show a Retry button → POST /setup/runs/{run_id}/resume
//      which continues from the failed step (DOCS §8b).
//
// Styling: the extension ships its own scoped Tailwind v4 bundle (app.css +
// vite.config.ts), so every shadcn token class resolves against the host theme.
// The whole page is wrapped in `<div data-ext-id="com.acme.devices">`.

import * as React from "react";
import {
  CheckCircle2,
  Cpu,
  Loader2,
  RotateCcw,
  ScanLine,
  XCircle,
} from "lucide-react";
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
import { Button } from "./components/ui/button";
import { Badge } from "./components/ui/badge";
import { Separator } from "./components/ui/separator";

const EXTENSION_ID = "com.acme.devices";
const TEMPLATE_ID = "com.acme.add-device";

type RunStatus =
  | "idle"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

interface StepEvent {
  event?: string;
  current_step?: string;
  total?: number;
  status?: string;
  error?: string;
  resumable?: boolean;
}

export default function DevicesPanel(): React.ReactElement {
  return (
    <BlockShell>
      <div
        data-ext-id={EXTENSION_ID}
        className="mx-auto flex max-w-3xl flex-col gap-6 p-1"
      >
        <PageInner />
      </div>
    </BlockShell>
  );
}

function PageInner(): React.ReactElement {
  const client = useHostClient();
  const [barcode, setBarcode] = React.useState("");
  const [location, setLocation] = React.useState("Roof AHU-3");
  const [runId, setRunId] = React.useState<string | null>(null);
  const [status, setStatus] = React.useState<RunStatus>("idle");
  const [step, setStep] = React.useState<string | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [resumable, setResumable] = React.useState(false);
  const esRef = React.useRef<EventSource | null>(null);

  const closeStream = React.useCallback(() => {
    esRef.current?.close();
    esRef.current = null;
  }, []);

  // Tail the SSE progress stream for a run. The host's EventSource path uses a
  // query-string token (cookie auth isn't sent on EventSource) — the SDK client
  // exposes the prefix; we append the standard token param the host expects.
  const openStream = React.useCallback(
    (id: string) => {
      closeStream();
      const url = `${client.apiPrefix}/setup/runs/${id}/events`;
      const es = new EventSource(url, { withCredentials: true });
      esRef.current = es;
      const onMsg = (raw: MessageEvent) => {
        let ev: StepEvent;
        try {
          ev = JSON.parse(raw.data) as StepEvent;
        } catch {
          return;
        }
        if (ev.current_step) setStep(ev.current_step);
        if (ev.event === "failed") {
          setStatus("failed");
          setError(ev.error ?? "step failed");
          setResumable(ev.resumable ?? true);
          closeStream();
        } else if (ev.event === "completed" || ev.status === "completed") {
          setStatus("completed");
          closeStream();
        } else if (ev.event === "cancelled") {
          setStatus("cancelled");
          closeStream();
        }
      };
      es.onmessage = onMsg;
      es.onerror = () => {
        // The stream closes when the run is terminal; the snapshot we already
        // rendered is authoritative, so a closed stream is not an error here.
        closeStream();
      };
    },
    [client, closeStream],
  );

  React.useEffect(() => () => closeStream(), [closeStream]);

  const launch = React.useCallback(() => {
    setError(null);
    setStatus("running");
    setStep(null);
    setResumable(false);
    fetchJson<{ run_id: string }>(
      client,
      `${client.apiPrefix}/setup/templates/${TEMPLATE_ID}/run`,
      {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ barcode, location }),
      },
    )
      .then((r) => {
        setRunId(r.run_id);
        openStream(r.run_id);
      })
      .catch((e: unknown) => {
        setStatus("failed");
        setError(e instanceof Error ? e.message : String(e));
      });
  }, [client, barcode, location, openStream]);

  const resume = React.useCallback(() => {
    if (!runId) return;
    setError(null);
    setStatus("running");
    setResumable(false);
    fetchJson<{ run_id: string }>(
      client,
      `${client.apiPrefix}/setup/runs/${runId}/resume`,
      { method: "POST" },
    )
      .then(() => openStream(runId))
      .catch((e: unknown) => {
        setStatus("failed");
        setError(e instanceof Error ? e.message : String(e));
      });
  }, [client, runId, openStream]);

  return (
    <>
      {/* Header */}
      <div className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1.5">
          <p className="text-sm text-muted-foreground">Acme Devices</p>
          <h1 className="flex items-center gap-2 text-2xl font-semibold tracking-tight">
            <Cpu className="size-6" /> Provision a device
          </h1>
        </div>
        <StatusBadge status={status} />
      </div>

      {/* Launch form */}
      <Card>
        <CardHeader>
          <CardTitle>Scan to provision</CardTitle>
          <CardDescription>
            Runs the <code className="font-mono">{TEMPLATE_ID}</code> automation:
            instant launch, streamed per-step progress, resume from the failed
            step.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <Separator />
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">Barcode</span>
            <input
              value={barcode}
              onChange={(e) => setBarcode(e.target.value)}
              placeholder="scan or type a box barcode"
              className="h-9 rounded-md border border-input bg-transparent px-3 text-sm outline-none ring-ring focus-visible:ring-2"
            />
          </label>
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">Install location</span>
            <input
              value={location}
              onChange={(e) => setLocation(e.target.value)}
              className="h-9 rounded-md border border-input bg-transparent px-3 text-sm outline-none ring-ring focus-visible:ring-2"
            />
          </label>
        </CardContent>
        <CardFooter className="gap-2">
          <Button
            size="sm"
            onClick={launch}
            disabled={!barcode || status === "running"}
          >
            {status === "running" ? (
              <Loader2 className="animate-spin" />
            ) : (
              <ScanLine />
            )}
            {status === "running" ? "Provisioning…" : "Provision"}
          </Button>
          {status === "failed" && resumable ? (
            <Button size="sm" variant="outline" onClick={resume}>
              <RotateCcw /> Retry from failed step
            </Button>
          ) : null}
        </CardFooter>
      </Card>

      {/* Progress */}
      {runId ? (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Run progress</CardTitle>
            <CardDescription>
              run <code className="font-mono">{runId}</code>
            </CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-2 text-sm">
            <Separator />
            <Row label="Status" value={status} />
            <Row label="Current step" value={step ?? "—"} />
            {error ? (
              <p className="text-sm text-destructive">error: {error}</p>
            ) : null}
            {status === "completed" ? (
              <p className="flex items-center gap-2 text-sm text-emerald-600">
                <CheckCircle2 className="size-4" /> Device provisioned.
              </p>
            ) : null}
          </CardContent>
        </Card>
      ) : null}
    </>
  );
}

function Row({ label, value }: { label: string; value: string }): React.ReactElement {
  return (
    <div className="grid grid-cols-[8rem_1fr] items-center gap-x-6 py-1">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-mono tabular-nums">{value}</span>
    </div>
  );
}

function StatusBadge({ status }: { status: RunStatus }): React.ReactElement {
  if (status === "completed")
    return (
      <Badge variant="success">
        <CheckCircle2 /> Done
      </Badge>
    );
  if (status === "failed")
    return (
      <Badge variant="destructive">
        <XCircle /> Failed
      </Badge>
    );
  if (status === "running")
    return (
      <Badge variant="secondary">
        <Loader2 className="animate-spin" /> Running
      </Badge>
    );
  return <Badge variant="secondary">Idle</Badge>;
}
