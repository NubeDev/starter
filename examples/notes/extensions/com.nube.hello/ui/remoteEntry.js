// Hand-written Module-Federation bundle output. No webpack/rspack
// build in this demo — the file is loaded directly by the host's
// `@nube/starter-ext-ui` runtime as an ES module.
//
// Contract: default-export `{ singletons, init }`. The host validates
// majors, then calls `init(handle)` with the negotiated React
// instance available at `handle.singletons.react`. Using the host's
// React rather than importing our own is what keeps the two trees
// from de-duplicating into "two copies of React".

const factory = {
  singletons: { react: { version: "18.3.1" } },
  init(handle) {
    const React = handle.singletons.react;
    if (!React) {
      throw new Error("com.nube.hello: host did not provide react singleton");
    }
    handle.register({ components: { HelloPanel: makePanel(React) } });
  },
};

function makePanel(React) {
  return function HelloPanel() {
    const [name, setName] = React.useState("world");
    const [message, setMessage] = React.useState(null);
    const [error, setError] = React.useState(null);

    const refresh = React.useCallback(async () => {
      try {
        const res = await fetch(
          "/hello?name=" + encodeURIComponent(name || "world"),
        );
        if (!res.ok) throw new Error("HTTP " + res.status);
        const body = await res.json();
        setMessage(body.message ?? null);
        setError(null);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    }, [name]);

    React.useEffect(() => {
      void refresh();
    }, [refresh]);

    return React.createElement(
      "section",
      {
        style: {
          border: "1px dashed #aac",
          borderRadius: 8,
          padding: 12,
          marginTop: 16,
        },
      },
      React.createElement("strong", null, "com.nube.hello"),
      React.createElement(
        "div",
        { style: { marginTop: 8, display: "flex", gap: 8 } },
        React.createElement("input", {
          "aria-label": "name",
          value: name,
          onChange: (e) => setName(e.target.value),
          style: { flex: 1, padding: 6 },
        }),
        React.createElement(
          "button",
          { type: "button", onClick: () => void refresh() },
          "greet",
        ),
      ),
      error
        ? React.createElement(
            "p",
            { style: { color: "crimson", marginTop: 8 } },
            error,
          )
        : React.createElement(
            "p",
            { style: { marginTop: 8 } },
            message ?? "(loading)",
          ),
    );
  };
}

export default factory;
