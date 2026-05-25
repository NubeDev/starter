// `ui/remoteEntry.js` — Module-Federation entry for com.rubix.example.
//
// Hand-authored, dependency-free ESM bundle that satisfies the same
// contract a real vite-plugin-federation output would: emit a
// default-exported `{ singletons, init(handle) }` factory, externalise
// React (pulled from `handle.singletons.react`), and register
// components via `handle.register({ components: ... })`.
//
// The runtime mirrors `main.tsx` (the developer-facing source in
// this dir): a small dashboard panel that fetches this extension's
// own manifest from `GET /api/v1/extensions/com.rubix.example`,
// shows id/version/state/enabled, lists every declared contribution
// (tools, skills, flows, ui slots), and offers a refresh button.
//
// Why plain `React.createElement` instead of JSX? This file ships
// to the browser as-is — no transpile step — so JSX is not an
// option until the Phase E build pipeline replaces this file.
//
// Why `fetch` directly instead of going through the host's
// `StarterClient`? The Module-Federation factory contract does not
// expose the host's StarterClient (only the negotiated singletons),
// and `fetch` against same-origin `/api/v1/...` is the canonical
// path. Auth cookies are sent via `credentials: 'same-origin'`.

const ID = "com.rubix.example";

/** @typedef {{ id: string, singletons: Record<string, any>, register(c: { components: Record<string, any> }): void }} ExtensionRemoteHandle */

function buildMainComponent(React) {
  const { createElement: h, Fragment, useState, useEffect } = React;

  function ContribRow({ label, items }) {
    return h(
      Fragment,
      null,
      h("dt", { style: { opacity: 0.7 } }, label),
      h(
        "dd",
        { style: { margin: 0 } },
        items.length === 0
          ? h("span", { style: { opacity: 0.5 } }, "—")
          : items.flatMap((id, i) => [
              i > 0 ? ", " : "",
              h("code", { key: id }, id),
            ]),
      ),
    );
  }

  return function Main(props) {
    const slotId = (props && props.slotId) || "main";
    const [detail, setDetail] = useState(null);
    const [error, setError] = useState(null);
    const [loading, setLoading] = useState(false);
    const [tick, setTick] = useState(0);

    useEffect(() => {
      let cancelled = false;
      setLoading(true);
      setError(null);
      fetch(`/api/v1/extensions/${ID}`, {
        credentials: "same-origin",
        headers: { accept: "application/json" },
      })
        .then(async (res) => {
          if (!res.ok) throw new Error(`HTTP ${res.status}`);
          return res.json();
        })
        .then((d) => {
          if (!cancelled) setDetail(d);
        })
        .catch((e) => {
          if (!cancelled) setError(e && e.message ? e.message : String(e));
        })
        .finally(() => {
          if (!cancelled) setLoading(false);
        });
      return () => {
        cancelled = true;
      };
    }, [tick]);

    const c = (detail && detail.manifest && detail.manifest.contributes) || {};
    const tools = (c.tools || []).map((t) => t.id);
    const skills = (c.skills || []).map((s) => s.dir);
    const flows = (c.flows || []).map((f) => f.id);
    const exposes = ((c.ui && c.ui.exposes) || []).map((e) => e.slot);
    const version = detail && detail.manifest && detail.manifest.version;

    return h(
      "section",
      {
        "data-ext-id": ID,
        "data-ext-slot": slotId,
        style: {
          padding: "1rem 1.25rem",
          borderRadius: "0.75rem",
          border: "1px solid var(--color-border, rgba(0,0,0,0.1))",
          background: "var(--color-surface, transparent)",
          color: "var(--color-foreground, inherit)",
          display: "flex",
          flexDirection: "column",
          gap: "0.75rem",
        },
      },
      h(
        "header",
        {
          style: {
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "1rem",
          },
        },
        h(
          "div",
          null,
          h(
            "h3",
            { style: { margin: 0, fontSize: "1rem" } },
            ID,
            version
              ? h(
                  "span",
                  { style: { opacity: 0.6, fontWeight: 400 } },
                  " v" + version,
                )
              : null,
          ),
          h(
            "small",
            { style: { opacity: 0.7 } },
            "slot=",
            h("code", null, slotId),
            detail
              ? h(
                  Fragment,
                  null,
                  " · state=",
                  h("code", null, detail.state),
                  " · enabled=",
                  h("code", null, detail.enabled),
                )
              : null,
          ),
        ),
        h(
          "button",
          {
            type: "button",
            onClick: () => setTick((t) => t + 1),
            disabled: loading,
            style: {
              padding: "0.35rem 0.75rem",
              borderRadius: "0.375rem",
              border: "1px solid var(--color-border, rgba(0,0,0,0.15))",
              background: "transparent",
              color: "inherit",
              cursor: loading ? "wait" : "pointer",
              font: "inherit",
            },
          },
          loading ? "loading…" : "refresh",
        ),
      ),
      error
        ? h(
            "p",
            {
              role: "alert",
              style: {
                margin: 0,
                padding: "0.5rem 0.75rem",
                borderRadius: "0.375rem",
                background:
                  "var(--color-danger-surface, rgba(220,38,38,0.08))",
                color: "var(--color-danger, rgb(185,28,28))",
                fontSize: "0.875rem",
              },
            },
            "failed to load manifest: " + error,
          )
        : null,
      h(
        "dl",
        {
          style: {
            margin: 0,
            display: "grid",
            gridTemplateColumns: "max-content 1fr",
            gap: "0.25rem 0.75rem",
            fontSize: "0.875rem",
          },
        },
        h(ContribRow, { label: "tools", items: tools }),
        h(ContribRow, { label: "skills", items: skills }),
        h(ContribRow, { label: "flows", items: flows }),
        h(ContribRow, { label: "ui slots", items: exposes }),
      ),
    );
  };
}

const factory = {
  singletons: {
    react: { version: "19.0.0" },
  },
  /** @param {ExtensionRemoteHandle} handle */
  init(handle) {
    const React = handle.singletons.react;
    if (!React || typeof React.createElement !== "function") {
      throw new Error(
        `[${ID}] init received no usable React singleton — host did not provide one`,
      );
    }
    handle.register({
      components: { Main: buildMainComponent(React) },
    });
  },
};

export default factory;
