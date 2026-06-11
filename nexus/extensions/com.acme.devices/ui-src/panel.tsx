// `panel.tsx` — the `main`-slot "Provision device" page for
// `com.acme.devices`. A self-explanatory, end-to-end demonstration of the
// Setup / Automation Builder, built from REAL shadcn/ui primitives (Card,
// Button, Badge, Separator, Progress) vendored under `components/ui/`.
//
// What this page demonstrates (and explains to a first-time viewer):
//   • A "barcode" stands for a physical device's box label a field tech would
//     scan. Here it is just INPUT DATA + the idempotency key for the run.
//   • Pressing Provision launches the `com.acme.add-device` automation:
//       POST /setup/templates/{id}/run → 202 { run_id }  (returns in ms)
//   • The run executes two steps IN THE EXTENSION'S OWN CHILD PROCESS over the
//     flow-node bridge: ① create the device, ② register its sensor.
//   • Progress streams live over SSE (GET /setup/runs/{id}/events); we also
//     poll the snapshot so a fast run still shows its steps + identity.
//   • The run is tagged with TRUSTED IDENTITY (owner / team / tenant) seeded by
//     the server from the verified session — never from this form.
//   • If a step fails, Retry resumes FROM THE FAILED STEP
//     (POST /setup/runs/{id}/resume); device.create is idempotent on the
//     barcode, so resume never double-creates.
//
// Styling: the extension ships its own scoped Tailwind v4 bundle (app.css +
// vite.config.ts), so every shadcn token class resolves against the host theme.
// The whole page is wrapped in `<div data-ext-id="com.acme.devices">`.

