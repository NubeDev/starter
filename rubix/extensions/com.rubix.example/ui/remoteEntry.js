import { jsx, jsxs, Fragment } from 'react/jsx-runtime';
import * as React from 'react';

const HOST_CLIENT_CTX_KEY = "__starterExtSdkHostClientContextV1";
globalThis[HOST_CLIENT_CTX_KEY] ?? (globalThis[HOST_CLIENT_CTX_KEY] = React.createContext(null));

const SLOT_CTX_KEY = "__starterExtSdkSlotContextV2";
const Context = globalThis[SLOT_CTX_KEY] ?? (globalThis[SLOT_CTX_KEY] = React.createContext(null));
function useSlotContext() {
  const ctx = React.useContext(Context);
  if (!ctx) {
    throw new Error(
      "useSlotContext() called outside <SlotContextProvider>. The host's federation runtime must wrap exposed components in SlotContextProvider."
    );
  }
  return ctx;
}
function useExtensionRoute() {
  return useSlotContext().route;
}

const DEFAULT_BLOCK_SHELL_MESSAGES = {
  loading: "Loading…",
  errorTitle: "Extension failed:"
};
function mergeBlockShellMessages(override) {
  return override ? { ...DEFAULT_BLOCK_SHELL_MESSAGES, ...override } : DEFAULT_BLOCK_SHELL_MESSAGES;
}
function BlockShell(props) {
  const slot = useSlotContext();
  const messages = React.useMemo(
    () => mergeBlockShellMessages(props.messages),
    [props.messages]
  );
  return /* @__PURE__ */ jsx(
    "div",
    {
      className: props.className ? `starter-ext-block ${props.className}` : "starter-ext-block",
      "data-ext-id": slot.extensionId,
      "data-ext-slot": slot.slotId,
      children: /* @__PURE__ */ jsx(
        ExtensionErrorBoundary,
        {
          extensionId: slot.extensionId,
          fallback: props.errorFallback,
          errorTitle: messages.errorTitle,
          children: /* @__PURE__ */ jsx(
            React.Suspense,
            {
              fallback: props.loading ?? /* @__PURE__ */ jsx(DefaultLoading, { slotId: slot.slotId, label: messages.loading }),
              children: props.children
            }
          )
        }
      )
    }
  );
}
class ExtensionErrorBoundary extends React.Component {
  state = { error: null };
  static getDerivedStateFromError(error) {
    return { error };
  }
  componentDidCatch(error, info) {
    console.error(
      `[starter-ext] extension ${this.props.extensionId} crashed in render:`,
      error,
      info
    );
  }
  render() {
    if (this.state.error !== null) {
      const fb = this.props.fallback;
      if (fb) {
        return fb(this.state.error, this.props.extensionId);
      }
      return defaultErrorFallback(
        this.state.error,
        this.props.extensionId,
        this.props.errorTitle
      );
    }
    return this.props.children;
  }
}
function defaultErrorFallback(err, extensionId, title) {
  const msg = err instanceof Error ? err.message : String(err);
  return /* @__PURE__ */ jsxs("div", { role: "alert", className: "starter-ext-block__error", children: [
    /* @__PURE__ */ jsx("strong", { children: title }),
    " ",
    extensionId,
    /* @__PURE__ */ jsx("div", { children: msg })
  ] });
}
function DefaultLoading(props) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      "aria-busy": "true",
      "aria-live": "polite",
      className: "starter-ext-block__loading",
      "data-slot": props.slotId,
      children: props.label
    }
  );
}

function useHostTheme() {
  const slot = useSlotContext();
  return React.useMemo(
    () => ({
      mode: slot.theme,
      tokens: slot.themeTokens,
      token(key) {
        const fromMap = slot.themeTokens?.[key];
        if (fromMap) return fromMap;
        if (typeof window === "undefined") return "";
        const styles = window.getComputedStyle(document.documentElement);
        return styles.getPropertyValue(`--${key}`).trim();
      }
    }),
    [slot.theme, slot.themeTokens]
  );
}

