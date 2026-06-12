// `onboard.tsx` — a CONSUMER-APP SIMULATION for `com.acme.devices`.
//
// This page role-plays the end-user journey the platform enables: someone buys
// an Acme device, signs up in the companion app, registers the device, and is
// dropped into their OWN workspace — a dashboard + sidebar entry scoped so they
// see only their device. It is the human-facing front of the
// SETUP_AUTOMATION_BUILDER + NAV_USERS_TEAMS flows.
//
// The flow, as a phone-style wizard:
//   ① Buy            — a product card; "I bought this" starts onboarding.
//   ② Create account — POST /auth/signup (self-service, root path, NOT /api/v1).
//   ③ Add device     — POST /api/v1/onboard { email, barcode } which, server-
//                      side and privileged, creates the user's team + membership,
//                      provisions the device tagged to that team, builds a
//                      dashboard + nav node, and grants the team `view` on both.
//   ④ Ready          — show the provisioned device + a deep-link into the new
//                      dashboard. Logging in as that user shows ONLY their device.
//
// Everything here calls the REAL endpoints; nothing is faked. The "app" framing
// is just CSS — a narrow, phone-like column.

import * as React from "react";
import {
  ArrowRight,
  BadgeCheck,
  Box,
  Cpu,
  KeyRound,
  Loader2,
  PartyPopper,
  ScanLine,
  ShieldCheck,
  Sparkles,
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

type Step = "buy" | "signup" | "addDevice" | "ready";

interface OnboardResult {
  user_id: string;
  team_slug: string;
  device_id: string;
  dashboard_id: string;
  dashboard_slug: string;
  nav_node_id: string;
}

// A sample barcode that looks like a box label, stable per session so the demo
// reads naturally ("the device you bought").
function sampleBarcode(): string {
  const part = () =>
    Math.random().toString(36).slice(2, 6).toUpperCase().padEnd(4, "0");
  return `ACME-${part()}-${part()}`;
}

export default function ConsumerOnboard(): React.ReactElement {
  return (
    <BlockShell>
      <div
        data-ext-id={EXTENSION_ID}
        className="mx-auto flex max-w-md flex-col gap-5 p-1"
      >
        <Wizard />
      </div>
    </BlockShell>
  );
}

function Wizard(): React.ReactElement {
  const client = useHostClient();
  const [step, setStep] = React.useState<Step>("buy");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const [email, setEmail] = React.useState("");
  const [password, setPassword] = React.useState("");
  const [barcode, setBarcode] = React.useState(() => sampleBarcode());
  const [location, setLocation] = React.useState("Living Room");
  const [result, setResult] = React.useState<OnboardResult | null>(null);

  // ② Create the account (self-service signup, root path).
  const signup = React.useCallback(() => {
    setError(null);
    setBusy(true);
    fetchJson<{ csrf_token: string }>(client, `/auth/signup`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ email: email.trim().toLowerCase(), password }),
    })
      .then(() => setStep("addDevice"))
      .catch((e: unknown) => setError(friendly(e)))
      .finally(() => setBusy(false));
  }, [client, email, password]);

  // ③ Register the device → build the user's whole workspace server-side.
  const addDevice = React.useCallback(() => {
    setError(null);
    setBusy(true);
    fetchJson<OnboardResult>(client, `${client.apiPrefix}/onboard`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        email: email.trim().toLowerCase(),
        barcode: barcode.trim(),
        location,
      }),
    })
      .then((r) => {
        setResult(r);
        setStep("ready");
      })
      .catch((e: unknown) => setError(friendly(e)))
      .finally(() => setBusy(false));
  }, [client, email, barcode, location]);

  return (
    <>
      {/* App chrome */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="grid size-8 place-items-center rounded-xl bg-primary/15 text-primary">
            <Box className="size-4" />
          </span>
          <div className="flex flex-col leading-tight">
            <span className="text-sm font-semibold">Acme Home</span>
            <span className="text-xs text-muted-foreground">device companion app</span>
          </div>
        </div>
        <StepDots step={step} />
      </div>

      {error ? (
        <div className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      {step === "buy" ? (
        <BuyCard onNext={() => setStep("signup")} />
      ) : step === "signup" ? (
        <SignupCard
          email={email}
          password={password}
          busy={busy}
          onEmail={setEmail}
          onPassword={setPassword}
          onBack={() => setStep("buy")}
          onNext={signup}
        />
      ) : step === "addDevice" ? (
        <AddDeviceCard
          barcode={barcode}
          location={location}
          busy={busy}
          onBarcode={setBarcode}
          onLocation={setLocation}
          onShuffle={() => setBarcode(sampleBarcode())}
          onNext={addDevice}
        />
      ) : (
        <ReadyCard
          result={result}
          email={email}
          location={location}
          baseUrl={client.baseUrl}
        />
      )}

      <p className="px-1 text-center text-[11px] leading-relaxed text-muted-foreground">
        A real end-to-end flow: self-service signup, then a privileged backend
        step provisions your device and a private dashboard scoped to you. Log in
        as the new user afterwards and you'll see only your device.
      </p>
    </>
  );
}

function BuyCard({ onNext }: { onNext: () => void }): React.ReactElement {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Sparkles className="size-5" /> You bought an Acme Sensor
        </CardTitle>
        <CardDescription>
          Set it up in two minutes. Create your account, scan the box barcode,
          and your dashboard is ready.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex items-center gap-3 rounded-lg border bg-muted/30 p-3">
          <span className="grid size-12 shrink-0 place-items-center rounded-lg bg-primary/15 text-primary">
            <Cpu className="size-6" />
          </span>
          <div className="flex flex-col">
            <span className="text-sm font-medium">Acme Sensor — Model S1</span>
            <span className="text-xs text-muted-foreground">
              Temperature · humidity · air quality
            </span>
          </div>
          <Badge variant="secondary" className="ml-auto">In the box</Badge>
        </div>
      </CardContent>
      <CardFooter>
        <Button onClick={onNext} className="w-full">
          I bought this — set it up <ArrowRight />
        </Button>
      </CardFooter>
    </Card>
  );
}

function SignupCard({
  email,
  password,
  busy,
  onEmail,
  onPassword,
  onBack,
  onNext,
}: {
  email: string;
  password: string;
  busy: boolean;
  onEmail: (v: string) => void;
  onPassword: (v: string) => void;
  onBack: () => void;
  onNext: () => void;
}): React.ReactElement {
  const canSubmit = email.includes("@") && password.length >= 8 && !busy;
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <KeyRound className="size-5" /> Create your account
        </CardTitle>
        <CardDescription>
          Self-service signup — this creates a real user (default role: reader).
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="text-muted-foreground">Email</span>
          <input
            type="email"
            value={email}
            onChange={(e) => onEmail(e.target.value)}
            placeholder="you@home.example"
            disabled={busy}
            className="h-9 rounded-md border border-input bg-transparent px-3 text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
          />
        </label>
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="text-muted-foreground">
            Password <span className="text-muted-foreground/70">(min 8 chars)</span>
          </span>
          <input
            type="password"
            value={password}
            onChange={(e) => onPassword(e.target.value)}
            placeholder="••••••••"
            disabled={busy}
            className="h-9 rounded-md border border-input bg-transparent px-3 text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
          />
        </label>
      </CardContent>
      <CardFooter className="gap-2">
        <Button variant="ghost" onClick={onBack} disabled={busy}>
          Back
        </Button>
        <Button onClick={onNext} disabled={!canSubmit} className="ml-auto">
          {busy ? <Loader2 className="animate-spin" /> : <ArrowRight />}
          {busy ? "Creating…" : "Create account"}
        </Button>
      </CardFooter>
    </Card>
  );
}

