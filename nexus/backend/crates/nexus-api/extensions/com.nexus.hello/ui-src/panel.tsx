// `panel.tsx` — the sidebar panel for `com.nexus.hello`.
//
// Deliberately exercises the whole WS-14 loop in one small component: it runs
// the extension's *own contributed query-kind* (`com.nexus.hello.ping`, the
// dispatcher's third source) through the host's client and renders the result.
// If this panel shows a greeting + server time, then federation load, singleton
// negotiation, slot mounting, cookie auth, kind contribution, and kind dispatch
// are all working end to end.

import * as React from "react";
import { fetchJson } from "@nube/starter-client-ts";
import { BlockShell, useHostClient, useSlotContext } from "@nube/starter-ext-sdk-ts";

const EXTENSION_ID = "com.nexus.hello";
const KIND = "com.nexus.hello.ping";

/** The slice of nexus's `QueryResponse` this panel reads. */
interface PingResponse {
  rows: Array<{ greeting?: string; server_time?: string }>;
}

export default function HelloPanel(): React.ReactElement {
  return (
    <BlockShell>
      <PanelInner />
    </BlockShell>
  );
}

function PanelInner(): React.ReactElement {
  const client = useHostClient();
  const slot = useSlotContext();
  const [result, setResult] = React.useState<PingResponse | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    // Kind-mode query: the server resolves `kind` against its registries
    // (file pack → extension-contributed → tenant overlay) — `sql` is ignored.
    fetchJson<PingResponse>(client, `${client.apiPrefix}/query`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ sql: "", kind: KIND }),
    })
      .then((r) => {
        if (!cancelled) setResult(r);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [client]);

  const row = result?.rows?.[0];
  return (
    <div
      data-ext-id={EXTENSION_ID}
      data-ext-slot={slot.slotId}
      style={{
        margin: "4px 8px",
        padding: "8px 10px",
        borderRadius: 8,
        border: "1px solid color-mix(in oklab, currentColor 15%, transparent)",
        fontSize: 12,
        lineHeight: 1.5,
        opacity: 0.9,
      }}
    >
      <div style={{ fontWeight: 600 }}>👋 {EXTENSION_ID}</div>
      {error ? (
        <div style={{ opacity: 0.7 }}>kind query failed: {error}</div>
      ) : row ? (
        <>
          <div>{row.greeting}</div>
          <div style={{ opacity: 0.6, fontVariantNumeric: "tabular-nums" }}>
            {row.server_time}
          </div>
        </>
      ) : (
        <div style={{ opacity: 0.6 }}>running {KIND}…</div>
      )}
    </div>
  );
}
