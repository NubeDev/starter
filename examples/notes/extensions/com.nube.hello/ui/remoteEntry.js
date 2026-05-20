// Hand-written Module-Federation bundle. Loaded by the host's
// `@nube/starter-ext-ui` runtime as a plain ES module — no bundler,
// no transpile step (SCOPE R7: static metadata only). The factory
// reads its dependencies off `handle.singletons`, so the entire file
// remains a single ESM module that runs verbatim in the browser.
//
// Stage 4 contract (examples/notes/user-pref.md):
//
//   The panel declares the two new ui-core singletons next to
//   `react`, reads the host's `PreferencesContext` + `IntlShape` off
//   the handle, and renders four localised + formatted surfaces:
//
//     1. greeting    — catalog lookup (`com.nube.hello.greeting`),
//                      proves language flips reach the panel.
//     2. unread      — ICU plural (`com.nube.hello.unread`),
//                      proves the catalog format is plural-aware.
//     3. today       — `formatDate(now)` against the host's resolved
//                      prefs, proves AU date matches host chrome.
//     4. temperature — `formatQuantity(22.44, "temperature",
//                      "celsius")`, proves the BBQ override flips
//                      °C → °F without the panel knowing units.
//
// Because the file is hand-written ESM, it cannot import from
// `@nube/starter-ext-sdk-ts`. Instead it inlines the minimum subset
// of `formatters.ts` it needs (the same affine table + the same
// Intl.DateTimeFormat options) so the rendered values match the host
// chrome to the byte. The drift surface is one PR: any change to the
// SDK formatter shape lands together with a refresh here.

const SINGLETON_REACT = "react";
const SINGLETON_PREFS = "@nube/starter-ui-core/preferences";
const SINGLETON_I18N = "@nube/starter-ui-core/i18n";

const factory = {
  // Major-pinned versions match what the notes host provides in
  // `frontend/src/extension-host.ts`. Bumping the host's major refuses
  // this load (`singleton-mismatch`); a higher host minor surfaces as
  // `extension.singleton_minor_drift` telemetry but still loads. We
  // declare the floor (`1.0.0`) we were built against.
  singletons: {
    [SINGLETON_REACT]: { version: "18.3.1" },
    [SINGLETON_PREFS]: { version: "1.0.0" },
    [SINGLETON_I18N]: { version: "1.0.0" },
  },
  init(handle) {
    const React = handle.singletons[SINGLETON_REACT];
    const PrefsContext = handle.singletons[SINGLETON_PREFS];
    const IntlContext = handle.singletons[SINGLETON_I18N];
    if (!React) {
      throw new Error("com.nube.hello: host did not provide react singleton");
    }
    if (!PrefsContext) {
      throw new Error(
        "com.nube.hello: host did not provide the " +
          "@nube/starter-ui-core/preferences singleton — check the host's " +
          "ExtensionHostManager singleton table.",
      );
    }
    if (!IntlContext) {
      throw new Error(
        "com.nube.hello: host did not provide the " +
          "@nube/starter-ui-core/i18n singleton — check the host's " +
          "ExtensionHostManager singleton table.",
      );
    }
    handle.register({
      components: {
        HelloPanel: makePanel(React, PrefsContext, IntlContext),
      },
    });
  },
};

// --- formatter mirror -------------------------------------------------
//
// Mirrors `starter-extensions/packages/starter-ext-sdk-ts/src/
// formatters.ts` for the two formatters this panel renders. Kept tiny
// and side-effect-free so it is obvious at review time that no fetch,
// no state, and no second source of truth is being introduced.

const UNIT_SYMBOL = {
  celsius: "°C",
  fahrenheit: "°F",
};

// Affine conversion to the canonical SI unit (kelvin-displaced
// celsius). Same table the SDK ships; we only need temperature here.
const TO_CANONICAL = {
  celsius: { scale: 1, offset: 0 },
  fahrenheit: { scale: 5 / 9, offset: -32 * (5 / 9) },
};

function convertTemperature(value, sourceUnit, targetUnit) {
  if (sourceUnit === targetUnit) return value;
  const src = TO_CANONICAL[sourceUnit];
  const dst = TO_CANONICAL[targetUnit];
  const canonical = value * src.scale + src.offset;
  return (canonical - dst.offset) / dst.scale;
}

function dateOptions(fmt, timeZone) {
  switch (fmt) {
    case "auto":
      return { timeZone, dateStyle: "short" };
    case "YYYY-MM-DD":
    case "DD/MM/YYYY":
    case "MM/DD/YYYY":
      return { timeZone, year: "numeric", month: "2-digit", day: "2-digit" };
    default:
      return { timeZone, dateStyle: "short" };
  }
}

function numberLocaleChain(prefs) {
  const fmt = prefs.number_format;
  if (fmt === "auto") return [prefs.locale];
  const forced =
    fmt === "1,234.56" ? "en-US" : fmt === "1.234,56" ? "de-DE" : "fr-FR";
  const sample = new Intl.NumberFormat(prefs.locale).format(1234.56);
  const matches =
    (fmt === "1,234.56" && sample === "1,234.56") ||
    (fmt === "1.234,56" && sample === "1.234,56") ||
    (fmt === "1 234,56" && (sample === "1 234,56" || sample === "1 234,56"));
  return matches ? [prefs.locale] : [forced, prefs.locale];
}

