import { jsx, jsxs } from 'react/jsx-runtime';
import * as React from 'react';

const HOST_CLIENT_CTX_KEY = "__starterExtSdkHostClientContextV1";
const HostClientContext = globalThis[HOST_CLIENT_CTX_KEY] ?? (globalThis[HOST_CLIENT_CTX_KEY] = React.createContext(null));
function useHostClient() {
  const client = React.useContext(HostClientContext);
  if (!client) {
    throw new Error(
      "useHostClient() called outside <ExtensionHostClientProvider>. The host shell must wrap extension slots in ExtensionHostProvider."
    );
  }
  return client;
}

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

class StarterError extends Error {
  status;
  problem;
  /**
   * Machine-readable error code. Set by client-side factories
   * (`invalidResponse` etc.) for cases the server cannot tag
   * itself. Server-driven errors carry their tag in `problem.type`.
   */
  code;
  constructor(status, message, problem, code) {
    super(message);
    this.name = "StarterError";
    this.status = status;
    this.problem = problem;
    this.code = code;
  }
  static async fromResponse(res) {
    let problem;
    try {
      const body = await res.clone().json();
      if (body && typeof body === "object" && "type" in body && "title" in body) {
        problem = body;
      }
    } catch {
    }
    let msg = problem?.title;
    if (!msg) {
      try {
        const text = (await res.clone().text()).trim();
        if (text) msg = text;
      } catch {
      }
    }
    return new StarterError(res.status, msg ?? `HTTP ${res.status}`, problem);
  }
  /**
   * Build an error for a 2xx response whose body is not JSON.
   * Typical cause: a dev-server SPA fallback returned `index.html`
   * instead of forwarding the request to the API — meaning the
   * client is asking a path the proxy does not cover. Surfaced as
   * `status = 502` + `code = "invalid-response-content-type"` so
   * callers (notably `AuthProvider`) can distinguish it from a
   * genuine server error.
   */
  static invalidResponse(url, contentType) {
    const ct = contentType ?? "<none>";
    return new StarterError(
      502,
      `Expected JSON from ${url} but got content-type ${ct}. This usually means the request was not routed to the API (e.g. the dev-server proxy is missing this path).`,
      void 0,
      "invalid-response-content-type"
    );
  }
  // Type guard. With one arg, narrows to StarterError; with two, also
  // requires that `.status` matches.
  static is(err, status) {
    if (!(err instanceof StarterError)) return false;
    return status === void 0 || err.status === status;
  }
}

function isJsonContentType(value) {
  if (!value) return false;
  const semi = value.indexOf(";");
  const main = (semi === -1 ? value : value.slice(0, semi)).trim().toLowerCase();
  return main === "application/json" || main === "application/problem+json" || main.endsWith("+json");
}
async function fetchJson(client, path, init = {}) {
  const headers = { ...client.headers, ...init.headers };
  const url = `${client.baseUrl}${path}`;
  const res = await client.fetch(url, {
    ...init,
    credentials: "include",
    headers
  });
  if (!res.ok) throw await StarterError.fromResponse(res);
  if (!isJsonContentType(res.headers.get("content-type"))) {
    throw StarterError.invalidResponse(url, res.headers.get("content-type"));
  }
  return await res.json();
}

