// Hand-written Module-Federation bundle. Loaded by the host's
// `@nube/starter-ext-ui` runtime as a plain ES module.
//
// Contract: default-export `{ singletons, init }`. Uses host's React
// (handle.singletons.react) to avoid double-React duplication.

const factory = {
  singletons: { react: { version: "18.3.1" } },
  init(handle) {
    const React = handle.singletons.react;
    if (!React) throw new Error("com.nube.hello: host did not provide react singleton");
    handle.register({ components: { HelloPanel: makePanel(React) } });
  },
};

function makePanel(React) {
  return function HelloPanel() {
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
        React.createElement("label", { style: { display: "block", fontSize: "0.8rem", fontWeight: 500, marginBottom: 6 } }, "Greet someone"),
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