const HOST_BINDINGS_CTX_KEY = "__starterExtSdkHostBindingsContextV1";
const HostBindingsContext = globalThis[HOST_BINDINGS_CTX_KEY] ?? (globalThis[HOST_BINDINGS_CTX_KEY] = React.createContext(null));
function HostBindingsProvider(props) {
  return /* @__PURE__ */ jsx(HostBindingsContext.Provider, { value: props.bindings, children: props.children });
}

function registerExtensionContributions(handle, contributions) {
  const bindings = { extensionId: handle.id, singletons: handle.singletons };
  const wrapped = {};
  for (const [name, Component] of Object.entries(contributions.components)) {
    wrapped[name] = wrapWithBindings(name, Component, bindings);
  }
  handle.register({ components: wrapped });
}
function wrapWithBindings(displayName, Component, bindings) {
  const Wrapped = (props) => /* @__PURE__ */ jsx(HostBindingsProvider, { bindings, children: /* @__PURE__ */ jsx(Component, { ...props }) });
  Wrapped.displayName = `HostBindings(${bindings.extensionId}:${displayName})`;
  return Wrapped;
}

const EXTENSION_ID = "com.rubix.example";

const SAMPLE_CUSTOMERS = [
  { customer_id: "DD37Cf93aecA6Dc", first_name: "Sheryl", country: "Chile", email: "zunigavanessa@smith.info", subscription_date: "2020-08-24" },
  { customer_id: "1Ef7b82A4CAAD10", first_name: "Preston", country: "Djibouti", email: "vmata@colon.com", subscription_date: "2021-04-23" },
  { customer_id: "5Cef8BFA16c5e3c", first_name: "Linda", country: "Dominican Republic", email: "stanleyblackwell@benson.org", subscription_date: "2020-06-02" },
  { customer_id: "053d585Ab6b3159", first_name: "Joanna", country: "Slovakia", email: "colinalvarado@miles.net", subscription_date: "2021-04-17" },
  { customer_id: "EA4d384DfDbBf77", first_name: "Darren", country: "Pitcairn Islands", email: "tgates@cantrell.com", subscription_date: "2021-08-24" },
  { customer_id: "C2dE4dEEc489ae0", first_name: "Sheryl", country: "Cyprus", email: "mariokhan@ryan-pope.org", subscription_date: "2020-01-13" },
  { customer_id: "8C2811a503C7c5a", first_name: "Michelle", country: "Timor-Leste", email: "mdyer@escobar.net", subscription_date: "2021-11-08" },
  { customer_id: "CEDec94deE6d69B", first_name: "Jenna", country: "Vietnam", email: "mark42@robbins.com", subscription_date: "2020-11-29" },
  { customer_id: "FFf18C760aA5b27", first_name: "Maxwell", country: "Malta", email: "ehyde@brewer.biz", subscription_date: "2020-12-19" },
  { customer_id: "BAD-NO-EMAIL-01", first_name: "Casey", country: "Chile", email: "", subscription_date: "2021-06-01" },
  { customer_id: "BAD-NO-CNTRY-01", first_name: "Riley", country: "", email: "riley@example.com", subscription_date: "2021-06-02" },
  { customer_id: "BAD-DATE-001", first_name: "Morgan", country: "Slovakia", email: "morgan@example.com", subscription_date: "1899-99-99" }
];
const SAMPLE_PRODUCTS = [
  { internal_id: "SKU-0001", name: "Slim-Fit Cotton Tee", brand: "Acme Apparel", category: "Clothing", price: 19.99, stock: 420, availability: "in_stock" },
  { internal_id: "SKU-0002", name: "Wireless Bluetooth Earbuds", brand: "SoundOrbit", category: "Electronics", price: 79, stock: 12, availability: "low_stock" },
  { internal_id: "SKU-0003", name: "Stainless Travel Mug", brand: "Hearthware", category: "Kitchen", price: 24.5, stock: 0, availability: "out_of_stock" },
  { internal_id: "SKU-0005", name: "Trail Running Shoes", brand: "Switchback", category: "Footwear", price: 129, stock: 3, availability: "low_stock" },
  { internal_id: "SKU-0008", name: "Insulated Water Bottle", brand: "Cascade", category: "Outdoor", price: 29.99, stock: 2, availability: "low_stock" },
  { internal_id: "SKU-0010", name: "Cast-Iron Skillet", brand: "Hearthware", category: "Kitchen", price: 44, stock: 0, availability: "out_of_stock" },
  { internal_id: "SKU-0013", name: "Espresso Tamper", brand: "Caffeo", category: "Kitchen", price: 55, stock: 7, availability: "low_stock" },
  { internal_id: "SKU-0015", name: "USB-C Power Bank", brand: "Voltcell", category: "Electronics", price: 89.99, stock: 0, availability: "out_of_stock" },
  { internal_id: "SKU-0016", name: "Down Camp Quilt", brand: "Switchback", category: "Outdoor", price: 189, stock: 9, availability: "low_stock" },
  { internal_id: "SKU-0019", name: "Trail Running Vest", brand: "Switchback", category: "Fitness", price: 99.5, stock: 4, availability: "low_stock" }
];

