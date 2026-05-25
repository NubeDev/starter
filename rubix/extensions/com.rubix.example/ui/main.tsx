// `ui/main.tsx` — UI contribution for `com.rubix.example`.
//
// The developer-facing source for the panel the host renders into
// `<ExtensionSlot id="main">` on the rubix-frontend `/extensions`
// route. The runtime bundle lives next to this file at
// `ui/remoteEntry.js` — that file is what the host actually loads;
// `main.tsx` is the human-readable source the future
// vite-plugin-federation build (SCOPE Phase E) will compile into
// the bundle.
//
// What the panel does:
//
//   1. Reads its own manifest via the same admin route the host
//      bootstrap loop uses (`GET <basePath>/<id>`). Proves the route
//      is reachable from inside an extension's UI context.
//   2. Renders extension metadata (id, version, state, enablement)
//      plus a contributions summary (tools, skills, flows, ui).
//   3. Offers a refresh button so the panel can re-fetch without a
//      full page reload — exercises the manager's component
//      lifecycle (state survives, host singletons unchanged).
//   4. Stamps host theme + slot context so a visual reviewer can
//      confirm theming tokens reach the extension surface.
//
// Uses only `@nube/starter-ext-sdk-ts` + React — no rubix-* imports,
// per SCOPE R8.

import * as React from "react";

import {
  BlockShell,
  useHostTheme,
  useSlotContext,
} from "@nube/starter-ext-sdk-ts";

const EXTENSION_ID = "com.rubix.example";

interface ExtensionDetail {
  id: string;
  enabled: string;
  state: string;
  manifest: {
    id?: string;
    version?: string;
    contributes?: {
      tools?: ReadonlyArray<{ id: string }>;
      skills?: ReadonlyArray<{ dir: string }>;
      flows?: ReadonlyArray<{ id: string }>;
      ui?: { entry: string; exposes?: ReadonlyArray<{ slot: string }> };
    };
  } | null;
}

export default function Main(): React.ReactElement {
  return (
    <BlockShell>
      <MainInner />
    </BlockShell>
  );
}

function MainInner(): React.ReactElement {
  const slot = useSlotContext();
  const theme = useHostTheme();
  const [detail, setDetail] = React.useState<ExtensionDetail | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(false);
  const [tick, setTick] = React.useState(0);

  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetch(`/api/v1/extensions/${EXTENSION_ID}`, {
      credentials: "same-origin",
      headers: { accept: "application/json" },
    })
      .then(async (res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return (await res.json()) as ExtensionDetail;
      })
      .then((d) => {
        if (!cancelled) setDetail(d);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [tick]);

  const c = detail?.manifest?.contributes ?? {};
  const tools = c.tools ?? [];
  const skills = c.skills ?? [];
  const flows = c.flows ?? [];
  const exposes = c.ui?.exposes ?? [];

  return (
    <section
      data-ext-id={EXTENSION_ID}
      data-ext-slot={slot.slotId}
      data-ext-theme={theme.mode}
      style={{
        padding: "1rem 1.25rem",
        borderRadius: "0.75rem",
        border: "1px solid var(--color-border, rgba(0,0,0,0.1))",
        background: "var(--color-surface, transparent)",
        color: "var(--color-foreground, inherit)",
        display: "flex",
        flexDirection: "column",
        gap: "0.75rem",
      }}
    >
      <header
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "1rem",
        }}
      >
        <div>
          <h3 style={{ margin: 0, fontSize: "1rem" }}>
            {EXTENSION_ID}
            {detail?.manifest?.version ? (
              <span style={{ opacity: 0.6, fontWeight: 400 }}>
                {" "}
                v{detail.manifest.version}
              </span>
            ) : null}
          </h3>
          <small style={{ opacity: 0.7 }}>
            slot=<code>{slot.slotId}</code> · theme=<code>{theme.mode}</code>
            {detail ? (
              <>
                {" "}
                · state=<code>{detail.state}</code> · enabled=
                <code>{detail.enabled}</code>
              </>
            ) : null}
          </small>
        </div>
        <button
          type="button"
          onClick={() => setTick((t) => t + 1)}
          disabled={loading}
          style={{
            padding: "0.35rem 0.75rem",
            borderRadius: "0.375rem",
            border: "1px solid var(--color-border, rgba(0,0,0,0.15))",
            background: "transparent",
            color: "inherit",
            cursor: loading ? "wait" : "pointer",
            font: "inherit",
          }}
        >
          {loading ? "loading…" : "refresh"}
        </button>
      </header>

      {error ? (
        <p
          role="alert"
          style={{
            margin: 0,
            padding: "0.5rem 0.75rem",
            borderRadius: "0.375rem",
            background: "var(--color-danger-surface, rgba(220,38,38,0.08))",
            color: "var(--color-danger, rgb(185,28,28))",
            fontSize: "0.875rem",
          }}
        >
          failed to load manifest: {error}
        </p>
      ) : null}

      <dl
        style={{
          margin: 0,
          display: "grid",
          gridTemplateColumns: "max-content 1fr",
          gap: "0.25rem 0.75rem",
          fontSize: "0.875rem",
        }}
      >
        <ContribRow label="tools" items={tools.map((t) => t.id)} />
        <ContribRow label="skills" items={skills.map((s) => s.dir)} />
        <ContribRow label="flows" items={flows.map((f) => f.id)} />
        <ContribRow label="ui slots" items={exposes.map((e) => e.slot)} />
      </dl>
    </section>
  );
}

function ContribRow({
  label,
  items,
}: {
  label: string;
  items: ReadonlyArray<string>;
}): React.ReactElement {
  return (
    <>
      <dt style={{ opacity: 0.7 }}>{label}</dt>
      <dd style={{ margin: 0 }}>
        {items.length === 0 ? (
          <span style={{ opacity: 0.5 }}>—</span>
        ) : (
          items.map((id, i) => (
            <React.Fragment key={id}>
              {i > 0 ? ", " : ""}
              <code>{id}</code>
            </React.Fragment>
          ))
        )}
      </dd>
    </>
  );
}
