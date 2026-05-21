// Wire-shape types for the ai-builder page slice. Mirrors the
// `BuilderEvent` enum in `starter-flow-node-ai-builder` (Rust crate)
// per DOCS/frontend/ai-builder/SCOPE.md R1.
//
// Theme-slice payloads (`TokenPatch`, `ShellPatch`) live here too
// because the discriminator is shared across both slices; a
// page-builder consumer that doesn't care about theme events just
// ignores those branches.

import type {
  UiComponent,
  UiComponentTree,
} from "@nube/starter-sdui-react";

export type BuilderPhase =
  | "idle"
  | "thinking"
  | "writing"
  | "done"
  | "error"
  | "cancelled";

/**
 * Theme-builder token-patch payload. Shape kept narrow on purpose —
 * `keys` is a flat `{ tokenName: cssValue }` map; the host validates
 * value shapes against the existing theme editor validator before
 * persisting.
 */
export interface TokenPatch {
  mode: "light" | "dark";
  keys: Record<string, string>;
}

/** Theme-builder shell-config payload. Shape is opaque — passed
 *  through to the existing theme editor's `ShellConfig` handler. */
export interface ShellPatch {
  config: Record<string, unknown>;
}

/**
 * Stream-control envelope. Matches the discriminated union the
 * `ai-builder` flow node emits on its output slot. All members carry
 * `type` so a TS discriminated-union narrows cleanly.
 */
export type BuilderEvent =
  | { type: "full-render"; tree: UiComponentTree }
  | { type: "patch"; targetComponentId: string; subtree: UiComponent }
  | { type: "token-patch"; patch: TokenPatch }
  | { type: "shell-patch"; patch: ShellPatch }
  | { type: "status"; phase: BuilderPhase; message?: string }
  | { type: "error"; error: string };

export interface BuilderSendInput {
  /** Free-text user prompt. */
  text: string;
  /** Optional structured slot writes, forwarded to the flow input. */
  slots?: Record<string, unknown>;
  /** Optional opaque metadata, passed through unchanged. */
  meta?: Record<string, unknown>;
}

/**
 * The single transport seam. The library never speaks HTTP/SSE itself
 * — the consumer plugs whatever stack they use behind this interface
 * (REST against starter-server, MCP tool call, an in-memory fixture,
 * a Tauri command, …).
 *
 * Implementations should respect `signal` and stop emitting promptly
 * on abort.
 */
export interface BuilderAdapter {
  send(
    input: BuilderSendInput,
    signal: AbortSignal,
  ): AsyncIterable<BuilderEvent>;
}
