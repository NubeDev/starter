// `index.tsx` — PWA step machine: Scan → Identify → Place → Confirm.
// Mobile-first, big tap targets. Decode happens on scan, before Identify.
import * as React from "react";
import { ArrowLeft, Loader2 } from "lucide-react";
import { decode } from "../provision/bc-api";
import type { ScannedIdentity } from "../provision/bc-types";
import { Scan } from "./scan";
import { Identify } from "./identify";
import { Place, EMPTY_PLACEMENT, type Placement } from "./place";
import { Confirm } from "./confirm";

type Step = "scan" | "identify" | "place" | "confirm";
const TITLES: Record<Step, string> = {
  scan: "Scan device",
  identify: "Confirm device",
  place: "Place device",
  confirm: "Provision",
};

export default function PwaApp(): React.ReactElement {
  const [step, setStep] = React.useState<Step>("scan");
  const [barcode, setBarcode] = React.useState("");
  const [identity, setIdentity] = React.useState<ScannedIdentity | null>(null);
  const [placement, setPlacement] = React.useState<Placement>(EMPTY_PLACEMENT);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const reset = () => {
    setStep("scan");
    setBarcode("");
    setIdentity(null);
    setPlacement(EMPTY_PLACEMENT);
    setError(null);
  };

  const onScanned = (code: string) => {
    setBusy(true);
    setError(null);
    decode(code)
      .then((id) => {
        setBarcode(code);
        setIdentity(id);
        setStep("identify");
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setBusy(false));
  };

  const back = () => {
    if (step === "identify") reset();
    else if (step === "place") setStep("identify");
    else if (step === "confirm") setStep("place");
  };

  return (
    <div className="mx-auto flex min-h-[100dvh] w-full max-w-md flex-col gap-4 p-4">
      <header className="flex items-center gap-2">
        {step !== "scan" ? (
          <button type="button" aria-label="Back" onClick={back} className="rounded p-1 hover:bg-accent">
            <ArrowLeft className="size-5" />
          </button>
        ) : null}
        <h2 className="text-lg font-semibold text-foreground">{TITLES[step]}</h2>
        {busy ? <Loader2 className="ml-auto size-5 animate-spin text-muted-foreground" /> : null}
      </header>

      {error ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      {step === "scan" ? <Scan onScanned={onScanned} /> : null}

      {step === "identify" && identity ? (
        <>
          <Identify identity={identity} />
          <button
            type="button"
            onClick={() => setStep("place")}
            className="rounded-lg bg-primary px-4 py-3 text-base font-semibold text-primary-foreground"
          >
            Continue
          </button>
        </>
      ) : null}

      {step === "place" ? (
        <>
          <Place value={placement} onChange={setPlacement} />
          <button
            type="button"
            onClick={() => setStep("confirm")}
            className="rounded-lg bg-primary px-4 py-3 text-base font-semibold text-primary-foreground"
          >
            Continue
          </button>
        </>
      ) : null}

      {step === "confirm" && identity ? (
        <Confirm barcode={barcode} identity={identity} placement={placement} onDone={reset} />
      ) : null}
    </div>
  );
}