function evaluateCustomerQuality(row) {
  const country = (row.country || "").trim();
  if (!country) {
    return {
      outcome: "flag",
      quality: "MissingCountry",
      note: `customer_id=${row.customer_id || "<unknown>"}`
    };
  }
  const email = (row.email || "").trim();
  if (!email) {
    return {
      outcome: "flag",
      quality: "MissingEmail",
      note: `customer_id=${row.customer_id || "<unknown>"}`
    };
  }
  if (!email.includes("@")) {
    return { outcome: "flag", quality: "InvalidEmail", note: `email=${email}` };
  }
  const d = row.subscription_date;
  if (d && !/^(20\d{2}|2100)-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$/.test(d)) {
    return { outcome: "flag", quality: "BadDate", note: `subscription_date=${d}` };
  }
  return { outcome: "ok" };
}

function ContribRow({
  label,
  items
}) {
  return /* @__PURE__ */ jsxs(Fragment, { children: [
    /* @__PURE__ */ jsx("dt", { style: { opacity: 0.7 }, children: label }),
    /* @__PURE__ */ jsx("dd", { style: { margin: 0 }, children: items.length === 0 ? /* @__PURE__ */ jsx("span", { style: { opacity: 0.5 }, children: "—" }) : items.map((id, i) => /* @__PURE__ */ jsxs(React.Fragment, { children: [
      i > 0 ? ", " : "",
      /* @__PURE__ */ jsx("code", { children: id })
    ] }, id + i)) })
  ] });
}
function Card({
  title,
  subtitle,
  children
}) {
  return /* @__PURE__ */ jsxs(
    "section",
    {
      style: {
        padding: "0.9rem 1rem",
        borderRadius: "0.6rem",
        border: "1px solid var(--color-border, rgba(0,0,0,0.1))",
        background: "var(--color-surface, transparent)",
        display: "flex",
        flexDirection: "column",
        gap: "0.5rem"
      },
      children: [
        /* @__PURE__ */ jsxs("header", { children: [
          /* @__PURE__ */ jsx("h4", { style: { margin: 0, fontSize: "0.95rem" }, children: title }),
          subtitle ? /* @__PURE__ */ jsx("small", { style: { opacity: 0.65 }, children: subtitle }) : null
        ] }),
        children
      ]
    }
  );
}
function CountryBarChart({
  rows
}) {
  const buckets = React.useMemo(() => {
    const m = /* @__PURE__ */ new Map();
    for (const r of rows) {
      const q = evaluateCustomerQuality(r);
      if (q.outcome !== "ok" && q.quality === "MissingCountry") continue;
      const c = (r.country || "").trim() || "(unknown)";
      m.set(c, (m.get(c) || 0) + 1);
    }
    return [...m.entries()].map(([country, count]) => ({ country, count })).sort((a, b) => b.count - a.count).slice(0, 10);
  }, [rows]);
  if (buckets.length === 0) return null;
  const max = Math.max(...buckets.map((b) => b.count));
  const labelW = 160;
  const barH = 18;
  const gap = 6;
  const w = 360;
  const height = buckets.length * (barH + gap);
  return /* @__PURE__ */ jsx(
    "svg",
    {
      width: "100%",
      viewBox: `0 0 ${labelW + w + 40} ${height}`,
      role: "img",
      "aria-label": "Customers by country (top 10)",
      style: { display: "block" },
      children: buckets.map((b, i) => {
        const y = i * (barH + gap);
        const bw = Math.max(1, Math.round(b.count / max * w));
        return /* @__PURE__ */ jsxs(React.Fragment, { children: [
          /* @__PURE__ */ jsx(
            "text",
            {
              x: labelW - 8,
              y: y + barH * 0.72,
              textAnchor: "end",
              fontSize: 12,
              fill: "currentColor",
              opacity: 0.85,
              children: b.country
            }
          ),
          /* @__PURE__ */ jsx(
            "rect",
            {
              x: labelW,
              y,
              width: bw,
              height: barH,
              rx: 3,
              fill: "var(--color-accent, #4f46e5)",
              opacity: 0.85
            }
          ),
          /* @__PURE__ */ jsx(
            "text",
            {
              x: labelW + bw + 6,
              y: y + barH * 0.72,
              fontSize: 12,
              fill: "currentColor",
              opacity: 0.85,
              children: b.count
            }
          )
        ] }, b.country);
      })
    }
  );
}

