// `wiresheet/wiresheet-panel.tsx` — full-bleed remote-programming canvas for
// one control engine.
//
// Looks up the selected engine's connection row (ip/port) via the `device_get`
// warehouse template, then mounts the shared editor (`@nube/ce-wiresheet`)
// pointed straight at that engine over REST + WebSocket (Model A — direct
// browser→CE, LAN/http). The same package powers the standalone dev harness.
//
// The surface is a fixed full-viewport overlay (not a boxed panel): the editor
// owns the whole screen so its viewport-anchored floating toolbar/panels line
// up with the canvas, exactly as in the standalone app. The host's own chrome
// (Page title, sidebar) is intentionally covered while editing.

import * as React from "react";

import { CeEditor } from "@nube/ce-wiresheet";

import { EXTENSION_ID, type DeviceRow } from "../types";
import { fetchTemplate } from "../api";

export function WiresheetPanel({ deviceId }: { deviceId: string }): React.ReactElement {
  const [device, setDevice] = React.useState<DeviceRow | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(true);

  React.useEffect(() => {
    let alive = true;
    setLoading(true);
    setError(null);
    fetchTemplate<DeviceRow>(`${EXTENSION_ID}.device_get`, { device_id: deviceId })
      .then((rows) => {
        if (!alive) return;
        setDevice(rows[0] ?? null);
        setLoading(false);
      })
      .catch((e: unknown) => {
        if (!alive) return;
        setError(e instanceof Error ? e.message : String(e));
        setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [deviceId]);

  // The engine serves REST under `/api/v0` and the value WS under `/ws` at its
  // own origin; pass that origin to the editor.
  const base = device ? `http://${device.ip}:${device.port}` : null;

  return (
    <div
      style={{
        // Full-bleed, but below the host sidebar (which is `fixed … z-10`) so
        // the rubix nav floats over the wiresheet rather than being covered.
        position: "fixed",
        inset: 0,
        zIndex: 5,
        background: "#0b0e14",
      }}
    >
      {loading && <Centered>Loading engine…</Centered>}
      {!loading && error && (
        <Centered>
          Failed to load engine <code>{deviceId}</code>: {error}
        </Centered>
      )}
      {!loading && !error && !device && (
        <Centered>
          No such engine: <code>{deviceId}</code>
        </Centered>
      )}
      {!loading && base && <CeEditor base={base} />}
    </div>
  );
}

function Centered({ children }: { children: React.ReactNode }): React.ReactElement {
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        color: "#9aa4b2",
        fontSize: 14,
        gap: 6,
      }}
    >
      {children}
    </div>
  );
}
