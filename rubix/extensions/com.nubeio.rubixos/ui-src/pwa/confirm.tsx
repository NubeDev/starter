// `confirm.tsx` — trend/alarm switches + Confirm (provision), then success.
import * as React from "react";
import { CheckCircle2 } from "lucide-react";
import { provision } from "../provision/bc-api";
import { buildProvisionInput } from "../provision/build-input";
import { renderWidget } from "../provision/widgets";
import type { ProvisionResult, ScannedIdentity } from "../provision/bc-types";
import { Switch } from "../provision/ui/switch";
import type { Placement } from "./place";

export function Confirm({
  barcode,
  identity,
  placement,
  onDone,
}: {
  barcode: string;
  identity: ScannedIdentity;
  placement: Placement;
  onDone: () => void;
}): React.ReactElement {
  const [trend, setTrend] = React.useState(true);
  const [alarm, setAlarm] = React.useState(true);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [result, setResult] = React.useState<ProvisionResult | null>(null);

  const submit = () => {
    setBusy(true);
    setError(null);
    provision(buildProvisionInput(barcode, placement, { trend, alarm }))
      .then(setResult)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false));
  };

  if (result) {
    const preview = identity.template.points[0];
    return (
      <div className="flex flex-col items-center gap-4 py-6 text-center">
        <CheckCircle2 className="size-14 text-emerald-500" />
        <div className="text-lg font-semibold text-foreground">Provisioned</div>
        <div className="text-sm text-muted-foreground">
          {result.points} points · {result.widgets} widgets · {result.alarms} alarms
        </div>
        {preview ? (
          <div className="w-full max-w-xs rounded-lg border border-border/60 bg-card p-4">
            {renderWidget(preview.widget, { title: preview.name, widget: preview.widget })}
          </div>
        ) : null}
        {result.warnings.length > 0 ? (
          <ul className="w-full rounded-md border border-yellow-500/40 bg-yellow-500/10 p-3 text-left text-xs text-foreground">
            {result.warnings.map((w, i) => (
              <li key={i}>• {w}</li>
            ))}
          </ul>
        ) : null}
        <button
          type="button"
          onClick={onDone}
          className="w-full rounded-lg bg-primary px-4 py-3 text-base font-semibold text-primary-foreground"
        >
          Scan another
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-3 rounded-xl border border-border/60 bg-card p-4">
        <Switch checked={trend} onChange={setTrend} label="Enable trends" />
        <Switch checked={alarm} onChange={setAlarm} label="Enable alarms" />
      </div>
      {error ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}
      <button
        type="button"
        onClick={submit}
        disabled={busy}
        className="rounded-lg bg-primary px-4 py-3 text-base font-semibold text-primary-foreground disabled:opacity-50"
      >
        {busy ? "Provisioning…" : "Confirm & provision"}
      </button>
    </div>
  );
}
