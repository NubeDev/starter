// CSV / PNG export utilities for the dashboard.  Browser-only.

import { asEpochMs, asNumber } from "../types";
import type { UsageBucketRow } from "../types";

function downloadCsv(filename: string, rows: ReadonlyArray<ReadonlyArray<string | number | null>>): void {
  const esc = (v: string | number | null): string => {
    if (v === null || v === undefined) return "";
    const s = String(v);
    return /[",\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const body = rows.map((r) => r.map(esc).join(",")).join("\r\n");
  const blob = new Blob([body], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  // Defer revoke so Chromium has a chance to start the download.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function exportUsageCsv(
  rows: ReadonlyArray<UsageBucketRow>,
  selectedHosts: ReadonlyArray<string>,
  hostName: (uuid: string) => string,
  kindLabel: string,
  rangeLabel: string,
  unit: string | null,
): void {
  if (rows.length === 0) return;
  // Pivot to long form: timestamp, host_uuid, host_name, value.
  // Long form survives schema drift better than a wide-pivot when
  // re-imported into BI tools — and dashboards don't need the
  // narrow column layout, the chart already has that.
  const sel = new Set(selectedHosts);
  const out: Array<ReadonlyArray<string | number | null>> = [
    ["timestamp_iso", "host_uuid", "host_name", "value", "unit", "sample_count"],
  ];
  const sorted = rows.slice().sort((a, b) => {
    const ta = asEpochMs(a.bucket) ?? 0;
    const tb = asEpochMs(b.bucket) ?? 0;
    return ta - tb;
  });
  for (const r of sorted) {
    if (!sel.has(r.host_uuid)) continue;
    const t = asEpochMs(r.bucket);
    out.push([
      t !== null ? new Date(t).toISOString() : "",
      r.host_uuid,
      hostName(r.host_uuid),
      asNumber(r.avg_value),
      unit ?? "",
      Number(r.sample_count) || 0,
    ]);
  }
  const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, "-");
  downloadCsv(
    `rubixos-${kindLabel.toLowerCase()}-${rangeLabel}-${stamp}.csv`,
    out,
  );
}

export function exportChartPng(filenameBase: string): void {
  // uPlot renders into a `<canvas>` inside our section wrapper
  // (`data-chart="usage-ts"`). Composite the canvas onto a dark
  // background so the exported PNG matches what users see on
  // screen (the chart canvas itself is transparent).
  const section = document.querySelector<HTMLElement>('[data-chart="usage-ts"]');
  const src = section?.querySelector<HTMLCanvasElement>("canvas");
  if (!src) return;
  const out = document.createElement("canvas");
  out.width = src.width;
  out.height = src.height;
  const ctx = out.getContext("2d");
  if (!ctx) return;
  ctx.fillStyle = "#0b1220"; // matches the glass surface tone
  ctx.fillRect(0, 0, out.width, out.height);
  ctx.drawImage(src, 0, 0);
  const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, "-");
  out.toBlob((blob) => {
    if (!blob) return;
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${filenameBase}-${stamp}.png`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }, "image/png");
}
