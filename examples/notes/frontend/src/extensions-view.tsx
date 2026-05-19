// Extensions admin panel. Lists every loaded extension as a card,
// renders the manifest's `contributes` breakdown, and offers an
// enable/disable toggle. The same bearer the user logged in with is
// forwarded (StarterClient handles `headers`); non-admin tokens get a
// 403 and the caller hides the tab.

import { useCallback, useEffect, useState } from "react";

import {
  ExtensionsClient,
  type ExtensionDetail,
  type ExtensionSummary,
} from "./extensions-client.js";

export function ExtensionsView({ client }: { client: ExtensionsClient }) {
  const [rows, setRows] = useState<ExtensionSummary[]>([]);
  const [details, setDetails] = useState<Record<string, ExtensionDetail>>({});
  const [err, setErr] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setErr(null);
      const list = await client.list();
      setRows(list);
      const fetched = await Promise.all(
        list.map(async (r) => [r.id, await client.get(r.id)] as const),
      );
      setDetails(Object.fromEntries(fetched));
    } catch (e) {
      setErr((e as Error).message);
    }
  }, [client]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function toggle(id: string, enabled: boolean) {
    try {
      await client.setEnabled(id, enabled);
      await refresh();
    } catch (e) {
      setErr((e as Error).message);
    }
  }

  if (err) {
    return (
      <section>
        <p style={{ color: "crimson" }}>{err}</p>
        <button type="button" onClick={() => void refresh()}>retry</button>
      </section>
    );
  }

  if (rows.length === 0) {
    return (
      <section>
        <p style={{ color: "#666" }}>
          No extensions loaded. Drop a bundle into the configured
          extensions directory (default <code>./extensions/</code>) and
          restart the server.
        </p>
      </section>
    );
  }

  return (
    <section style={{ display: "grid", gap: 12 }}>
      {rows.map((r) => (
        <ExtensionCard
          key={r.id}
          summary={r}
          detail={details[r.id]}
          onToggle={(enabled) => void toggle(r.id, enabled)}
        />
      ))}
    </section>
  );
}

function ExtensionCard({
  summary,
  detail,
  onToggle,
}: {
  summary: ExtensionSummary;
  detail?: ExtensionDetail;
  onToggle: (enabled: boolean) => void;
}) {
  const enabled = summary.enabled === "enabled";
  const stateColor =
    summary.state === "failed" || summary.state === "crashed"
      ? "crimson"
      : summary.state === "validated" || summary.state === "running"
        ? "seagreen"
        : "#888";
  const contributes = detail?.manifest?.contributes ?? {};
  return (
    <article
      style={{
        border: "1px solid #ddd",
        borderRadius: 8,
        padding: 16,
        display: "grid",
        gap: 6,
      }}
    >
      <header style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
        <div>
          <strong>{summary.display_name ?? summary.id}</strong>{" "}
          <code style={{ color: "#666" }}>{summary.id}</code>
        </div>
        <small>
          v{summary.version ?? "?"} · {summary.runtime_kind ?? "unknown"}
        </small>
      </header>

      <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
        <span style={{ color: stateColor }}>● {summary.state}</span>
        <span style={{ color: "#666" }}>
          restarts {summary.restart_count} · cap-violations {summary.capability_violations}
        </span>
        <span style={{ marginLeft: "auto" }}>
          <label>
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => onToggle(e.target.checked)}
            />{" "}
            enabled
          </label>
        </span>
      </div>

      {detail?.failure && (
        <pre
          style={{
            background: "#fff5f5",
            color: "crimson",
            padding: 8,
            margin: 0,
            whiteSpace: "pre-wrap",
          }}
        >
          {detail.failure}
        </pre>
      )}

      <dl
        style={{
          display: "grid",
          gridTemplateColumns: "auto 1fr",
          gap: "2px 12px",
          margin: 0,
          fontSize: "0.9em",
        }}
      >
        <dt>tools</dt>
        <dd style={{ margin: 0 }}>
          {contributes.tools?.length
            ? contributes.tools.map((t) => t.id).join(", ")
            : "—"}
        </dd>
        <dt>rest</dt>
        <dd style={{ margin: 0 }}>
          {contributes.rest?.length
            ? contributes.rest.map((r) => `${r.method} ${r.path}`).join(", ")
            : "—"}
        </dd>
        <dt>cli</dt>
        <dd style={{ margin: 0 }}>
          {contributes.cli?.length
            ? contributes.cli.map((c) => c.name).join(", ")
            : "—"}
        </dd>
        <dt>ui</dt>
        <dd style={{ margin: 0 }}>
          {contributes.ui?.exposes?.length
            ? contributes.ui.exposes.map((u) => u.slot ?? u.id).join(", ")
            : "—"}
        </dd>
      </dl>
    </article>
  );
}
