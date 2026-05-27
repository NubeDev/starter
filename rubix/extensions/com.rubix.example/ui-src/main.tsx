// `ui/main.tsx` — developer-facing source for the UI panel served by
// `com.rubix.example` at `<ExtensionSlot id="main">` on the
// rubix-frontend `/extensions` route.

import * as React from "react";

import {
  BlockShell,
  useExtensionRoute,
  useHostTheme,
  useSlotContext,
} from "@nube/starter-ext-sdk-ts";

import { EXTENSION_ID } from "./types";
import type { ExtensionDetail } from "./types";
import { SAMPLE_CUSTOMERS, SAMPLE_PRODUCTS } from "./data";
import { evaluateCustomerQuality } from "./quality";
import { Card, ContribRow, CountryBarChart } from "./components";

export default function Main(): React.ReactElement {
  return (
    <BlockShell>
      <MainRouter />
    </BlockShell>
  );
}

/** Sub-route switch for `com.rubix.example`. The host's
 * `/extensions/$extId/$rest` route hands us `route` via
 * `useExtensionRoute()` — null means we're mounted in a non-route
 * slot (admin index page), an empty string means the per-extension
 * index, anything else is a deep link from the sidebar nav-tree. */
function MainRouter(): React.ReactElement {
  const route = useExtensionRoute();
  if (route === "customers/by-country") return <SubView title="Customers by country" route={route} />;
  if (route === "customers/quality") return <SubView title="Customer quality issues" route={route} />;
  if (route === "products/low-stock") return <SubView title="Low-stock products" route={route} />;
  if (route === "products/catalog") return <SubView title="Product catalog" route={route} />;
  return <MainInner />;
}

function SubView({ title, route }: { title: string; route: string }): React.ReactElement {
  return (
    <section style={{ padding: "1rem", color: "var(--color-foreground, inherit)" }}>
      <h1 style={{ fontSize: "1.25rem", fontWeight: 600, marginBottom: "0.5rem" }}>{title}</h1>
      <p style={{ opacity: 0.7, fontSize: "0.875rem" }}>
        Sub-route: <code>{route}</code>
      </p>
      <p style={{ marginTop: "1rem", fontSize: "0.875rem" }}>
        Placeholder view. Replace with charts/tables specific to this nav item.
      </p>
    </section>
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
  const tools = (c.tools ?? []).map((t) => t.id);
  const warehouseTables = (c.warehouse_tables ?? []).map((t) => t.name);
  const warehouseTemplates = (c.warehouse_templates ?? []).map((t) => t.name);
  const anomalyRules = (c.anomaly_rules ?? []).map((r) => r.id);
  const exposes = (c.ui?.exposes ?? []).map((e) => e.slot);

  const flagged = React.useMemo(
    () =>
      SAMPLE_CUSTOMERS.map((r) => ({ r, q: evaluateCustomerQuality(r) })).filter(
        (x) => x.q.outcome !== "ok",
      ),
    [],
  );

  const lowStock = React.useMemo(
    () => SAMPLE_PRODUCTS.filter((p) => p.stock < 10).slice().sort((a, b) => a.stock - b.stock),
    [],
  );

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
        gap: "1rem",
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
          <h3 style={{ margin: 0, fontSize: "1.05rem" }}>
            {EXTENSION_ID}
            {detail?.manifest?.version ? (
              <span style={{ opacity: 0.6, fontWeight: 400 }}>
                {" "}
                v{detail.manifest.version}
              </span>
            ) : null}
          </h3>
          <small style={{ opacity: 0.7 }}>
            datablist sample data · warehouse + anomaly-rule demo
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
          fontSize: "0.85rem",
        }}
      >
        <ContribRow label="tools" items={tools} />
        <ContribRow label="warehouse tables" items={warehouseTables} />
        <ContribRow label="warehouse templates" items={warehouseTemplates} />
        <ContribRow label="anomaly rules" items={anomalyRules} />
        <ContribRow label="ui slots" items={exposes} />
      </dl>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))",
          gap: "0.75rem",
        }}
      >
        <Card
          title="Customers by country (top 10)"
          subtitle="preview of com.rubix.example.customers_by_country template"
        >
          <CountryBarChart rows={SAMPLE_CUSTOMERS} />
        </Card>

        <Card
          title="Low-stock products (< 10)"
          subtitle="preview of com.rubix.example.products_low_stock template"
        >
          <table
            style={{
              width: "100%",
              borderCollapse: "collapse",
              fontSize: "0.8rem",
            }}
          >
            <thead>
              <tr style={{ opacity: 0.7, textAlign: "left" }}>
                <th style={{ padding: "0.2rem 0.3rem" }}>SKU</th>
                <th style={{ padding: "0.2rem 0.3rem" }}>Name</th>
                <th style={{ padding: "0.2rem 0.3rem", textAlign: "right" }}>Stock</th>
                <th style={{ padding: "0.2rem 0.3rem", textAlign: "right" }}>Price</th>
              </tr>
            </thead>
            <tbody>
              {lowStock.map((p) => (
                <tr
                  key={p.internal_id}
                  style={{ borderTop: "1px solid var(--color-border, rgba(0,0,0,0.08))" }}
                >
                  <td style={{ padding: "0.2rem 0.3rem" }}>
                    <code>{p.internal_id}</code>
                  </td>
                  <td style={{ padding: "0.2rem 0.3rem" }}>{p.name}</td>
                  <td
                    style={{
                      padding: "0.2rem 0.3rem",
                      textAlign: "right",
                      color:
                        p.stock === 0
                          ? "var(--color-danger, rgb(185,28,28))"
                          : "inherit",
                      fontWeight: p.stock === 0 ? 600 : 400,
                    }}
                  >
                    {p.stock}
                  </td>
                  <td style={{ padding: "0.2rem 0.3rem", textAlign: "right" }}>
                    ${p.price.toFixed(2)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      </div>

      <Card
        title={`Data-quality rule preview · ${flagged.length} / ${SAMPLE_CUSTOMERS.length} flagged`}
        subtitle="client-side mirror of com.rubix.example.customer_quality (server-side rule lives in process/src/main.rs)"
      >
        {flagged.length === 0 ? (
          <em style={{ opacity: 0.6 }}>no flags — sample data is clean</em>
        ) : (
          <ul style={{ margin: 0, paddingLeft: "1.1rem", fontSize: "0.85rem" }}>
            {flagged.map(({ r, q }) => (
              <li key={r.customer_id}>
                <code
                  style={{
                    marginRight: "0.4rem",
                    padding: "0 0.3rem",
                    borderRadius: "0.25rem",
                    background: "var(--color-warning-surface, rgba(217,119,6,0.12))",
                    color: "var(--color-warning, rgb(180,83,9))",
                    fontSize: "0.75rem",
                  }}
                >
                  {q.quality}
                </code>
                <code>{r.customer_id}</code>
                {" · "}
                <span style={{ opacity: 0.75 }}>{q.note || ""}</span>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </section>
  );
}
