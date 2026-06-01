// `wizard-tab.tsx` — desktop twin of the phone flow: decode → place →
// trend/alarm → provision → success summary.
import * as React from "react";
import { decode, provision } from "../bc-api";
import { buildProvisionInput } from "../build-input";
import type { ProvisionResult, ScannedIdentity } from "../bc-types";
import { Switch } from "../ui/switch";
import { Place, EMPTY_PLACEMENT, type Placement } from "../../pwa/place";
import { IdentifyCard } from "./identify-card";

export function WizardTab(): React.ReactElement {
  const [barcode, setBarcode] = React.useState("");
  const [identity, setIdentity] = React.useState<ScannedIdentity | null>(null);
  const [placement, setPlacement] = React.useState<Placement>(EMPTY_PLACEMENT);
  const [name, setName] = React.useState("");
  const [trend, setTrend] = React.useState(true);
  const [alarm, setAlarm] = React.useState(true);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [result, setResult] = React.useState<ProvisionResult | null>(null);

  const reset = () => {
    setBarcode("");
    setIdentity(null);
    setPlacement(EMPTY_PLACEMENT);
    setName("");
    setResult(null);
    setError(null);
  };

  const runDecode = () => {
    if (!barcode.trim()) return;
    setBusy(true);
    setError(null);
    decode(barcode.trim())
      .then(setIdentity)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false));
  };

  const runProvision = () => {
    setBusy(true);
    setError(null);
    provision(buildProvisionInput(barcode.trim(), placement, { trend, alarm }, name))
      .then(setResult)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false));
  };

  if (result) {
    return (
      <div className="flex flex-col gap-3 rounded-lg border border-emerald-500/40 bg-emerald-500/10 p-4">
        <div className="font-semibold text-foreground">Provisioned device {result.device_id}</div>
        <div className="text-sm text-muted-foreground">
          {result.points} points · {result.widgets} widgets · {result.alarms} alarms · page {result.page_id}
        </div>
        {result.warnings.length > 0 ? (
          <ul className="text-xs text-foreground">
            {result.warnings.map((w, i) => (
              <li key={i}>• {w}</li>
            ))}
          </ul>
        ) : null}
        <button type="button" onClick={reset} className="self-start rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground">
          Provision another
        </button>
      </div>
    );
  }

  return (
    <div className="flex max-w-2xl flex-col gap-4">
      <div className="flex flex-col gap-2">
        <label htmlFor="wiz-bc" className="text-xs font-medium text-muted-foreground">
          Barcode / serial
        </label>
        <div className="flex gap-2">
          <input
            id="wiz-bc"
            value={barcode}
            onChange={(e) => setBarcode(e.target.value)}
            placeholder="rubix://add?... or device serial"
            className="flex-1 rounded-md border border-border/60 bg-background px-3 py-1.5 text-sm text-foreground outline-none focus:border-primary"
          />
          <button
            type="button"
            onClick={runDecode}
            disabled={busy || !barcode.trim()}
            className="rounded-md border border-border/60 px-3 py-1.5 text-sm hover:bg-accent disabled:opacity-50"
          >
            Decode
          </button>
        </div>
      </div>

      {error ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      {identity ? (
        <>
          <IdentifyCard identity={identity} />
          <div className="flex flex-col gap-1">
            <label htmlFor="wiz-name" className="text-xs font-medium text-muted-foreground">
              Device name (optional)
            </label>
            <input
              id="wiz-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={identity.template.display_name}
              className="rounded-md border border-border/60 bg-background px-3 py-1.5 text-sm text-foreground outline-none focus:border-primary"
            />
          </div>
          <Place value={placement} onChange={setPlacement} />
          <div className="flex gap-6 rounded-md border border-border/60 bg-card p-3">
            <Switch checked={trend} onChange={setTrend} label="Trends" />
            <Switch checked={alarm} onChange={setAlarm} label="Alarms" />
          </div>
          <button
            type="button"
            onClick={runProvision}
            disabled={busy}
            className="self-start rounded-md bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground disabled:opacity-50"
          >
            {busy ? "Provisioning…" : "Provision"}
          </button>
        </>
      ) : null}
    </div>
  );
}