const EXTENSION_ID = "com.nexus.demo";
function Main() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(MainRouter, {}) });
}
function MainRouter() {
  const route = useExtensionRoute();
  const page = route === "readings" || route?.startsWith("readings") ? "readings" : route === "about" || route?.startsWith("about") ? "about" : "overview";
  return /* @__PURE__ */ jsxs("div", { className: "mx-auto flex max-w-5xl flex-col gap-6", children: [
    /* @__PURE__ */ jsx(Header, { page }),
    page === "overview" ? /* @__PURE__ */ jsx(OverviewPage, {}) : null,
    page === "readings" ? /* @__PURE__ */ jsx(ReadingsPage, {}) : null,
    page === "about" ? /* @__PURE__ */ jsx(AboutPage, {}) : null
  ] });
}
function Header({ page }) {
  const titles = {
    overview: "Overview",
    readings: "Readings",
    about: "About"
  };
  return /* @__PURE__ */ jsxs("div", { className: "flex flex-col gap-1", children: [
    /* @__PURE__ */ jsx("p", { className: "text-sm text-muted-foreground", children: "Nexus Demo extension" }),
    /* @__PURE__ */ jsx("h1", { className: "text-2xl font-semibold tracking-tight", children: titles[page] }),
    /* @__PURE__ */ jsxs("nav", { className: "mt-2 flex gap-1 border-b", children: [
      /* @__PURE__ */ jsx(Tab, { to: "/x/com.nexus.demo", active: page === "overview", children: "Overview" }),
      /* @__PURE__ */ jsx(Tab, { to: "/x/com.nexus.demo/readings", active: page === "readings", children: "Readings" }),
      /* @__PURE__ */ jsx(Tab, { to: "/x/com.nexus.demo/about", active: page === "about", children: "About" })
    ] })
  ] });
}
function Tab({
  to,
  active,
  children
}) {
  return /* @__PURE__ */ jsx(
    "a",
    {
      href: to,
      className: "-mb-px border-b-2 px-3 py-2 text-sm transition-colors " + (active ? "border-primary font-medium text-foreground" : "border-transparent text-muted-foreground hover:text-foreground"),
      children
    }
  );
}
function OverviewPage() {
  const client = useHostClient();
  const [ping, setPing] = React.useState(null);
  const [error, setError] = React.useState(null);
  React.useEffect(() => {
    let cancelled = false;
    fetchJson(client, `${client.apiPrefix}/query`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ sql: "", kind: `${EXTENSION_ID}.ping` })
    }).then((r) => !cancelled && setPing(r)).catch(
      (e) => !cancelled ? setError(e instanceof Error ? e.message : String(e)) : void 0
    );
    return () => {
      cancelled = true;
    };
  }, [client]);
  const row = ping?.rows?.[0];
  return /* @__PURE__ */ jsxs("div", { className: "flex flex-col gap-6", children: [
    /* @__PURE__ */ jsxs("div", { className: "grid grid-cols-1 gap-4 sm:grid-cols-3", children: [
      /* @__PURE__ */ jsx(Card, { title: "Status", value: error ? "error" : row ? "live" : "…" }),
      /* @__PURE__ */ jsx(Card, { title: "Sites", value: "12", hint: "across 3 regions" }),
      /* @__PURE__ */ jsx(Card, { title: "Open alerts", value: "2", hint: "1 critical" })
    ] }),
    /* @__PURE__ */ jsx(Panel, { title: "Live ping (contributed kind)", children: error ? /* @__PURE__ */ jsx("p", { className: "text-sm text-destructive", children: error }) : row ? /* @__PURE__ */ jsxs("dl", { className: "grid grid-cols-2 gap-2 text-sm", children: [
      /* @__PURE__ */ jsx("dt", { className: "text-muted-foreground", children: "Greeting" }),
      /* @__PURE__ */ jsx("dd", { className: "font-mono", children: row.greeting }),
      /* @__PURE__ */ jsx("dt", { className: "text-muted-foreground", children: "Server time" }),
      /* @__PURE__ */ jsx("dd", { className: "font-mono", children: row.server_time })
    ] }) : /* @__PURE__ */ jsx("p", { className: "text-sm text-muted-foreground", children: "Loading…" }) })
  ] });
}
const READINGS = [
  { site: "HQ — Roof", metric: "Power", value: "42.1 kW", trend: "▲ 3%" },
  { site: "HQ — Floor 2", metric: "Temp", value: "21.4 °C", trend: "▼ 1%" },
  { site: "Depot A", metric: "Water", value: "118 L/min", trend: "▲ 8%" },
  { site: "Depot B", metric: "Power", value: "9.7 kW", trend: "—" }
];
function ReadingsPage() {
  return /* @__PURE__ */ jsx(Panel, { title: "Latest readings", children: /* @__PURE__ */ jsx("div", { className: "overflow-hidden rounded-lg border", children: /* @__PURE__ */ jsxs("table", { className: "w-full text-sm", children: [
    /* @__PURE__ */ jsx("thead", { className: "bg-muted/50 text-left text-muted-foreground", children: /* @__PURE__ */ jsxs("tr", { children: [
      /* @__PURE__ */ jsx("th", { className: "px-3 py-2 font-medium", children: "Site" }),
      /* @__PURE__ */ jsx("th", { className: "px-3 py-2 font-medium", children: "Metric" }),
      /* @__PURE__ */ jsx("th", { className: "px-3 py-2 font-medium", children: "Value" }),
      /* @__PURE__ */ jsx("th", { className: "px-3 py-2 font-medium", children: "Trend" })
    ] }) }),
    /* @__PURE__ */ jsx("tbody", { children: READINGS.map((r) => /* @__PURE__ */ jsxs("tr", { className: "border-t", children: [
      /* @__PURE__ */ jsx("td", { className: "px-3 py-2", children: r.site }),
      /* @__PURE__ */ jsx("td", { className: "px-3 py-2 text-muted-foreground", children: r.metric }),
      /* @__PURE__ */ jsx("td", { className: "px-3 py-2 font-mono", children: r.value }),
      /* @__PURE__ */ jsx("td", { className: "px-3 py-2", children: r.trend })
    ] }, `${r.site}-${r.metric}`)) })
  ] }) }) });
}
function AboutPage() {
  return /* @__PURE__ */ jsx(Panel, { title: "About this extension", children: /* @__PURE__ */ jsxs("div", { className: "flex flex-col gap-3 text-sm leading-relaxed text-muted-foreground", children: [
    /* @__PURE__ */ jsxs("p", { children: [
      /* @__PURE__ */ jsx("span", { className: "font-medium text-foreground", children: "com.nexus.demo" }),
      " is a worked WS-14 example: it contributes a sidebar nav group and a full page rendered into the host's content area, plus two query-kinds and an insight on the backend."
    ] }),
    /* @__PURE__ */ jsxs("p", { children: [
      "The page you're reading is the extension's own federated UI (the",
      " ",
      /* @__PURE__ */ jsx("code", { className: "rounded bg-muted px-1 py-0.5", children: "main" }),
      " slot), mounted by the host route",
      " ",
      /* @__PURE__ */ jsx("code", { className: "rounded bg-muted px-1 py-0.5", children: "/x/:extId/*" }),
      ". The tabs above change the slot route; the extension dispatches its own sub-pages — the host registers no routes for it."
    ] })
  ] }) });
}
function Card({
  title,
  value,
  hint
}) {
  return /* @__PURE__ */ jsxs("div", { className: "rounded-xl border bg-card p-4 text-card-foreground shadow-sm", children: [
    /* @__PURE__ */ jsx("p", { className: "text-sm text-muted-foreground", children: title }),
    /* @__PURE__ */ jsx("p", { className: "mt-1 text-2xl font-semibold tracking-tight", children: value }),
    hint ? /* @__PURE__ */ jsx("p", { className: "mt-1 text-xs text-muted-foreground", children: hint }) : null
  ] });
}
function Panel({
  title,
  children
}) {
  return /* @__PURE__ */ jsxs("section", { className: "rounded-xl border bg-card p-5 text-card-foreground shadow-sm", children: [
    /* @__PURE__ */ jsx("h2", { className: "mb-3 text-sm font-medium", children: title }),
    children
  ] });
}