function Main() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(MainRouter, {}) });
}
function MainRouter() {
  const route = useExtensionRoute();
  if (route === "customers/by-country") return /* @__PURE__ */ jsx(SubView, { title: "Customers by country", route });
  if (route === "customers/quality") return /* @__PURE__ */ jsx(SubView, { title: "Customer quality issues", route });
  if (route === "products/low-stock") return /* @__PURE__ */ jsx(SubView, { title: "Low-stock products", route });
  if (route === "products/catalog") return /* @__PURE__ */ jsx(SubView, { title: "Product catalog", route });
  return /* @__PURE__ */ jsx(MainInner, {});
}
function SubView({ title, route }) {
  return /* @__PURE__ */ jsxs("section", { style: { padding: "1rem", color: "var(--color-foreground, inherit)" }, children: [
    /* @__PURE__ */ jsx("h1", { style: { fontSize: "1.25rem", fontWeight: 600, marginBottom: "0.5rem" }, children: title }),
    /* @__PURE__ */ jsxs("p", { style: { opacity: 0.7, fontSize: "0.875rem" }, children: [
      "Sub-route: ",
      /* @__PURE__ */ jsx("code", { children: route })
    ] }),
    /* @__PURE__ */ jsx("p", { style: { marginTop: "1rem", fontSize: "0.875rem" }, children: "Placeholder view. Replace with charts/tables specific to this nav item." })
  ] });
}
function MainInner() {
  const slot = useSlotContext();
  const theme = useHostTheme();
  const [detail, setDetail] = React.useState(null);
  const [error, setError] = React.useState(null);
  const [loading, setLoading] = React.useState(false);
  const [tick, setTick] = React.useState(0);
  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetch(`/api/v1/extensions/${EXTENSION_ID}`, {
      credentials: "same-origin",
      headers: { accept: "application/json" }
    }).then(async (res) => {
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return await res.json();
    }).then((d) => {
      if (!cancelled) setDetail(d);
    }).catch((e) => {
      if (!cancelled) setError(e instanceof Error ? e.message : String(e));
    }).finally(() => {
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
    () => SAMPLE_CUSTOMERS.map((r) => ({ r, q: evaluateCustomerQuality(r) })).filter(
      (x) => x.q.outcome !== "ok"
    ),
    []
  );
  const lowStock = React.useMemo(
    () => SAMPLE_PRODUCTS.filter((p) => p.stock < 10).slice().sort((a, b) => a.stock - b.stock),
    []
  );
  return /* @__PURE__ */ jsxs(
    "section",
    {
      "data-ext-id": EXTENSION_ID,
      "data-ext-slot": slot.slotId,
      "data-ext-theme": theme.mode,
      style: {
        padding: "1rem 1.25rem",
        borderRadius: "0.75rem",
        border: "1px solid var(--color-border, rgba(0,0,0,0.1))",
        background: "var(--color-surface, transparent)",
        color: "var(--color-foreground, inherit)",
        display: "flex",
        flexDirection: "column",
        gap: "1rem"
      },
      children: [
        /* @__PURE__ */ jsxs(
          "header",
          {
            style: {
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              gap: "1rem"
            },
            children: [
              /* @__PURE__ */ jsxs("div", { children: [
                /* @__PURE__ */ jsxs("h3", { style: { margin: 0, fontSize: "1.05rem" }, children: [
                  EXTENSION_ID,
                  detail?.manifest?.version ? /* @__PURE__ */ jsxs("span", { style: { opacity: 0.6, fontWeight: 400 }, children: [
                    " ",
                    "v",
                    detail.manifest.version
                  ] }) : null
                ] }),
                /* @__PURE__ */ jsxs("small", { style: { opacity: 0.7 }, children: [
                  "datablist sample data · warehouse + anomaly-rule demo",
                  detail ? /* @__PURE__ */ jsxs(Fragment, { children: [
                    " ",
                    "· state=",
                    /* @__PURE__ */ jsx("code", { children: detail.state }),
                    " · enabled=",
                    /* @__PURE__ */ jsx("code", { children: detail.enabled })
                  ] }) : null
                ] })
              ] }),
              /* @__PURE__ */ jsx(
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
                    font: "inherit"
                  },
                  children: loading ? "loading…" : "refresh"
                }
              )
            ]
          }
        ),
        error ? /* @__PURE__ */ jsxs(
          "p",
          {
            role: "alert",
            style: {
              margin: 0,
              padding: "0.5rem 0.75rem",
              borderRadius: "0.375rem",
              background: "var(--color-danger-surface, rgba(220,38,38,0.08))",
              color: "var(--color-danger, rgb(185,28,28))",
              fontSize: "0.875rem"
            },
            children: [
              "failed to load manifest: ",
              error
            ]
          }
        ) : null,
        /* @__PURE__ */ jsxs(
          "dl",
          {
            style: {
              margin: 0,
              display: "grid",
              gridTemplateColumns: "max-content 1fr",
              gap: "0.25rem 0.75rem",
              fontSize: "0.85rem"
            },
            children: [
              /* @__PURE__ */ jsx(ContribRow, { label: "tools", items: tools }),
              /* @__PURE__ */ jsx(ContribRow, { label: "warehouse tables", items: warehouseTables }),
              /* @__PURE__ */ jsx(ContribRow, { label: "warehouse templates", items: warehouseTemplates }),
              /* @__PURE__ */ jsx(ContribRow, { label: "anomaly rules", items: anomalyRules }),
              /* @__PURE__ */ jsx(ContribRow, { label: "ui slots", items: exposes })
            ]
          }
        ),
        /* @__PURE__ */ jsxs(
          "div",
          {
            style: {
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))",
              gap: "0.75rem"
            },
            children: [
              /* @__PURE__ */ jsx(
                Card,
                {
                  title: "Customers by country (top 10)",
                  subtitle: "preview of com.rubix.example.customers_by_country template",
                  children: /* @__PURE__ */ jsx(CountryBarChart, { rows: SAMPLE_CUSTOMERS })
                }
              ),
              /* @__PURE__ */ jsx(
                Card,
                {
                  title: "Low-stock products (< 10)",
                  subtitle: "preview of com.rubix.example.products_low_stock template",
                  children: /* @__PURE__ */ jsxs(
                    "table",
                    {
                      style: {
                        width: "100%",
                        borderCollapse: "collapse",
                        fontSize: "0.8rem"
                      },
                      children: [
                        /* @__PURE__ */ jsx("thead", { children: /* @__PURE__ */ jsxs("tr", { style: { opacity: 0.7, textAlign: "left" }, children: [
                          /* @__PURE__ */ jsx("th", { style: { padding: "0.2rem 0.3rem" }, children: "SKU" }),
                          /* @__PURE__ */ jsx("th", { style: { padding: "0.2rem 0.3rem" }, children: "Name" }),
                          /* @__PURE__ */ jsx("th", { style: { padding: "0.2rem 0.3rem", textAlign: "right" }, children: "Stock" }),
                          /* @__PURE__ */ jsx("th", { style: { padding: "0.2rem 0.3rem", textAlign: "right" }, children: "Price" })
                        ] }) }),
                        /* @__PURE__ */ jsx("tbody", { children: lowStock.map((p) => /* @__PURE__ */ jsxs(
                          "tr",
                          {
                            style: { borderTop: "1px solid var(--color-border, rgba(0,0,0,0.08))" },
                            children: [
                              /* @__PURE__ */ jsx("td", { style: { padding: "0.2rem 0.3rem" }, children: /* @__PURE__ */ jsx("code", { children: p.internal_id }) }),
                              /* @__PURE__ */ jsx("td", { style: { padding: "0.2rem 0.3rem" }, children: p.name }),
                              /* @__PURE__ */ jsx(
                                "td",
                                {
                                  style: {
                                    padding: "0.2rem 0.3rem",
                                    textAlign: "right",
                                    color: p.stock === 0 ? "var(--color-danger, rgb(185,28,28))" : "inherit",
                                    fontWeight: p.stock === 0 ? 600 : 400
                                  },
                                  children: p.stock
                                }
                              ),
                              /* @__PURE__ */ jsxs("td", { style: { padding: "0.2rem 0.3rem", textAlign: "right" }, children: [
                                "$",
                                p.price.toFixed(2)
                              ] })
                            ]
                          },
                          p.internal_id
                        )) })
                      ]
                    }
                  )
                }
              )
            ]
          }
        ),
        /* @__PURE__ */ jsx(
          Card,
          {
            title: `Data-quality rule preview · ${flagged.length} / ${SAMPLE_CUSTOMERS.length} flagged`,
            subtitle: "client-side mirror of com.rubix.example.customer_quality (server-side rule lives in process/src/main.rs)",
            children: flagged.length === 0 ? /* @__PURE__ */ jsx("em", { style: { opacity: 0.6 }, children: "no flags — sample data is clean" }) : /* @__PURE__ */ jsx("ul", { style: { margin: 0, paddingLeft: "1.1rem", fontSize: "0.85rem" }, children: flagged.map(({ r, q }) => /* @__PURE__ */ jsxs("li", { children: [
              /* @__PURE__ */ jsx(
                "code",
                {
                  style: {
                    marginRight: "0.4rem",
                    padding: "0 0.3rem",
                    borderRadius: "0.25rem",
                    background: "var(--color-warning-surface, rgba(217,119,6,0.12))",
                    color: "var(--color-warning, rgb(180,83,9))",
                    fontSize: "0.75rem"
                  },
                  children: q.quality
                }
              ),
              /* @__PURE__ */ jsx("code", { children: r.customer_id }),
              " · ",
              /* @__PURE__ */ jsx("span", { style: { opacity: 0.75 }, children: q.note || "" })
            ] }, r.customer_id)) })
          }
        )
      ]
    }
  );
}

