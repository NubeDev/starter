// # @nube/starter-ui-flow
//
// React components for the `starter-flow` engine. Wraps @xyflow/react
// (React Flow v12) into the shape `starter-flow-spi` cares about: typed
// slots, node-kind registry, branded IDs, and read-only visualisation
// or interactive authoring of a `Flow`.
//
// R6 of the starter SCOPE: this package is zero-I/O. No fetches, no
// stores. Consumers feed it `FlowGraph` data and receive `onChange`
// events. Persistence is the host app's job.
//
// One-line stylesheet import at app entry:
//
//     import "@xyflow/react/dist/style.css";
//     import "@nube/starter-ui-flow/styles.css";

export * from "./types.js";
export * from "./canvas/index.js";
export * from "./nodes/index.js";
export * from "./edges/index.js";
export * from "./slots/index.js";
export * from "./hooks/index.js";
export * from "./i18n/index.js";
