// `label-dialog.tsx` — render + show a device's QR/Code128 label.
//
// NOTE: the backend's `qr_url` field is the QR *payload* (the canonical
// `rubix://add?…` string to encode), NOT an image URL. We render it into
// an actual QR with `qrcode.react` — pointing an <img src> at the raw
// payload (a custom scheme) just yields a broken image.
import * as React from "react";
import { QRCodeSVG } from "qrcode.react";
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
          <div className="rounded-lg border border-border/60 bg-white p-3">
            <QRCodeSVG
              value={label.qr_url}
              size={160}
              level="M"
              marginSize={0}
              title={`QR code for ${label.serial}`}
            />
          </div>
          <code className="break-all text-center font-mono text-xs text-muted-foreground">{label.code128}</code>
          <div className="font-mono text-xs text-muted-foreground">{label.serial}</div>
        </div>
      )}
    </Dialog>
  );
}