const TREE = [
  {
    title: "Customers",
    children: [
      { title: "By country", href: `/extensions/${EXTENSION_ID}/customers/by-country` },
      { title: "Quality issues", href: `/extensions/${EXTENSION_ID}/customers/quality` }
    ]
  },
  {
    title: "Products",
    children: [
      { title: "Low stock", href: `/extensions/${EXTENSION_ID}/products/low-stock` },
      { title: "Catalog", href: `/extensions/${EXTENSION_ID}/products/catalog` }
    ]
  }
];
function NavTree() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(NavTreeInner, {}) });
}
function NavTreeInner() {
  return /* @__PURE__ */ jsxs(
    "nav",
    {
      "aria-label": "Rubix Example",
      style: {
        margin: "0.25rem 0.5rem",
        fontSize: "0.8125rem",
        color: "var(--color-foreground, inherit)"
      },
      children: [
        /* @__PURE__ */ jsx(
          "div",
          {
            style: {
              padding: "0.25rem 0.5rem",
              fontSize: "0.7rem",
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.04em",
              opacity: 0.6
            },
            children: "Rubix Example"
          }
        ),
        /* @__PURE__ */ jsx("ul", { style: { margin: 0, padding: 0, listStyle: "none" }, children: TREE.map((branch) => /* @__PURE__ */ jsx(Branch, { branch }, branch.title)) })
      ]
    }
  );
}
function Branch({ branch }) {
  const [open, setOpen] = React.useState(true);
  return /* @__PURE__ */ jsxs("li", { children: [
    /* @__PURE__ */ jsxs(
      "button",
      {
        type: "button",
        onClick: () => setOpen((v) => !v),
        "aria-expanded": open,
        style: {
          width: "100%",
          display: "flex",
          alignItems: "center",
          gap: "0.4rem",
          padding: "0.3rem 0.5rem",
          background: "transparent",
          border: 0,
          color: "inherit",
          font: "inherit",
          cursor: "pointer",
          borderRadius: "0.375rem",
          textAlign: "left"
        },
        children: [
          /* @__PURE__ */ jsx(Chevron, { open }),
          /* @__PURE__ */ jsx("span", { children: branch.title })
        ]
      }
    ),
    open ? /* @__PURE__ */ jsx(
      "ul",
      {
        style: {
          margin: 0,
          paddingInlineStart: "1.25rem",
          listStyle: "none",
          borderInlineStart: "1px solid var(--color-border, rgba(0,0,0,0.12))",
          marginInlineStart: "1rem"
        },
        children: branch.children.map((leaf) => /* @__PURE__ */ jsx("li", { children: /* @__PURE__ */ jsx(
          "a",
          {
            href: leaf.href,
            style: {
              display: "block",
              padding: "0.25rem 0.5rem",
              textDecoration: "none",
              color: "inherit",
              borderRadius: "0.375rem",
              opacity: 0.85
            },
            children: leaf.title
          }
        ) }, leaf.href))
      }
    ) : null
  ] });
}
function Chevron({ open }) {
  return /* @__PURE__ */ jsx(
    "svg",
    {
      width: "10",
      height: "10",
      viewBox: "0 0 10 10",
      "aria-hidden": "true",
      style: {
        transition: "transform 120ms",
        transform: open ? "rotate(90deg)" : "rotate(0deg)",
        flexShrink: 0,
        opacity: 0.7
      },
      children: /* @__PURE__ */ jsx("path", { d: "M3 1.5 L7 5 L3 8.5", stroke: "currentColor", strokeWidth: "1.4", fill: "none", strokeLinecap: "round", strokeLinejoin: "round" })
    }
  );
}

