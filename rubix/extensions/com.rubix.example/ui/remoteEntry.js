// `ui/remoteEntry.js` — Module-Federation entry for com.rubix.example.
//
// This file is a working, hand-authored remoteEntry bundle. It is
// what `starter-ext-server` serves at
// `GET /api/v1/extensions/com.rubix.example/ui/remoteEntry.js` and
// what the host's `@nube/starter-ext-ui::bootstrapExtensions` loop
// dynamic-imports.
//
// Why hand-authored instead of bundler output? The full pipeline
// (`vite-plugin-federation` + matching host config + shared-deps
// graph) is significantly heavier than this single example needs.
// The contract a real federated bundle satisfies — emit ESM with
// React externalised, expose a `{ singletons, init(handle) }`
// factory, get React from the negotiated handle — is preserved
// exactly: no `import "react"` at the top of this file; React is
// pulled from `handle.singletons.react` inside `init`. A future
// build pipeline drop-in (see SCOPE Phase E) will replace this file
// without changing the surface.
//
// The developer-facing source lives next to it in `main.tsx` — the
// build pipeline, when added, compiles that file and overwrites
// this one.

const ID = "com.rubix.example";

/** @typedef {{ id: string, singletons: Record<string, unknown>, register(c: { components: Record<string, unknown> }): void }} ExtensionRemoteHandle */

/**
 * Build the `Main` panel component using the host's React.
 *
 * Plain `React.createElement` calls instead of JSX so this file
 * needs no transpile step. The component renders the same content
 * the `main.tsx` source describes: an extension id stamp + slot id +
 * theme mode, styled with CSS variables that fall through to the
 * host's theme tokens.
 *
 * @param {*} React host React singleton (post-negotiation).
 */
function buildMainComponent(React) {
  return function Main(props) {
    // The host's `<ExtensionSlot>` passes `slotId` to every
    // contributed component as a prop alongside any caller-supplied
    // props. We read it defensively — a future host change might
    // route it through context instead.
    const slotId = (props && props.slotId) || "main";
    return React.createElement(
      "section",
      {
        "data-ext-id": ID,
        "data-ext-slot": slotId,
        style: {
          padding: "0.75rem 1rem",
          borderRadius: "0.5rem",
          border: "1px solid var(--color-border, rgba(0,0,0,0.1))",
          background: "var(--color-surface, transparent)",
        },
      },
      React.createElement(
        "p",
        { style: { margin: 0 } },
        "hello-from-com.rubix.example",
      ),
      React.createElement(
        "small",
        { style: { opacity: 0.7 } },
        "slot=",
        React.createElement("code", null, slotId),
      ),
    );
  };
}

/**
 * The factory the host calls. Default export per
 * `ExtensionRemoteFactory` in `@nube/starter-ext-ui`.
 */
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
