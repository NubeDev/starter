import { jsx, jsxs, Fragment } from 'react/jsx-runtime';
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

const EXTENSION_ID = "com.nexus.hello";
const KIND = "com.nexus.hello.ping";
function HelloPanel() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(PanelInner, {}) });
}
function PanelInner() {
  const client = useHostClient();
  const slot = useSlotContext();
  const [result, setResult] = React.useState(null);
  const [error, setError] = React.useState(null);
  React.useEffect(() => {
    let cancelled = false;
    fetchJson(client, `${client.apiPrefix}/query`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ sql: "", kind: KIND })
    }).then((r) => {
      if (!cancelled) setResult(r);
    }).catch((e) => {
      if (!cancelled) setError(e instanceof Error ? e.message : String(e));
    });
    return () => {
      cancelled = true;
    };
  }, [client]);
  const row = result?.rows?.[0];
  return /* @__PURE__ */ jsxs(
    "div",
    {
      "data-ext-id": EXTENSION_ID,
      "data-ext-slot": slot.slotId,
      style: {
        margin: "4px 8px",
        padding: "8px 10px",
        borderRadius: 8,
        border: "1px solid color-mix(in oklab, currentColor 15%, transparent)",
        fontSize: 12,
        lineHeight: 1.5,
        opacity: 0.9
      },
      children: [
        /* @__PURE__ */ jsxs("div", { style: { fontWeight: 600 }, children: [
          "👋 ",
          EXTENSION_ID
        ] }),
        error ? /* @__PURE__ */ jsxs("div", { style: { opacity: 0.7 }, children: [
          "kind query failed: ",
          error
        ] }) : row ? /* @__PURE__ */ jsxs(Fragment, { children: [
          /* @__PURE__ */ jsx("div", { children: row.greeting }),
          /* @__PURE__ */ jsx("div", { style: { opacity: 0.6, fontVariantNumeric: "tabular-nums" }, children: row.server_time })
        ] }) : /* @__PURE__ */ jsxs("div", { style: { opacity: 0.6 }, children: [
          "running ",
          KIND,
          "…"
        ] })
      ]
    }
  );
}

const factory = {
  // The host enforces matching majors; nexus-ui ships React 19, so any 19.x
  // declaration negotiates.
  singletons: {
    react: { version: "19.1.0" }
  },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { HelloPanel }
    });
  }
};

export { factory as default };