const BASE = "/x/com.nexus.demo";
const LINKS = [
  { label: "Overview", to: BASE, icon: "▦" },
  { label: "Readings", to: `${BASE}/readings`, icon: "≣" },
  { label: "About", to: `${BASE}/about`, icon: "ⓘ" }
];
function DemoNav() {
  return /* @__PURE__ */ jsxs("div", { className: "flex flex-col gap-0.5 px-2 py-1", children: [
    /* @__PURE__ */ jsx("div", { className: "px-2 pb-1 text-xs font-medium text-sidebar-foreground/50", children: "Nexus Demo" }),
    LINKS.map((l) => /* @__PURE__ */ jsxs(
      "a",
      {
        href: l.to,
        className: "flex h-8 items-center gap-2 rounded-md px-2 text-sm text-sidebar-foreground/80 outline-none ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2",
        children: [
          /* @__PURE__ */ jsx(
            "span",
            {
              "aria-hidden": true,
              className: "grid size-5 shrink-0 place-items-center rounded bg-primary/15 text-primary",
              children: l.icon
            }
          ),
          /* @__PURE__ */ jsx("span", { className: "truncate", children: l.label })
        ]
      },
      l.to
    ))
  ] });
}

const factory = {
  // The host enforces matching majors; nexus-ui ships React 19, so any 19.x
  // declaration negotiates.
  singletons: {
    react: { version: "19.1.0" }
  },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { Main, DemoNav }
    });
  }
};

export { factory as default };
