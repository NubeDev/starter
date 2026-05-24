// Localizable strings the ai-builder package emits at runtime.
//
// Same pattern as `@nube/starter-ui-chat/i18n` and
// `@nube/starter-ui-flow/i18n`: a typed `BuilderMessages` shape,
// English defaults, a partial-merge helper, and a context
// provider. The package never imports `react-intl` — hosts derive
// a `BuilderMessages` from their own translation hook and pass it
// to `<AiBuilder i18n={…}>` (or wrap the primitives in
// `<BuilderI18nProvider>` directly).
//
// `phase` labels are surfaced both in the header `PhaseBadge` and in
// the `BusyBubble` suffix while streaming.

import type { BuilderPhase } from "../types/index.js";

export interface BuilderMessages {
  /** Default streaming label for the transcript's "busy" bubble. */
  busyLabel: string;
  /** Composer placeholder while in `mode: "build"`. */
  buildPlaceholder: string;
  /** Composer placeholder while in `mode: "ask"`. */
  askPlaceholder: string;
  /** Empty-state copy in the transcript before any prompt is sent. */
  emptyTranscript: string;
  /** "Regenerate" button label below the last assistant turn. */
  regenerate: string;
  /** Tag rendered above an Ask-lane user bubble. */
  askTag: string;
  /** `<ModeToggle>` `aria-label`. */
  modeToggleAriaLabel: string;
  /** Build-mode pill label. */
  buildModeLabel: string;
  /** Build-mode pill `title` hint. */
  buildModeHint: string;
  /** Ask-mode pill label. */
  askModeLabel: string;
  /** Ask-mode pill `title` hint. */
  askModeHint: string;
  /** Canvas empty-state copy when `tree` is null. */
  canvasEmpty: string;
  /**
   * Suffix shown next to the buffered-patches counter, e.g.
   * `"3 buffered"`. The number is rendered separately.
   */
  bufferedSuffix: string;
  /** Phase labels (header `PhaseBadge`). Keys cover every
   * `BuilderPhase` variant. */
  phase: Record<BuilderPhase, string>;
}

/** Default English messages. */
export const DEFAULT_BUILDER_MESSAGES: BuilderMessages = {
  busyLabel: "Working…",
  buildPlaceholder: "Describe the UI you want…",
  askPlaceholder: "Ask a question about your page…",
  emptyTranscript:
    "Tell the agent what to build. Updates stream into the canvas on the right.",
  regenerate: "Regenerate",
  askTag: "Ask",
  modeToggleAriaLabel: "Conversation mode",
  buildModeLabel: "Build",
  buildModeHint: "Generate or edit the page",
  askModeLabel: "Ask",
  askModeHint: "Chat about the page without changing it",
  canvasEmpty: "Send a prompt to start building.",
  bufferedSuffix: "buffered",
  phase: {
    idle: "Idle",
    thinking: "Thinking",
    writing: "Writing",
    done: "Done",
    error: "Error",
    cancelled: "Cancelled",
  },
};

/** Merge a partial override on top of `DEFAULT_BUILDER_MESSAGES`. */
export function mergeBuilderMessages(
  override: Partial<BuilderMessages> | undefined,
): BuilderMessages {
  if (!override) return DEFAULT_BUILDER_MESSAGES;
  return {
    ...DEFAULT_BUILDER_MESSAGES,
    ...override,
    phase: { ...DEFAULT_BUILDER_MESSAGES.phase, ...(override.phase ?? {}) },
  };
}