function AddDeviceCard({
  barcode,
  location,
  busy,
  onBarcode,
  onLocation,
  onShuffle,
  onNext,
}: {
  barcode: string;
  location: string;
  busy: boolean;
  onBarcode: (v: string) => void;
  onLocation: (v: string) => void;
  onShuffle: () => void;
  onNext: () => void;
}): React.ReactElement {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ScanLine className="size-5" /> Add your device
        </CardTitle>
        <CardDescription>
          Scan the barcode on the box. (No scanner here — use the sample, or
          shuffle a new one.)
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="text-muted-foreground">Device barcode</span>
          <div className="flex gap-2">
            <input
              value={barcode}
              onChange={(e) => onBarcode(e.target.value)}
              disabled={busy}
              className="h-9 flex-1 rounded-md border border-input bg-transparent px-3 font-mono text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
            />
            <Button type="button" size="sm" variant="outline" onClick={onShuffle} disabled={busy}>
              Shuffle
            </Button>
          </div>
        </label>
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="text-muted-foreground">Where is it?</span>
          <input
            value={location}
            onChange={(e) => onLocation(e.target.value)}
            disabled={busy}
            className="h-9 rounded-md border border-input bg-transparent px-3 text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
          />
        </label>
      </CardContent>
      <CardFooter>
        <Button onClick={onNext} disabled={busy || !barcode.trim()} className="w-full">
          {busy ? <Loader2 className="animate-spin" /> : <ScanLine />}
          {busy ? "Setting up your workspace…" : "Register device"}
        </Button>
      </CardFooter>
    </Card>
  );
}

