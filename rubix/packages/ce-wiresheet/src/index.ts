// `@nube/ce-wiresheet` — public surface.
//
// The editor is a single component parameterised by the engine's REST origin
// (`base`, e.g. `http://192.168.1.50:7878`). It speaks the control engine's
// REST `/api/v0` + binary-WebSocket `/ws` protocol and renders the wiresheet
// on React Flow. Everything below `CeEditor` (the rest/ws/wire/store layer and
// the node/edge components) is internal.
//
// Consumed two ways:
//   - the standalone dev harness (`rubix/apps/ce-wiresheet-dev`) for fast HMR,
//   - the `com.nubeio.ce` rubix extension (its wiresheet route).

export { default as CeEditor } from "./CeEditor";
export { setEngineBase } from "./lib/rest";
export { wsUrlFromBase } from "./lib/ws";
export type {
  Component,
  Edge,
  FlexValue,
} from "./lib/engine-types";
