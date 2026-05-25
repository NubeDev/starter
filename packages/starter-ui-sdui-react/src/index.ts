// # @nube/starter-ui-sdui-react
//
// Headless renderer for starter SDUI trees. Mount one `<SduiPage>`,
// supply an `SduiTransport` via `<SduiProvider>`, and the package
// dispatches one renderer per IR variant. Zero I/O — every network
// call rides on the transport.
//
// The root entry re-exports the platform-neutral headless API from
// `./headless` and additionally side-effect-registers the web
// renderers backed by `@nube/starter-ui-kit`. Mobile (React Native)
// consumers should import from `@nube/starter-ui-sdui-react/headless`
// directly and register their own renderers.

export * from "./headless/index.js";
export * from "./renderer/index.js";