function formatDate(ts, prefs) {
  return new Intl.DateTimeFormat(
    prefs.locale,
    dateOptions(prefs.date_format, prefs.timezone),
  ).format(new Date(ts));
}

function formatTemperatureQuantity(valueCelsius, prefs) {
  const target = prefs.temperature_unit;
  const converted = convertTemperature(valueCelsius, "celsius", target);
  const num = new Intl.NumberFormat(numberLocaleChain(prefs), {
    maximumFractionDigits: 2,
  }).format(converted);
  return `${num} ${UNIT_SYMBOL[target] || target}`;
}

// --- panel ------------------------------------------------------------

function makePanel(React, PrefsContext, IntlContext) {
  return function HelloPanel() {
    // Read the singletons via React.useContext — this is the call that
    // makes the panel a *consumer* of the host's prefs + i18n state.
    // A language flip in the host's <IntlProvider> rerenders this
    // panel in the same commit; a `setPreferences` patch rerenders
    // it the same way. One source of truth, one fetch.
    const prefsCtx = React.useContext(PrefsContext);
    const intlCtx = React.useContext(IntlContext);
    if (!prefsCtx || !prefsCtx.preferences) {
      throw new Error(
        "com.nube.hello/HelloPanel: PreferencesContext not resolved. " +
          "The host's <PreferencesProvider> should hold back render until " +
          "preferences are loaded (Stage 1 loading contract).",
      );
    }
    if (!intlCtx) {
      throw new Error(
        "com.nube.hello/HelloPanel: IntlContext not resolved. " +
          "The host's <IntlProvider> must wrap the extension host slot.",
      );
    }
    const prefs = prefsCtx.preferences;
    const intl = intlCtx.intl;

    // Local UI state for the existing greet button. The button keeps
    // working because the REST call doesn't change; the human-facing
    // strings around it now flow through the catalog so a Spanish
    // operator sees Spanish chrome.
    const [name, setName] = React.useState("world");
    const [message, setMessage] = React.useState(null);
    const [error, setError] = React.useState(null);
    const [loading, setLoading] = React.useState(false);
    const [callCount, setCallCount] = React.useState(0);

    const greet = React.useCallback(async () => {
      setLoading(true);
      setError(null);
      try {
        const res = await fetch("/hello", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name: name || "world" }),
        });
        if (!res.ok) throw new Error("HTTP " + res.status + " " + res.statusText);
        const body = await res.json();
        setMessage(body.message ?? null);
        setCallCount((c) => c + 1);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setLoading(false);
      }
    }, [name]);

    React.useEffect(() => { void greet(); }, []);

    // The four Stage-4 surfaces. Each goes through the host's IntlShape
    // (catalog + plural) or the host's resolved prefs (date + unit) —
    // no second copy of the catalog, no second copy of the prefs.
    const greetingText = intl.formatMessage(
      { id: "com.nube.hello.greeting" },
      { name: name || "world" },
    );
    const unreadCount = callCount;
    const unreadText = intl.formatMessage(
      { id: "com.nube.hello.unread" },
      { count: unreadCount },
    );
    const todayText = formatDate(Date.now(), prefs);
    const temperatureText = formatTemperatureQuantity(22.44, prefs);

    // Styles use the host's CSS variables (--background, --primary, etc.)
    // so the panel matches the shell's design tokens automatically.
    const card = {
      borderRadius: "0.5rem",
      border: "1px solid hsl(var(--border, 240 5.9% 90%))",
      background: "hsl(var(--card, 0 0% 100%))",
      boxShadow: "0 1px 2px rgba(0,0,0,0.05)",
      padding: 0,
      overflow: "hidden",
    };
    const cardHeader = {
      padding: "16px 20px 12px",
      borderBottom: "1px solid hsl(var(--border, 240 5.9% 90%))",
      background: "linear-gradient(to bottom, hsl(var(--muted, 240 4.8% 95.9%)), transparent)",
    };
    const cardBody = { padding: "16px 20px" };
    const badge = (variant) => ({
      display: "inline-flex",
      alignItems: "center",
      gap: 4,
      padding: "2px 8px",
      borderRadius: 9999,
      fontSize: "0.7rem",
      fontWeight: 500,
      background:
        variant === "success" ? "hsl(142 71% 45%)" :
        variant === "error" ? "hsl(0 84.2% 60.2%)" :
        variant === "loading" ? "hsl(38 92% 50%)" :
        "hsl(240 4.8% 95.9%)",
      color:
        variant === "success" || variant === "error" || variant === "loading"
          ? "white" : "hsl(240 5.9% 10%)",
    });
    const input = {
      flex: 1,
      padding: "8px 12px",
      borderRadius: "0.375rem",
      border: "1px solid hsl(var(--border, 240 5.9% 90%))",
      fontSize: "0.875rem",
      outline: "none",
      background: "transparent",
    };
    const button = (primary) => ({
      padding: "8px 16px",
      borderRadius: "0.375rem",
      border: primary ? "none" : "1px solid hsl(var(--border, 240 5.9% 90%))",
      background: primary ? "hsl(240 5.9% 10%)" : "transparent",
      color: primary ? "white" : "hsl(240 5.9% 10%)",
      fontSize: "0.875rem",
      fontWeight: 500,
      cursor: loading ? "wait" : "pointer",
      opacity: loading ? 0.6 : 1,
      transition: "opacity 150ms",
    });
    const factRow = {
      display: "flex",
      justifyContent: "space-between",
      alignItems: "baseline",
      gap: 12,
      padding: "6px 0",
      borderBottom: "1px dashed hsl(var(--border, 240 5.9% 90%))",
      fontSize: "0.8rem",
    };
    const factLabel = { color: "hsl(240 3.8% 46.1%)" };
    const factValue = { fontVariantNumeric: "tabular-nums", fontWeight: 500 };

    const status = loading ? "loading" : error ? "error" : message ? "success" : "idle";
    const statusLabel = {
      loading: "● Calling…",
      error: "● Failed",
      success: "● Live",
      idle: "○ Idle",
    }[status];

    return React.createElement("section", { style: card },
      // Header
      React.createElement("div", { style: cardHeader },
        React.createElement("div", { style: { display: "flex", justifyContent: "space-between", alignItems: "center" } },
          React.createElement("div", null,
            React.createElement("div", { style: { fontWeight: 600, fontSize: "0.95rem" } }, "🧩 Hello Extension"),
            React.createElement("div", { style: { fontSize: "0.75rem", color: "hsl(240 3.8% 46.1%)", marginTop: 2, fontFamily: "monospace" } }, "com.nube.hello"),
          ),
          React.createElement("span", { style: badge(status) }, statusLabel),
        ),
      ),
      // Body
      React.createElement("div", { style: cardBody },
        // Stage-4 i18n + prefs surfaces. Each row has a data-testid so
        // the Stage-6 Playwright spec can pin the exact text per
        // language flip without traversing the entire DOM.
        React.createElement("div", { "data-testid": "hello-greeting", style: { fontSize: "0.95rem", fontWeight: 600, marginBottom: 8 } }, greetingText),
        React.createElement("div", { "data-testid": "hello-unread", style: { fontSize: "0.8rem", color: "hsl(240 3.8% 46.1%)", marginBottom: 12 } }, unreadText),
        React.createElement("div", { style: factRow },
          React.createElement("span", { style: factLabel }, "today"),
          React.createElement("span", { "data-testid": "hello-date", style: factValue }, todayText),
        ),
        React.createElement("div", { style: factRow },
          React.createElement("span", { style: factLabel }, "BBQ"),
          React.createElement("span", { "data-testid": "hello-temperature", style: factValue }, temperatureText),
        ),
        // Existing greet form — kept so the demo's REST round-trip
        // still works. The input label and button stay short and
        // unlocalised here (Stage-5 catalogs will pick up the
        // remaining strings); the surfaces tested by Stage 6 are the
        // four data-testid'd lines above.
        React.createElement("label", { style: { display: "block", fontSize: "0.8rem", fontWeight: 500, margin: "14px 0 6px" } }, "Greet someone"),
        React.createElement("div", { style: { display: "flex", gap: 8 } },
          React.createElement("input", {
            "aria-label": "name",
            value: name,
            onChange: (e) => setName(e.target.value),
            placeholder: "Enter a name…",
            style: input,
            onKeyDown: (e) => { if (e.key === "Enter") void greet(); },
          }),
          React.createElement("button", { type: "button", onClick: () => void greet(), disabled: loading, style: button(true) },
            loading ? "..." : "Greet",
          ),
        ),
        // Response
        React.createElement("div", {
          style: {
            marginTop: 14,
            padding: "12px 14px",
            borderRadius: "0.375rem",
            background: error ? "hsl(0 84% 97%)" : "hsl(142 71% 96%)",
            border: "1px solid " + (error ? "hsl(0 84% 85%)" : "hsl(142 71% 80%)"),
            fontSize: "0.875rem",
          },
        },
          error
            ? React.createElement("span", { style: { color: "hsl(0 84.2% 40%)" } }, "⚠ " + error)
            : React.createElement("span", { style: { color: "hsl(142 71% 25%)" } },
                "💬 ", React.createElement("strong", null, message ?? "(no response yet)"),
              ),
        ),
        // Footer
        React.createElement("div", {
          style: {
            marginTop: 12,
            display: "flex",
            justifyContent: "space-between",
            fontSize: "0.7rem",
            color: "hsl(240 3.8% 46.1%)",
          },
        },
          React.createElement("span", null, "POST /hello → BuiltinRestDispatcher"),
          React.createElement("span", null, "calls: ", callCount),
        ),
      ),
    );
  };
}

export default factory;