import * as React from "react";
import {
  CheckCircle2,
  ChevronRight,
  Cpu,
  Dice5,
  Info,
  Loader2,
  RotateCcw,
  ScanLine,
  ShieldCheck,
  Ban,
  Radio,
  XCircle,
  Circle,
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
import { Progress } from "./components/ui/progress";

const EXTENSION_ID = "com.acme.devices";
const TEMPLATE_ID = "com.acme.add-device";

// The two real flow-node ids the `add-device` template runs, in order, with
// friendly labels. The SSE `current_step` / snapshot `current_step` carry these
// reverse-DNS node ids; we map them to a human step in the timeline.
const STEPS: { node: string; title: string; detail: string }[] = [
  {
    node: "com.acme.create-device",
    title: "Create device",
    detail: "Provision a device record from the barcode (idempotent on barcode).",
  },
  {
    node: "com.acme.register-sensor",
    title: "Register sensor",
    detail: "Attach the device's sensor (idempotent on device id).",
  },
];

type RunStatus = "idle" | "running" | "completed" | "failed" | "cancelled";

interface StepEvent {
  event?: string;
  current_step?: string;
  total?: number;
  status?: string;
  error?: string;
  resumable?: boolean;
}

interface RunSnapshot {
  run_id: string;
  owner?: string;
  tenant_id?: string;
  team?: string | null;
  status?: string;
  progress?: { done?: number; total?: number; current_step?: string | null };
  resumable?: boolean;
  finished_at?: string | null;
}

// Mirror the extension child's `stable_id` (FNV-1a → `dev-<hex>`), a pure
// function of the barcode (DOCS §8c). Showing it here lets the UI display the
// real provisioned device id and prove idempotency (same barcode → same id)
// without the REST surface having to echo run outputs.
function stableId(prefix: string, key: string): string {
  let hash = 0xcbf29ce484222325n;
  for (const b of new TextEncoder().encode(key)) {
    hash ^= BigInt(b);
    hash = (hash * 0x100000001b3n) & 0xffffffffffffffffn;
  }
  return `${prefix}-${hash.toString(16).padStart(16, "0")}`;
}

function randomBarcode(): string {
  // Looks like a scanned box label: ACME-XXXX-XXXX.
  const part = () =>
    Math.random().toString(36).slice(2, 6).toUpperCase().padEnd(4, "0");
  return `ACME-${part()}-${part()}`;
}

export default function DevicesPanel(): React.ReactElement {
  return (
    <BlockShell>
      <div
        data-ext-id={EXTENSION_ID}
        className="mx-auto flex max-w-3xl flex-col gap-5 p-1"
      >
        <PageInner />
      </div>
    </BlockShell>
  );
}

function PageInner(): React.ReactElement {
  const client = useHostClient();
  const [barcode, setBarcode] = React.useState(() => randomBarcode());
  const [location, setLocation] = React.useState("Roof AHU-3");
  const [runId, setRunId] = React.useState<string | null>(null);
  const [status, setStatus] = React.useState<RunStatus>("idle");
  const [step, setStep] = React.useState<string | null>(null);
  const [done, setDone] = React.useState(0);
  const [error, setError] = React.useState<string | null>(null);
  const [resumable, setResumable] = React.useState(false);
  const [snap, setSnap] = React.useState<RunSnapshot | null>(null);
  const esRef = React.useRef<EventSource | null>(null);
  const pollRef = React.useRef<ReturnType<typeof setInterval> | null>(null);

  const stopPoll = React.useCallback(() => {
    if (pollRef.current) clearInterval(pollRef.current);
    pollRef.current = null;
  }, []);

  const closeStream = React.useCallback(() => {
    esRef.current?.close();
    esRef.current = null;
  }, []);

  const refreshSnapshot = React.useCallback(
    async (id: string) => {
      try {
        const s = await fetchJson<RunSnapshot>(
          client,
          `${client.apiPrefix}/setup/runs/${id}`,
        );
        setSnap(s);
        if (typeof s.progress?.done === "number") setDone(s.progress.done);
        if (s.progress?.current_step) setStep(s.progress.current_step);
        if (s.status === "completed") {
          setStatus("completed");
          closeStream();
          stopPoll();
        } else if (s.status === "failed") {
          setStatus("failed");
          setResumable(s.resumable ?? true);
          closeStream();
          stopPoll();
        } else if (s.status === "cancelled") {
          setStatus("cancelled");
          closeStream();
          stopPoll();
        }
      } catch {
        /* transient; the next tick / SSE will reconcile */
      }
    },
    [client, closeStream, stopPoll],
  );

  // Tail the SSE progress stream. Cookie auth rides the EventSource via
  // `withCredentials`; the host serves the same origin in dev.
  const openStream = React.useCallback(
    (id: string) => {
      closeStream();
      const url = `${client.apiPrefix}/setup/runs/${id}/events`;
      const es = new EventSource(url, { withCredentials: true });
      esRef.current = es;
      es.onmessage = (raw: MessageEvent) => {
        let ev: StepEvent;
        try {
          ev = JSON.parse(raw.data) as StepEvent;
        } catch {
          return;
        }
        if (ev.current_step) setStep(ev.current_step);
        if (typeof ev.total === "number") {
          // a "step" event marks one node starting; reconcile via snapshot.
          void refreshSnapshot(id);
        }
        if (ev.event === "failed") {
          setStatus("failed");
          setError(ev.error ?? "step failed");
          setResumable(ev.resumable ?? true);
          closeStream();
          stopPoll();
        } else if (ev.event === "completed" || ev.status === "completed") {
          setStatus("completed");
          setDone(STEPS.length);
          void refreshSnapshot(id);
          closeStream();
          stopPoll();
        } else if (ev.event === "cancelled") {
          setStatus("cancelled");
          closeStream();
          stopPoll();
        }
      };
      es.onerror = () => {
        // Terminal runs close the stream; the snapshot poll is authoritative.
        closeStream();
      };
    },
    [client, closeStream, refreshSnapshot, stopPoll],
  );

  // Poll the snapshot too — fast runs can finish before the SSE connects, and
  // the snapshot carries the trusted identity (owner/team/tenant) to display.
  const startPoll = React.useCallback(
    (id: string) => {
      stopPoll();
      void refreshSnapshot(id);
      pollRef.current = setInterval(() => void refreshSnapshot(id), 600);
    },
    [refreshSnapshot, stopPoll],
  );

  React.useEffect(
    () => () => {
      closeStream();
      stopPoll();
    },
    [closeStream, stopPoll],
  );

  const launch = React.useCallback(() => {
    setError(null);
    setStatus("running");
    setStep(null);
    setDone(0);
    setSnap(null);
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
        startPoll(r.run_id);
      })
      .catch((e: unknown) => {
        setStatus("failed");
        setError(friendlyError(e));
      });
  }, [client, barcode, location, openStream, startPoll]);

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
      .then(() => {
        openStream(runId);
        startPoll(runId);
      })
      .catch((e: unknown) => {
        setStatus("failed");
        setError(friendlyError(e));
      });
  }, [client, runId, openStream, startPoll]);

  const cancel = React.useCallback(() => {
    if (!runId) return;
    fetchJson<{ cancelled: boolean }>(
      client,
      `${client.apiPrefix}/setup/runs/${runId}/cancel`,
      { method: "POST" },
    )
      .then(() => {
        setStatus("cancelled");
        closeStream();
        stopPoll();
        if (runId) void refreshSnapshot(runId);
      })
      .catch((e: unknown) => setError(friendlyError(e)));
  }, [client, runId, closeStream, stopPoll, refreshSnapshot]);

  const reset = React.useCallback(() => {
    closeStream();
    stopPoll();
    setRunId(null);
    setStatus("idle");
    setStep(null);
    setDone(0);
    setError(null);
    setResumable(false);
    setSnap(null);
    setBarcode(randomBarcode());
  }, [closeStream, stopPoll]);

  const busy = status === "running";
  const deviceId = barcode ? stableId("dev", barcode) : "";
  const sensorId = deviceId ? stableId("sen", deviceId) : "";

  return (
    <>
      {/* Header */}
      <div className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1.5">
          <p className="text-sm text-muted-foreground">Acme Devices · Setup automation demo</p>
          <h1 className="flex items-center gap-2 text-2xl font-semibold tracking-tight">
            <Cpu className="size-6" /> Provision a device
          </h1>
        </div>
        <StatusBadge status={status} />
      </div>

      {/* What this is — answers "what's a barcode?" up front */}
      <Card className="border-dashed bg-muted/30">
        <CardContent className="flex gap-3 pt-6 text-sm">
          <Info className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
          <div className="flex flex-col gap-1.5 text-muted-foreground">
            <p>
              <span className="font-medium text-foreground">What's happening here?</span>{" "}
              This simulates a field technician scanning a new device's{" "}
              <span className="font-medium text-foreground">box barcode</span> to set
              it up. The barcode is just an identifier — provisioning runs a small
              multi-step <span className="font-medium text-foreground">automation</span>{" "}
              on the server.
            </p>
            <p>
              You don't have a real scanner, so use the sample barcode below (or{" "}
              <span className="font-medium text-foreground">Randomize</span> a new one),
              then press <span className="font-medium text-foreground">Provision</span>.
              The same barcode always provisions the same device — that's the
              idempotency the automation guarantees.
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Launch form */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ScanLine className="size-5" /> Scan to provision
          </CardTitle>
          <CardDescription>
            Runs the <code className="font-mono">{TEMPLATE_ID}</code> automation —
            instant launch, streamed per-step progress, resume from a failed step.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <Separator />
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">
              Device barcode <span className="text-muted-foreground/70">(scanned box label)</span>
            </span>
            <div className="flex gap-2">
              <input
                value={barcode}
                onChange={(e) => setBarcode(e.target.value)}
                placeholder="e.g. ACME-7F3A-9C21"
                disabled={busy}
                className="h-9 flex-1 rounded-md border border-input bg-transparent px-3 font-mono text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
              />
              <Button
                type="button"
                size="sm"
                variant="outline"
                onClick={() => setBarcode(randomBarcode())}
                disabled={busy}
                title="Generate a sample barcode"
              >
                <Dice5 /> Randomize
              </Button>
            </div>
            {deviceId ? (
              <span className="text-xs text-muted-foreground">
                → will provision device{" "}
                <code className="font-mono text-foreground">{deviceId}</code>
              </span>
            ) : null}
          </label>
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">Install location</span>
            <input
              value={location}
              onChange={(e) => setLocation(e.target.value)}
              disabled={busy}
              className="h-9 rounded-md border border-input bg-transparent px-3 text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
            />
          </label>
        </CardContent>
        <CardFooter className="flex-wrap gap-2">
          <Button onClick={launch} disabled={!barcode || busy}>
            {busy ? <Loader2 className="animate-spin" /> : <ScanLine />}
            {busy ? "Provisioning…" : "Provision"}
          </Button>
          {status === "failed" && resumable ? (
            <Button variant="default" onClick={resume}>
              <RotateCcw /> Retry from failed step
            </Button>
          ) : null}
          {busy ? (
            <Button variant="outline" onClick={cancel}>
              <Ban /> Cancel
            </Button>
          ) : null}
          {runId && !busy ? (
            <Button variant="ghost" onClick={reset}>
              Provision another
            </Button>
          ) : null}
        </CardFooter>
      </Card>

      {/* Progress + steps */}
      {runId ? (
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between gap-3">
              <CardTitle className="flex items-center gap-2 text-base">
                <Radio className="size-4" /> Run progress
              </CardTitle>
              <span className="text-xs tabular-nums text-muted-foreground">
                {done}/{STEPS.length} steps
              </span>
            </div>
            <CardDescription>
              run <code className="font-mono">{runId}</code>
            </CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            <Progress value={(done / STEPS.length) * 100} />

            {/* Step timeline */}
            <ol className="flex flex-col gap-1">
              {STEPS.map((s, i) => (
                <StepRow
                  key={s.node}
                  index={i}
                  title={s.title}
                  detail={s.detail}
                  state={stepState(i, done, step, s.node, status)}
                />
              ))}
            </ol>

            {error ? (
              <div className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                {error}
              </div>
            ) : null}

            {/* Result */}
            {status === "completed" ? (
              <div className="flex flex-col gap-2 rounded-md border border-emerald-600/30 bg-emerald-600/10 p-3 text-sm">
                <p className="flex items-center gap-2 font-medium text-emerald-700 dark:text-emerald-400">
                  <CheckCircle2 className="size-4" /> Device provisioned
                </p>
                <div className="grid grid-cols-[7rem_1fr] gap-x-4 gap-y-1 font-mono text-xs text-foreground">
                  <span className="text-muted-foreground">device_id</span>
                  <span>{deviceId}</span>
                  <span className="text-muted-foreground">sensor_id</span>
                  <span>{sensorId}</span>
                  <span className="text-muted-foreground">location</span>
                  <span className="font-sans">{location}</span>
                </div>
              </div>
            ) : null}
          </CardContent>

          {/* Trusted identity — the security boundary */}
          {snap ? (
            <CardFooter className="flex-col items-start gap-2 border-t pt-4">
              <p className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
                <ShieldCheck className="size-3.5" /> Trusted identity — seeded by the
                server from your session, not this form
              </p>
              <div className="flex flex-wrap gap-1.5">
                {snap.team ? <Badge variant="secondary">team: {snap.team}</Badge> : null}
                {snap.tenant_id ? (
                  <Badge variant="secondary">tenant: {snap.tenant_id}</Badge>
                ) : null}
                {snap.owner ? (
                  <Badge variant="outline" className="font-mono">
                    owner: {snap.owner.slice(0, 8)}…
                  </Badge>
                ) : null}
              </div>
            </CardFooter>
          ) : null}
        </Card>
      ) : null}
    </>
  );
}

