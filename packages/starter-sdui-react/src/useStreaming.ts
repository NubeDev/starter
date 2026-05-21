/**
 * `useStreaming` — wires a single component-scoped streaming
 * subscription. Used by `text`, `markdown`, `code`, and `timeline`
 * nodes that carry a `subscribe` subject string and an optional
 * `mode: "append" | "replace"`.
 *
 * Per SCOPE.md § "Streaming content", the server emits a stable
 * end-of-stream sentinel (decision S-D5, inherited verbatim from
 * Rubix):
 *
 *     { "type": "stream_end",
 *       "channel": "...",
 *       "reason": "done" | "error" | "timeout" | "gone" }
 *
 * On `stream_end` the hook unsubscribes, surfaces the reason
 * through `onEnd`, and the component renders its terminal state
 * (no further chunks expected). The server's 60-second inactivity
 * timeout produces `reason: "timeout"`; a client unmount drops
 * the subscription and the server GCs its channel within the same
 * window.
 *
 * The streaming transport is **host-provided**: the
 * `SubscriptionTransport.subscribe` callback delivers either a
 * string / unknown chunk (appended or replaced) or a structured
 * `{ type: "stream_end", reason }` sentinel. The hook itself is
 * transport-agnostic — SSE, WebSocket, NATS, polling, anything
 * the host wires up.
 */
import { useEffect, useState } from "react";
import { useSdui } from "./context.js";
import type { SubscriptionTransport } from "./useSubscriptions.js";

/** End-of-stream sentinel reason values per SCOPE S-D5. */
export type StreamEndReason = "done" | "error" | "timeout" | "gone";

/** Sentinel payload pushed by the transport when the stream closes. */
export interface StreamEndSentinel {
  type: "stream_end";
  reason: StreamEndReason;
  channel?: string;
}

function isEndSentinel(v: unknown): v is StreamEndSentinel {
  return (
    typeof v === "object" &&
    v !== null &&
    (v as { type?: unknown }).type === "stream_end"
  );
}

export interface UseStreamingOptions {
  /** Subscription subject string (NATS-shaped). */
  subscribe?: string;
  /** Initial seed value the server baked into the IR. */
  initial?: string;
  /** `"append"` (default) accumulates chunks; `"replace"` swaps. */
  mode?: "append" | "replace";
  /** Transport override — defaults to a render-time noop. */
  transport?: SubscriptionTransport;
  /** Fired when the server emits the `stream_end` sentinel. */
  onEnd?: (reason: StreamEndReason) => void;
}

export interface StreamingState {
  value: string;
  /** End-of-stream reason once the stream closes. */
  endedReason?: StreamEndReason;
}

/**
 * Append / replace into a string scratch buffer as chunks arrive.
 * Unmounting drops the subscription — the server's GC closes the
 * channel within its 60s inactivity window per SCOPE S-D5.
 */
export function useStreaming(opts: UseStreamingOptions): StreamingState {
  const { subscribe, initial, mode, transport, onEnd } = opts;
  const [value, setValue] = useState<string>(initial ?? "");
  const [endedReason, setEndedReason] = useState<StreamEndReason | undefined>();

  // Keep the buffer in sync when the server's baked-in `initial`
  // changes — happens on a `patch` / `full_render` round-trip.
  useEffect(() => {
    setValue(initial ?? "");
    setEndedReason(undefined);
  }, [initial]);

  useEffect(() => {
    if (!subscribe || !transport) return;
    const unsubscribe = transport.subscribe(
      { key: subscribe, target_node_id: "", slot: subscribe },
      (chunk) => {
        if (isEndSentinel(chunk)) {
          setEndedReason(chunk.reason);
          onEnd?.(chunk.reason);
          // Defensive: drop the subscription immediately so the
          // transport stops delivering after the sentinel.
          unsubscribe();
          return;
        }
        const text = typeof chunk === "string" ? chunk : String(chunk ?? "");
        if (mode === "replace") {
          setValue(text);
        } else {
          setValue((prev) => prev + text);
        }
      },
    );
    return unsubscribe;
    // `transport` is provider-stable; `subscribe` / `mode` define identity.
  }, [subscribe, transport, mode, onEnd]);

  return { value, endedReason };
}

/**
 * Module-level transport slot — components that want streaming pull
 * the transport via `useSdui()` (host provides) or fall back to this
 * shared registration. Tests can install a fake transport without
 * threading provider boilerplate through every renderer test.
 */
let registeredTransport: SubscriptionTransport | undefined;
export function registerStreamingTransport(t?: SubscriptionTransport): void {
  registeredTransport = t;
}
export function getStreamingTransport(): SubscriptionTransport | undefined {
  return registeredTransport;
}

/**
 * Component-side hook — pulls the transport from the optional
 * SDUI context extension or the module-level registration. Keeps
 * each streaming component a one-liner.
 */
export function useStreamingTransport(): SubscriptionTransport | undefined {
  // The transport sits next to the action dispatcher in `useSdui`;
  // the optional shape keeps existing consumers untouched.
  const ctx = useSdui() as unknown as {
    streamingTransport?: SubscriptionTransport;
  };
  return ctx.streamingTransport ?? getStreamingTransport();
}
