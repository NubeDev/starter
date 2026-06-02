// `wizard-tab.tsx` — desktop twin of the phone flow: choose how to
// identify the device (pick a type, or scan/paste a barcode) → place →
// trend/alarm → provision → success summary.
import * as React from "react";
import { Check, QrCode, ScanLine } from "lucide-react";
import { decode, listTemplates, provision } from "../bc-api";
import { buildProvisionInput } from "../build-input";
import { useRefreshKey } from "../refresh";
import type { ProvisionResult, ScannedIdentity, TemplateRow } from "../bc-types";
import { Switch } from "../ui/switch";
import { Place, EMPTY_PLACEMENT, type Placement } from "../../pwa/place";
import { IdentifyCard } from "./identify-card";

type Mode = "pick" | "barcode";

// A friendly, unique-ish serial for manually-added devices so the
// device gets a stable id without the user inventing one. (No
// Date.now()/Math.random() ban here — this is browser code.)
function genSerial(model: string): string {
  const prefix = (model.replace(/[^a-z0-9]/gi, "").slice(0, 3) || "DEV").toUpperCase();
  const rand = Math.random().toString(36).slice(2, 8).toUpperCase();
  return `${prefix}-${rand}`;
}

// Build the canonical `rubix://add?…` string the decoder understands,
// from a chosen template + generated serial. `network` falls back to
// "rubix" when the template doesn't declare one (decoder requires it).
function barcodeFor(model: string, network: string, serial: string): string {
  const net = network && network.trim() ? network.trim() : "rubix";
  const q = `v=1&id=${encodeURIComponent(serial)}&model=${encodeURIComponent(model)}&network=${encodeURIComponent(net)}`;
  return `rubix://add?${q}`;
}

type StepState = "done" | "active" | "pending";

// Vertical progress rail mirroring the phone flow: Identify → Place →
// Review. Steps are derived from where the user is in the wizard.
function StepRail({ steps }: { steps: ReadonlyArray<{ label: string; hint: string; state: StepState }> }): React.ReactElement {
  return (
    <ol className="flex flex-row gap-4 sm:flex-col sm:gap-0">
      {steps.map((s, i) => {
        const last = i === steps.length - 1;
        return (
          <li key={s.label} className="relative flex flex-1 gap-3 sm:flex-none sm:pb-6">
            {/* connector line (vertical layout only) */}
            {!last ? (
              <span
                aria-hidden
                className={
                  "absolute left-[15px] top-8 hidden h-[calc(100%-1rem)] w-px sm:block " +
                  (s.state === "done" ? "bg-primary/60" : "bg-border/60")
                }
              />
            ) : null}
            <span
              className={
                "z-10 flex size-8 shrink-0 items-center justify-center rounded-full border text-xs font-semibold transition-colors " +
                (s.state === "done"
                  ? "border-primary bg-primary text-primary-foreground"
                  : s.state === "active"
                    ? "border-primary bg-primary/10 text-primary"
                    : "border-border/60 bg-background text-muted-foreground")
              }
            >
              {s.state === "done" ? <Check className="size-4" /> : i + 1}
            </span>
            <div className="min-w-0 pt-0.5">
              <div className={"text-sm font-medium " + (s.state === "pending" ? "text-muted-foreground" : "text-foreground")}>
                {s.label}
              </div>
              <div className="ext-eyebrow mt-0.5">{s.hint}</div>
            </div>
          </li>
        );
      })}
    </ol>
  );
}