function Sidebar() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(SidebarInner, {}) });
}
function SidebarInner() {
  const slot = useSlotContext();
  const [detail, setDetail] = React.useState(null);
  const [error, setError] = React.useState(null);
  React.useEffect(() => {
    let cancelled = false;
    fetch(`/api/v1/extensions/${EXTENSION_ID}`, {
      credentials: "same-origin",
      headers: { accept: "application/json" }
    }).then(async (res) => {
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return await res.json();
    }).then((d) => {
      if (!cancelled) setDetail(d);
    }).catch((e) => {
      if (!cancelled) setError(e instanceof Error ? e.message : String(e));
    });
    return () => {
      cancelled = true;
    };
  }, []);
  const c = detail?.manifest?.contributes ?? {};
  const toolCount = (c.tools ?? []).length;
  const tableCount = (c.warehouse_tables ?? []).length;
  const ruleCount = (c.anomaly_rules ?? []).length;
  const version = detail?.manifest?.version ?? null;
  return /* @__PURE__ */ jsxs(
    "section",
    {
      "data-ext-id": EXTENSION_ID,
      "data-ext-slot": slot.slotId,
      style: {
        margin: "0.25rem 0.5rem",
        padding: "0.5rem 0.625rem",
        borderRadius: "0.5rem",
        border: "1px solid var(--color-border, rgba(0,0,0,0.1))",
        background: "var(--color-surface, transparent)",
        color: "var(--color-foreground, inherit)",
        display: "flex",
        flexDirection: "column",
        gap: "0.35rem",
        fontSize: "0.75rem"
      },
      children: [
        /* @__PURE__ */ jsxs(
          "header",
          {
            style: {
              display: "flex",
              alignItems: "baseline",
              justifyContent: "space-between",
              gap: "0.5rem"
            },
            children: [
              /* @__PURE__ */ jsx("strong", { style: { fontSize: "0.78rem" }, children: "Rubix Example" }),
              version ? /* @__PURE__ */ jsxs("span", { style: { opacity: 0.6 }, children: [
                "v",
                version
              ] }) : null
            ]
          }
        ),
        error ? /* @__PURE__ */ jsx("p", { role: "alert", style: { margin: 0, opacity: 0.8 }, children: error }) : /* @__PURE__ */ jsxs(
          "ul",
          {
            style: {
              margin: 0,
              padding: 0,
              listStyle: "none",
              display: "flex",
              flexWrap: "wrap",
              gap: "0.25rem"
            },
            children: [
              /* @__PURE__ */ jsx(Pill, { label: "tools", count: toolCount }),
              /* @__PURE__ */ jsx(Pill, { label: "tables", count: tableCount }),
              /* @__PURE__ */ jsx(Pill, { label: "rules", count: ruleCount })
            ]
          }
        ),
        /* @__PURE__ */ jsx(
          "a",
          {
            href: "/extensions",
            style: {
              alignSelf: "flex-start",
              fontSize: "0.72rem",
              textDecoration: "none",
              opacity: 0.85
            },
            children: "open full panel →"
          }
        )
      ]
    }
  );
}
function Pill({
  label,
  count
}) {
  return /* @__PURE__ */ jsxs(
    "li",
    {
      style: {
        padding: "0.1rem 0.4rem",
        borderRadius: "999px",
        border: "1px solid var(--color-border, rgba(0,0,0,0.12))",
        opacity: count > 0 ? 1 : 0.5
      },
      children: [
        label,
        ": ",
        /* @__PURE__ */ jsx("strong", { children: count })
      ]
    }
  );
}

const factory = {
  // The host enforces matching-majors. Declare the React / ReactDOM
  // we authored against; the host will refuse to load this remote
  // if it ships a different React major.
  // Host (rubix-frontend) ships React 19. The host's singleton
  // gate compares majors only, so any 19.x works.
  singletons: {
    react: { version: "19.1.0" },
    "react-dom": { version: "19.1.0" }
  },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { Main, NavTree, Sidebar }
    });
  }
};

export { factory as default };
