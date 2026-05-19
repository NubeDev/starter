// Placeholder bundle output.
//
// A real build wires `ui-src/remoteEntry.ts` through webpack/rspack
// with the Module-Federation plugin, externalises `react`, and emits
// the runtime bundle at this path. The host's `@nube/starter-ext-ui`
// loader fetches it from `/extensions/com.nube.notes/ui/remoteEntry.js`
// (the `starter-ext-server` admin slice serves the bundle dir).
//
// For the demo this stub lets the UI route 200 on the request so the
// loader can see something — replace it with the real bundle output
// to actually mount the panel.
export default {
  singletons: { react: { version: "18.3.1" } },
  init: () => {},
};
