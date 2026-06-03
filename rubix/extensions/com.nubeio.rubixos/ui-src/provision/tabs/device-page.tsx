// `device-page.tsx` — full-page detail view for a single device. Shows
// identity (name, status, template, network/addr, site/page), its points,
// and a Share card with a QR + copy-link that encode the *page URL* (so a
// scan opens this page), distinct from the hardware-provisioning label.
import * as React from "react";
import { QRCodeSVG } from "qrcode.react";
import { ArrowLeft, Check, Copy, QrCode, Radio } from "lucide-react";
import { getDevice } from "../bc-api";
import { deviceShareUrl, gotoDevicesList } from "../nav";
import { statusTone } from "../status";
import type { DeviceRow } from "../bc-types";
import { DeviceDetail } from "./device-detail";
import { LabelDialog } from "./label-dialog";

export function DevicePage({ deviceId }: { deviceId: string }): React.ReactElement {
  const [device, setDevice] = React.useState<DeviceRow | null>(null);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [copied, setCopied] = React.useState(false);
  const [showLabel, setShowLabel] = React.useState(false);

  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    // Single-device read (R of CRUD) — fetches just this row by id
    // rather than listing the fleet and filtering client-side.
    getDevice(deviceId)
      .then((row) => {
        if (cancelled) return;
        setDevice(row);
      })
      .catch((e: unknown) => !cancelled && setError(e instanceof Error ? e.message : String(e)))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [deviceId]);

  const shareUrl = deviceShareUrl(deviceId);
  const copy = () => {
    navigator.clipboard?.writeText(shareUrl).then(
      () => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1500);
      },
      () => undefined,
    );
  };

  const back = (
    <button
      type="button"
      onClick={gotoDevicesList}
      className="mb-1 flex cursor-pointer items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
    >
      <ArrowLeft className="size-4" /> Back to devices
    </button>
  );

  if (loading) {
    return <div className="flex flex-col gap-2">{back}<p className="text-sm italic text-muted-foreground">Loading device…</p></div>;
  }
  if (error) {
    return <div className="flex flex-col gap-2">{back}<p className="text-sm text-destructive">{error}</p></div>;
  }
  if (!device) {
    return (
      <div className="flex flex-col gap-2">
        {back}
        <p className="text-sm italic text-muted-foreground">
          Device <span className="font-mono">{deviceId}</span> not found.
        </p>
      </div>
    );
  }

  const tone = statusTone(device.status);

  return (
    <div className="flex flex-col gap-4">
      {back}

      {/* Identity header */}
      <div className="ext-glass flex flex-col gap-4 p-5 sm:flex-row sm:items-start sm:justify-between">
        <div className="flex items-start gap-3">
          <span className="flex size-12 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
            <Radio className="size-6" />
          </span>
          <div className="min-w-0">
            <h2 className="text-xl font-semibold tracking-tight text-foreground">
              {device.name ?? device.device_id}
            </h2>
            <div className="mt-0.5 font-mono text-xs uppercase tracking-wide text-muted-foreground">
              {device.device_id}
            </div>
            <div className="mt-2 inline-flex items-center gap-1.5 text-sm font-medium">
              <span className={"inline-block size-1.5 rounded-full " + tone.dot} />
              <span className={tone.text}>{device.status}</span>
            </div>
          </div>
        </div>
        <button
          type="button"
          onClick={() => setShowLabel(true)}
          className="flex cursor-pointer items-center gap-1.5 self-start rounded-lg border border-border/60 px-3 py-2 text-sm font-medium text-foreground transition-colors hover:bg-accent"
        >
          <QrCode className="size-4" /> Provisioning label
        </button>
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
        {/* Left: specs + points */}
        <div className="flex flex-col gap-4">
          <div className="ext-glass p-5">
            <div className="ext-eyebrow mb-3">Specifications</div>
            <dl className="grid grid-cols-2 gap-x-6 gap-y-3 text-sm">
              <Spec label="Template" value={device.template} mono />
              <Spec label="Network" value={device.network ?? "—"} mono />
              <Spec label="Address" value={device.address ?? "—"} mono />
              <Spec label="Default IP" value={device.default_ip ?? "—"} mono />
              <Spec label="HW rev" value={device.hw_rev ?? "—"} mono />
              <Spec label="Status" value={device.status} />
              <Spec label="Site" value={device.site_id ?? "Unassigned"} mono />
              <Spec label="Page" value={device.page_id ?? "—"} mono />
              <Spec label="Provisioned" value={device.provisioned_at ?? "—"} />
            </dl>
          </div>

          <div className="ext-glass p-5">
            <div className="ext-eyebrow mb-3">Points</div>
            <DeviceDetail deviceId={device.device_id} />
          </div>
        </div>

        {/* Right: share card */}
        <div className="ext-glass flex h-fit flex-col items-center gap-3 p-5">
          <div className="ext-eyebrow self-start">Share this device</div>
          <div className="rounded-lg border border-border/60 bg-white p-3">
            <QRCodeSVG value={shareUrl} size={176} level="M" marginSize={0} title={`Open ${device.name ?? device.device_id}`} />
          </div>
          <p className="text-center text-xs text-muted-foreground">
            Scan to open this device&apos;s page on another device.
          </p>
          <div className="flex w-full items-center gap-2 rounded-lg border border-border/60 bg-background p-2">
            <span className="min-w-0 flex-1 truncate font-mono text-xs text-muted-foreground" title={shareUrl}>
              {shareUrl}
            </span>
            <button
              type="button"
              onClick={copy}
              aria-label="Copy link"
              className={
                "flex shrink-0 cursor-pointer items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition-colors " +
                (copied ? "text-emerald-400" : "text-foreground hover:bg-accent")
              }
            >
              {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
              {copied ? "Copied" : "Copy"}
            </button>
          </div>
        </div>
      </div>

      {showLabel ? <LabelDialog deviceId={device.device_id} onClose={() => setShowLabel(false)} /> : null}
    </div>
  );
}

function Spec({ label, value, mono }: { label: string; value: string; mono?: boolean }): React.ReactElement {
  return (
    <div className="flex flex-col gap-0.5">
      <dt className="ext-eyebrow">{label}</dt>
      <dd className={"truncate text-foreground " + (mono ? "font-mono text-xs" : "")} title={value}>
        {value}
      </dd>
    </div>
  );
}
