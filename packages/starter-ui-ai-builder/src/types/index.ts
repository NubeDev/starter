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
 * Conversation lane.
 *
 * - `"build"` (default): the model generates / edits the SDUI tree;
 *   the response lands as a `full-render` (or `patch`) event and the
 *   canvas updates.
 * - `"ask"`: the model answers conversationally without touching
 *   the tree; the response lands as a `message` event and the
 *   transcript shows it as an assistant bubble.
 *
 * Backend contract: forwarded as the `mode` field on the
 * `/api/builder/stream` request body. Unknown values return 400.
 */
export type BuilderMode = "build" | "ask";

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
  /** Ask-mode reply. The assistant's prose answer; render as a chat
   *  bubble in the transcript. Build-mode turns never emit this. */
  | { type: "message"; role: "assistant"; text: string }
  | { type: "status"; phase: BuilderPhase; message?: string }
  | { type: "error"; error: string }
  /** MEMORY.md Phase M-D — server confirms the assistant turn's
   *  output was persisted as a versioned artifact under this
   *  session. Surface uses this to refresh undo state. */
  | {
      type: "session_artifact";
      session_id: string;
      key: string;
      /** May be absent on backends that don't echo the assigned
       *  version in-band — surface should refetch on demand. */
      version?: number;
    }
  /** MEMORY.md Phase M-D — server kept the response intact but the
   *  session-store write failed. The request still completed; the
   *  surface should degrade gracefully and stay stateless. */
  | { type: "session_error"; error: string };

export interface BuilderSendInput {
  /** Free-text user prompt. */
  text: string;
  /** Conversation lane for this turn. Defaults to `"build"` when
   *  omitted; surfaces that expose a Build/Ask toggle pass the
   *  current selection here. */
  mode?: BuilderMode;
  /** Optional structured slot writes, forwarded to the flow input. */
  slots?: Record<string, unknown>;
  /** Optional opaque metadata, passed through unchanged. */
  meta?: Record<string, unknown>;
  /** MEMORY.md Phase M-D — opt into session persistence. When set,
   *  the adapter forwards the id as `session_id` and the backend
   *  persists this turn + the produced artifact. Omitting it keeps
   *  the call ephemeral per M13. */
  sessionId?: string;
  /** MEMORY.md Phase M-D — artifact key (e.g. `"tree"`) to seed
   *  the prompt with from the session's latest snapshot. Honored
   *  only when `sessionId` is set. */
  includeArtifact?: string;
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