type StepState = "pending" | "running" | "done" | "failed";

function stepState(
  index: number,
  done: number,
  currentStep: string | null,
  node: string,
  status: RunStatus,
): StepState {
  if (index < done) return "done";
  if (status === "failed" && (currentStep === node || index === done)) return "failed";
  if (status === "running" && (currentStep === node || index === done)) return "running";
  if (status === "completed") return "done";
  return "pending";
}

function StepRow({
  index,
  title,
  detail,
  state,
}: {
  index: number;
  title: string;
  detail: string;
  state: StepState;
}): React.ReactElement {
  return (
    <li className="flex items-start gap-3 rounded-md px-2 py-2">
      <span className="mt-0.5">
        <StepIcon state={state} />
      </span>
      <div className="flex flex-col">
        <span className="flex items-center gap-1.5 text-sm font-medium">
          <span className="text-muted-foreground">{index + 1}.</span> {title}
          {state === "running" ? (
            <span className="text-xs font-normal text-muted-foreground">running…</span>
          ) : null}
        </span>
        <span className="text-xs text-muted-foreground">{detail}</span>
      </div>
    </li>
  );
}

function StepIcon({ state }: { state: StepState }): React.ReactElement {
  switch (state) {
    case "done":
      return <CheckCircle2 className="size-4 text-emerald-600 dark:text-emerald-400" />;
    case "running":
      return <Loader2 className="size-4 animate-spin text-primary" />;
    case "failed":
      return <XCircle className="size-4 text-destructive" />;
    default:
      return <Circle className="size-4 text-muted-foreground/40" />;
  }
}

function StatusBadge({ status }: { status: RunStatus }): React.ReactElement {
  switch (status) {
    case "completed":
      return (
        <Badge variant="success">
          <CheckCircle2 /> Done
        </Badge>
      );
    case "failed":
      return (
        <Badge variant="destructive">
          <XCircle /> Failed
        </Badge>
      );
    case "running":
      return (
        <Badge variant="secondary">
          <Loader2 className="animate-spin" /> Running
        </Badge>
      );
    case "cancelled":
      return (
        <Badge variant="outline">
          <Ban /> Cancelled
        </Badge>
      );
    default:
      return (
        <Badge variant="outline">
          <ChevronRight /> Ready
        </Badge>
      );
  }
}

// Turn a raw API/transport error into something a demo viewer can act on. The
// most common one is the team gate (403) on a caller not in `allowed_teams`.
function friendlyError(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e);
  if (/team/i.test(msg) && /(allowed|forbidden|403)/i.test(msg)) {
    return "Forbidden: your user isn't in a team this template allows (allowed_teams: hvac-ops). Add yourself to that team and retry.";
  }
  if (/403|forbidden|csrf/i.test(msg)) {
    return `${msg} — if this is a CSRF error, reload to refresh your session token.`;
  }
  return msg;
}
