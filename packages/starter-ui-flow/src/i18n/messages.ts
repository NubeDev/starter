// Localizable strings the package emits at runtime.
//
// The package stays react-intl-free (matches the rest of the kit —
// see `starter-ui-kit/src/theme-editor/config-drawer/drawer.tsx`).
// Hosts derive a `FlowMessages` object from their own translation
// hook and pass it via `FlowCanvas.i18n` (or `BaseNode.i18n` if you
// render nodes outside the canvas).
//
// Every visible string the package owns lives here. Built-in node
// kind labels (Tool Call, Trigger, …) intentionally do NOT live in
// this map: they're part of the `NodeKindSpec`, which is data the
// host owns and translates the same way it translates any other
// kind it registers — see the `i18n.kindLabels` override below.

import type { NodeRunState } from "../types.js";

export interface FlowMessages {
  /** Labels for the `NodeRunState` indicator. Surfaces as the state
   * dot's `aria-label` and `title`. */
  state: Record<NodeRunState, string>;
  /** Optional label overrides for built-in node kinds. Key is the
   * `NodeKindSpec.kind` id (e.g. `"ai-agent"`). When supplied, the
   * value replaces `spec.label` in the node header. Hosts that
   * register their own kinds typically translate at registration
   * time and skip this map. */
  kindLabels?: Record<string, string>;
  /** Optional label overrides for slot names, keyed as
   * `"<kind>.<slot>"` (e.g. `"ai-agent.tools"`). When supplied,
   * the value replaces `slot.label` in the slot row. Same rule as
   * `kindLabels`: hosts that own their specs usually translate at
   * registration time. */
  slotLabels?: Record<string, string>;
}

/** Default English messages. Used when no `i18n` prop is supplied
 * and as a fallback for missing keys. */
export const DEFAULT_FLOW_MESSAGES: FlowMessages = {
  state: {
    idle: "Idle",
    ready: "Ready",
    running: "Running",
    ok: "Succeeded",
    error: "Failed",
    cancelled: "Cancelled",
    skipped: "Skipped",
  },
};

/** Merge a partial override on top of `DEFAULT_FLOW_MESSAGES`.
 * Exposed so consumers can supply only the keys they care about. */
export function mergeFlowMessages(
  override: Partial<FlowMessages> | undefined,
): FlowMessages {
  if (!override) return DEFAULT_FLOW_MESSAGES;
  return {
    state: { ...DEFAULT_FLOW_MESSAGES.state, ...(override.state ?? {}) },
    kindLabels: override.kindLabels,
    slotLabels: override.slotLabels,
  };
}
