// `label-dialog.tsx` — render + show a device's QR/Code128 label.
import * as React from "react";
import { labelRender } from "../bc-api";
import type { LabelRender } from "../bc-types";
import { Dialog } from "../ui/dialog";

export function LabelDialog({
  deviceId,
  onClose,
}: {
  deviceId: string;
  onClose: () => void;
}): React.ReactElement {
  const [label, setLabel] = React.useState<LabelRender | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    labelRender(deviceId)
      .then(setLabel)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  }, [deviceId]);

  return (
    <Dialog title="Device label" onClose={onClose}>
      {error ? (
        <p className="text-sm text-destructive">{error}</p>
      ) : !label ? (
        <p className="text-sm italic text-muted-foreground">loading…</p>
      ) : (
        <div className="flex flex-col items-center gap-3">
          <div className="text-sm font-semibold text-foreground">{label.display_name}</div>
          <img src={label.qr_url} alt={`QR code for ${label.serial}`} className="size-40 rounded border border-border/60 bg-white p-2" />
          <code className="break-all text-center font-mono text-xs text-muted-foreground">{label.code128}</code>
          <div className="font-mono text-xs text-muted-foreground">{label.serial}</div>
        </div>
      )}
    </Dialog>
  );
}