function ReadyCard({
  result,
  email,
  location,
  baseUrl,
}: {
  result: OnboardResult | null;
  email: string;
  location: string;
  baseUrl: string;
}): React.ReactElement {
  // Deep-link to the freshly-created nexus dashboard. The host serves the SPA at
  // the root, with dashboards under `/dashboards/:slug`.
  const dashUrl = result ? `${baseUrl}/dashboards/${result.dashboard_slug}` : "#";
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-emerald-700 dark:text-emerald-400">
          <PartyPopper className="size-5" /> You're all set
        </CardTitle>
        <CardDescription>
          Your device is registered and your private dashboard is ready.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3 text-sm">
        <div className="flex items-center gap-3 rounded-lg border border-emerald-600/30 bg-emerald-600/10 p-3">
          <BadgeCheck className="size-5 text-emerald-600 dark:text-emerald-400" />
          <div className="flex flex-col">
            <span className="font-medium">Device provisioned</span>
            <span className="font-mono text-xs text-muted-foreground">
              {result?.device_id ?? "—"} · {location}
            </span>
          </div>
        </div>

        <div className="grid grid-cols-[6rem_1fr] gap-x-3 gap-y-1.5 text-xs">
          <span className="text-muted-foreground">Account</span>
          <span className="font-mono">{email}</span>
          <span className="text-muted-foreground">Your space</span>
          <span className="flex items-center gap-1.5">
            <Badge variant="secondary">{result?.team_slug ?? "—"}</Badge>
          </span>
          <span className="text-muted-foreground">Dashboard</span>
          <span className="font-mono">{result?.dashboard_slug ?? "—"}</span>
        </div>

        <Separator />
        <p className="flex items-start gap-2 text-xs text-muted-foreground">
          <ShieldCheck className="mt-0.5 size-3.5 shrink-0" />
          Access is scoped to you: log in as{" "}
          <span className="font-mono text-foreground">{email}</span> and your
          sidebar shows only “My devices” — just this one device, nothing else in
          the tenant.
        </p>
      </CardContent>
      <CardFooter>
        <a href={dashUrl} className="w-full">
          <Button className="w-full">
            Open my dashboard <ArrowRight />
          </Button>
        </a>
      </CardFooter>
    </Card>
  );
}

function StepDots({ step }: { step: Step }): React.ReactElement {
  const order: Step[] = ["buy", "signup", "addDevice", "ready"];
  const idx = order.indexOf(step);
  return (
    <div className="flex items-center gap-1">
      {order.map((s, i) => (
        <span
          key={s}
          className={`h-1.5 rounded-full transition-all ${
            i <= idx ? "w-4 bg-primary" : "w-1.5 bg-muted-foreground/30"
          }`}
        />
      ))}
    </div>
  );
}

function friendly(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e);
  if (/409|conflict|already/i.test(msg)) {
    return "That email is already registered. Try logging in, or use a different email.";
  }
  if (/password/i.test(msg) && /short|length|8/i.test(msg)) {
    return "Password must be at least 8 characters.";
  }
  return msg;
}
