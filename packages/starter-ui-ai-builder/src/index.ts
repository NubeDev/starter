// # @nube/starter-ui-ai-builder
//
// AI-assisted SDUI page-builder primitives. Pairs a chat transcript
// with a live `<Renderer>` canvas that streams updates from a
// `BuilderAdapter`. Transport-agnostic — wire the adapter against the
// future `starter-flow-node-ai-builder` SSE endpoint, an MCP tool,
// `fetch`, an in-process Tauri command, or `createFixtureBuilderAdapter`
// for stories and tests.
//
// Quick start:
//
// ```tsx
// import { AiBuilder, createFixtureBuilderAdapter } from "@nube/starter-ui-ai-builder";
//
// const adapter = createFixtureBuilderAdapter({
//   scripts: [
//     {
//       matchPrefix: "dashboard",
//       events: [
//         { type: "full-render", tree: myInitialTree },
//         { type: "status", phase: "done" },
//       ],
//     },
//   ],
// });
//
// export default function Page() {
//   return <AiBuilder adapter={adapter} title="AI Builder" />;
// }
// ```
//
// For full control, compose primitives directly:
// `useBuilder` + `<BuilderTranscript>` + `<AiBuilderCanvas>`.

export * from "./types/index.js";
export * from "./hooks/index.js";
export * from "./adapters/index.js";

export { AiBuilder } from "./components/ai-builder.js";
export type { AiBuilderProps } from "./components/ai-builder.js";
export { AiBuilderCanvas } from "./components/ai-builder-canvas.js";
export type { AiBuilderCanvasProps } from "./components/ai-builder-canvas.js";
export { BuilderTranscript } from "./components/builder-transcript.js";
export type { BuilderTranscriptProps } from "./components/builder-transcript.js";

export * from "./i18n/index.js";

export { cn, makeId, treeHasId } from "./lib/utils.js";