export function WizardTab(): React.ReactElement {
  const [mode, setMode] = React.useState<Mode>("pick");
  const [templates, setTemplates] = React.useState<ReadonlyArray<TemplateRow>>([]);
  const [model, setModel] = React.useState("");
  const [serial, setSerial] = React.useState("");
  const [barcode, setBarcode] = React.useState("");

  const [identity, setIdentity] = React.useState<ScannedIdentity | null>(null);
  const [placement, setPlacement] = React.useState<Placement>(EMPTY_PLACEMENT);
  const [name, setName] = React.useState("");
  const [trend, setTrend] = React.useState(true);
  const [alarm, setAlarm] = React.useState(true);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [result, setResult] = React.useState<ProvisionResult | null>(null);
  const refresh = useRefreshKey();

  React.useEffect(() => {
    listTemplates()
      .then((ts) => {
        setTemplates(ts);
        // Default the picker to the first template, generate its serial.
        setModel((cur) => {
          const next = cur && ts.some((t) => t.template === cur) ? cur : ts[0]?.template ?? "";
          return next;
        });
      })
      .catch(() => setTemplates([]));
  }, [refresh]);

  // Whenever the chosen model changes (and the serial is empty or
  // auto-derived), refresh the suggested serial.
  React.useEffect(() => {
    if (model) setSerial(genSerial(model));
  }, [model]);

  const reset = () => {
    setIdentity(null);
    setPlacement(EMPTY_PLACEMENT);
    setName("");
    setBarcode("");
    setResult(null);
    setError(null);
    if (model) setSerial(genSerial(model));
  };

  const runDecode = (raw: string) => {
    const bc = raw.trim();
    if (!bc) return;
    setBusy(true);
    setError(null);
    decode(bc)
      .then(setIdentity)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false));
  };

  const findPicked = () => {
    if (!model || !serial.trim()) return;
    const tpl = templates.find((t) => t.template === model);
    runDecode(barcodeFor(model, tpl?.network ?? "rubix", serial.trim()));
  };

  const runProvision = () => {
    setBusy(true);
    setError(null);
    const bc =
      mode === "barcode"
        ? barcode.trim()
        : barcodeFor(model, templates.find((t) => t.template === model)?.network ?? "rubix", serial.trim());
    provision(buildProvisionInput(bc, placement, { trend, alarm }, name))
      .then(setResult)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false));
  };

  if (result) {
    return (
      <div className="flex max-w-2xl flex-col gap-3 rounded-xl border border-emerald-500/40 bg-emerald-500/10 p-5">
        <div className="flex items-center gap-2 font-semibold text-foreground">
          <span className="flex size-6 items-center justify-center rounded-full bg-emerald-500 text-white">
            <Check className="size-4" />
          </span>
          Added {result.device_id}
        </div>
        <div className="text-sm text-muted-foreground">
          {result.points} readings · {result.widgets} tiles · {result.alarms} alarms ·{" "}
          {result.page_id ? "placed on a dashboard page" : "commissioned · pending placement"}
        </div>
        <p className="text-sm text-foreground">
          {result.page_id ? (
            <>It’s live now — open <span className="font-medium">Page preview</span> to see it, or add another device below.</>
          ) : (
            <>Commissioned as <span className="font-medium">pending</span> — place it on a page later from the <span className="font-medium">Devices</span> tab, or add another device below.</>
          )}
        </p>
        {result.warnings.length > 0 ? (
          <ul className="text-xs text-foreground">
            {result.warnings.map((w, i) => (
              <li key={i}>• {w}</li>
            ))}
          </ul>
        ) : null}
        <button type="button" onClick={reset} className="mt-1 cursor-pointer self-start rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90">
          Add another device
        </button>
      </div>
    );
  }

  const noTemplates = templates.length === 0;
  const steps: ReadonlyArray<{ label: string; hint: string; state: StepState }> = [
    { label: "Identification", hint: identity ? "Done" : "In progress", state: identity ? "done" : "active" },
    { label: "Placement", hint: identity ? "In progress" : "Pending", state: identity ? "active" : "pending" },
    { label: "Review & Finalize", hint: "Pending", state: "pending" },
  ];

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-[200px_minmax(0,42rem)]">
      <aside className="lg:pt-2">
        <StepRail steps={steps} />
      </aside>

      <div className="flex flex-col gap-4">
      {/* Step 1 — identify the device */}
      <div className="ext-glass flex flex-col gap-3 p-5">
        <div>
          <div className="text-base font-semibold text-foreground">1. Which device are you adding?</div>
          <p className="mt-0.5 text-sm text-muted-foreground">
            Pick the device type, or scan/paste the barcode from its sticker.
          </p>
        </div>

        {/* mode toggle */}
        <div className="flex w-fit rounded-lg border border-border/60 bg-muted/20 p-0.5 text-sm">
          <button
            type="button"
            onClick={() => setMode("pick")}
            className={"flex cursor-pointer items-center gap-1.5 rounded-md px-3 py-1.5 transition-colors " + (mode === "pick" ? "bg-card font-medium text-foreground shadow-sm ring-1 ring-border/60" : "text-muted-foreground hover:text-foreground")}
          >
            <QrCode className="size-3.5" /> Choose a type
          </button>
          <button
            type="button"
            onClick={() => setMode("barcode")}
            className={"flex cursor-pointer items-center gap-1.5 rounded-md px-3 py-1.5 transition-colors " + (mode === "barcode" ? "bg-card font-medium text-foreground shadow-sm ring-1 ring-border/60" : "text-muted-foreground hover:text-foreground")}
          >
            <ScanLine className="size-3.5" /> Scan barcode
          </button>
        </div>

        {mode === "pick" ? (
          noTemplates ? (
            <div className="rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm text-foreground">
              No device types yet. Open the <span className="font-medium">Templates</span> tab and add one
              (e.g. a LoRa temperature sensor) — then come back here.
            </div>
          ) : (
            <div className="flex flex-col gap-3 sm:flex-row sm:items-end">
              <label className="flex flex-1 flex-col gap-1.5">
                <span className="ext-eyebrow">Device type</span>
                <select
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  aria-label="Device type"
                  className="cursor-pointer rounded-lg border border-border/60 bg-background px-3 py-2 text-sm text-foreground transition-colors hover:border-border focus:border-primary focus:outline-none"
                >
                  {templates.map((t) => (
                    <option key={t.template} value={t.template}>
                      {t.display_name || t.template}
                    </option>
                  ))}
                </select>
              </label>
              <label className="flex flex-col gap-1.5">
                <span className="ext-eyebrow">Serial / ID</span>
                <input
                  value={serial}
                  onChange={(e) => setSerial(e.target.value)}
                  aria-label="Serial"
                  className="rounded-lg border border-border/60 bg-background px-3 py-2 font-mono text-sm text-foreground outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/30"
                />
              </label>
              <button
                type="button"
                onClick={findPicked}
                disabled={busy || !model || !serial.trim()}
                className="cursor-pointer rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {busy ? "…" : "Continue"}
              </button>
            </div>
          )
        ) : (
          <div className="flex gap-2">
            <input
              value={barcode}
              onChange={(e) => setBarcode(e.target.value)}
              placeholder="rubix://add?... or device serial"
              aria-label="Barcode / serial"
              className="flex-1 rounded-lg border border-border/60 bg-background px-3 py-2 font-mono text-sm text-foreground outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/30"
            />
            <button
              type="button"
              onClick={() => runDecode(barcode)}
              disabled={busy || !barcode.trim()}
              className="cursor-pointer rounded-lg border border-border/60 px-4 py-2 text-sm font-medium transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
            >
              Decode
            </button>
          </div>
        )}
      </div>

      {error ? (
        <div role="alert" className="rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      {/* Step 2 — place + confirm */}
      {identity ? (
        <div className="ext-glass ext-glass--accent flex flex-col gap-4 p-5">
          <div className="text-base font-semibold text-foreground">2. Where should it go?</div>
          <IdentifyCard identity={identity} />
          <div className="flex flex-col gap-1.5">
            <label htmlFor="wiz-name" className="ext-eyebrow">
              Friendly name (optional)
            </label>
            <input
              id="wiz-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={identity.template.display_name}
              className="rounded-lg border border-border/60 bg-background px-3 py-2 text-sm text-foreground outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/30"
            />
          </div>
          <Place value={placement} onChange={setPlacement} />
          <div className="flex gap-6 rounded-lg border border-border/60 bg-background p-3">
            <Switch checked={trend} onChange={setTrend} label="Trends" />
            <Switch checked={alarm} onChange={setAlarm} label="Alarms" />
          </div>
          {(() => {
            const hasSite = !!placement.siteId;
            const hasPage = !!placement.pageId || !!placement.newPage.trim();
            // Site is required; a page is optional. With no page the device
            // is commissioned as `pending` and can be placed later from the
            // Devices tab — so the button enables on site alone.
            const ready = hasSite;
            const hint = !hasSite
              ? "Pick or create a site so the device has a home."
              : !hasPage
                ? "No page selected — the device will be commissioned as pending; you can place it on a page later from Devices."
                : null;
            return (
              <>
                <button
                  type="button"
                  onClick={runProvision}
                  disabled={busy || !ready}
                  title={!hasSite ? hint ?? undefined : undefined}
                  className="cursor-pointer self-start rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
                >
                  {busy ? "Adding…" : hasPage ? "Add device" : "Commission (no page)"}
                </button>
                {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
              </>
            );
          })()}
        </div>
      ) : null}
      </div>
    </div>
  );
}
