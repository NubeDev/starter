// `useExtensionEvents` — live SSE feed of the rubix-agent extension
// admin event stream (`GET /api/v1/extensions/events`).
//
// Wraps `useEventStream` from `@nube/starter-client-react` (which in
// turn wraps the `streamJson` SSE primitive) so consumers get the
// same React-friendly `{ events, status, reconnect }` surface as
// every other live-feed in the rubix UI.
//
// Frames are accumulated into a bounded ring (default 200) so a
// noisy extension can't grow React state without limit. Pass
// `bufferSize` to override or `Infinity` to keep every frame.

import { useEffect, useRef, useState } from "react";

import {
  useEventStream,
  type EventStreamStatus,
} from "@nube/starter-client-react";

/** Path mounted by rubix-agent — see `rubix/crates/rubix-agent/src/routes/extensions.rs`. */
const EXTENSION_EVENTS_PATH = "/api/v1/extensions/events";

/**
 * Discriminated-union event shape. Mirrors the wire types emitted
 * by the rubix-agent extension supervisor. Open enough to accept
 * additional `kind`s without forcing a hook revision.
 */
export type ExtensionEvent =
  | { kind: "lifecycle"; extension_id: string; state: string; at_ms: number }
  | { kind: "log"; extension_id: string; level: string; message: string; at_ms: number }
  | { kind: "error"; extension_id: string; message: string; at_ms: number };

export interface UseExtensionEventsOptions {
  /** Maximum frames retained in `events`. Default 200. */
  bufferSize?: number;
  /** Pause subscription. Default `true`. */
  enabled?: boolean;
  /** Test seam — forwarded to `useEventStream`. */
  eventSourceCtor?: typeof EventSource;
  /** Test seam — forwarded to `useEventStream`. */
  forceFetch?: boolean;
}

export interface UseExtensionEventsResult {
  /** Most-recent-last buffer of frames. Length ≤ `bufferSize`. */
  events: ExtensionEvent[];
  /** Last frame, if any — convenient for "live status" displays. */
  latest: ExtensionEvent | null;
  status: EventStreamStatus;
  error: Error | null;
  /** Drop the buffer and reopen the connection. Stable identity. */
  reconnect(): void;
}

export function useExtensionEvents(
  options: UseExtensionEventsOptions = {},
): UseExtensionEventsResult {
  const bufferSize = options.bufferSize ?? 200;
  const stream = useEventStream<ExtensionEvent>(EXTENSION_EVENTS_PATH, {
    enabled: options.enabled,
    eventSourceCtor: options.eventSourceCtor,
    forceFetch: options.forceFetch,
  });

  const [events, setEvents] = useState<ExtensionEvent[]>([]);
  const lastFrameRef = useRef<ExtensionEvent | null>(null);

  // Accumulate frames as they arrive. `useEventStream` only surfaces
  // the latest snapshot, so we promote each new reference into our
  // own ring buffer.
  useEffect(() => {
    const frame = stream.data;
    if (!frame || frame === lastFrameRef.current) return;
    lastFrameRef.current = frame;
    setEvents((prev) => {
      const next = prev.length >= bufferSize ? prev.slice(prev.length - bufferSize + 1) : prev;
      return [...next, frame];
    });
  }, [stream.data, bufferSize]);

  // Reset the buffer when the consumer calls reconnect so old
  // frames don't linger across a deliberate refresh.
  const reconnect = () => {
    setEvents([]);
    lastFrameRef.current = null;
    stream.reconnect();
  };

  return {
    events,
    latest: events.length > 0 ? events[events.length - 1]! : null,
    status: stream.status,
    error: stream.error,
    reconnect,
  };
}
