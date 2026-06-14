// `template-qr-dialog.tsx` — generate a printable QR sticker for a template
// (a device *type*) before any device exists. The tech enters/ mints a serial
// and an optional address; we encode the canonical `rubix://add?…` payload so
// scanning the printed sticker round-trips through bc_decode → provision.
//
// NOTE: `qr_url`/the payload is the QR *value* to encode, NOT an image URL —
// we render it with `qrcode.react` (an <img src> of the raw scheme is broken).
import * as React from "react";
import { QRCodeSVG } from "qrcode.react";
import { Printer, Wand2 } from "lucide-react";
import { Dialog } from "../ui/dialog";
import { buildAddUrl, addressLabel, mintSerial } from "../build-add-url";
import type { TemplateRow } from "../bc-types";

export function TemplateQrDialog({
  template,
  onClose,
}: {
  template: TemplateRow;
  onClose: () => void;
}): React.ReactElement {
  const [id, setId] = React.useState("");
  const [address, setAddress] = React.useState("");
  const labelRef = React.useRef<HTMLDivElement>(null);

  const trimmedId = id.trim();
  const payload = trimmedId
    ? buildAddUrl({ id: trimmedId, model: template.template, network: template.network, address })
    : "";

  const print = () => {
    const node = labelRef.current;
    if (!node) return;
    const win = window.open("", "_blank", "width=420,height=560");
    if (!win) return;
    win.document.write(
      `<!doctype html><html><head><title>${trimmedId || "label"}</title>` +
        `<style>body{margin:0;display:grid;place-items:center;min-height:100vh;` +
        `font-family:ui-sans-serif,system-ui,sans-serif}` +
        `.label{display:flex;flex-direction:column;align-items:center;gap:8px;padding:16px}` +
        `.serial{font-family:ui-monospace,monospace;font-size:12px}` +
        `.name{font-weight:700;font-size:16px}</style></head>` +
        `<body><div class="label">${node.innerHTML}</div>` +
        `<script>window.onload=function(){window.print();window.close()}</script></body></html>`,
    );
    win.document.close();
  };

  return (
    <Dialog title={`QR sticker · ${template.display_name}`} onClose={onClose}>
      <div className="flex flex-col gap-4">
        <label className="flex flex-col gap-1.5">
          <span className="text-xs font-medium text-muted-foreground">Device ID / serial</span>
          <div className="flex gap-2">
            <input
              value={id}
              onChange={(e) => setId(e.target.value)}
              placeholder={`${template.template.slice(0, 3).toUpperCase()}-0001`}
              aria-label="Device ID"
              className="flex-1 rounded-lg border border-border/60 bg-background/40 px-3 py-2 font-mono text-sm text-foreground outline-none focus:ring-2 focus:ring-primary/40"
            />
            <button
              type="button"
              onClick={() => setId(mintSerial(template.template))}
              aria-label="Mint an ID"
              className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-border/60 px-3 py-2 text-sm text-primary transition-colors hover:bg-primary/10"
            >
              <Wand2 className="size-4" /> Mint
            </button>
          </div>
        </label>

        <label className="flex flex-col gap-1.5">
          <span className="text-xs font-medium text-muted-foreground">
            {addressLabel(template.network)} (optional)
          </span>
          <input
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            placeholder={template.network === "lora" ? "70B3D5499F2C18" : "192.168.15.42"}
            aria-label={addressLabel(template.network)}
            className="rounded-lg border border-border/60 bg-background/40 px-3 py-2 font-mono text-sm text-foreground outline-none focus:ring-2 focus:ring-primary/40"
          />
        </label>

        {payload ? (
          <div className="flex flex-col items-center gap-4">
            <div ref={labelRef} className="flex flex-col items-center gap-2">
              <div className="rounded-lg bg-white p-3">
                <QRCodeSVG value={payload} size={176} level="M" marginSize={0} title={`QR code for ${trimmedId}`} />
              </div>
              <div className="name text-base font-bold text-foreground">{template.display_name}</div>
              <div className="serial font-mono text-sm text-muted-foreground">{trimmedId}</div>
              <code className="break-all text-center font-mono text-[11px] text-muted-foreground">{payload}</code>
            </div>
            <button
              type="button"
              onClick={print}
              className="flex w-full cursor-pointer items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90"
            >
              <Printer className="size-4" /> Print sticker
            </button>
          </div>
        ) : (
          <p className="py-6 text-center text-sm italic text-muted-foreground">
            Enter or mint an ID to generate the QR.
          </p>
        )}
      </div>
    </Dialog>
  );
}
