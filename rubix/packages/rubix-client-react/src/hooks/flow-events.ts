// `useFlowEvents` — live SSE feed of per-flow runtime events served
// by rubix-agent at `GET /api/v1/flows/{flow_id}/events`.
//
// The wire today carries one default `data:` frame per
// engine-side `FlowEvent::NodeEmitted`, JSON-encoded as
// `NodeSlotValue` (see
// `rubix/crates/rubix-agent/src/routes/flow_events.rs` and
// `crates/starter-flow-spi/src/event_dto.rs`). Other event variants
// (`NodeStarted`, `NodeFailed`, …) are filtered server-side today
// and will land as *named* SSE `event:` types so today's `data:`-only
// subscribers keep working. The aggregator below is shaped around
// the broader event model so callers can pass `runOverlay` straight
// to `<FlowCanvas overlay>` once the richer wire frames land.
//
// Wraps `useEventStream` from `@nube/starter-client-react` so
// consumers get the same `{ events, status, reconnect }` surface as
// the other live-feed hooks in the rubix UI.

import { useEffect, useMemo, useRef, useState } from "react";

import {
  useEventStream,
  type EventStreamStatus,
} from "@nube/starter-client-react";

/** Build the SSE path for a given flow id. */
export function flowEventsPath(flowId: string): string {
  return `/api/v1/flows/${encodeURIComponent(flowId)}/events`;
}

/**
 * Wire DTO mirroring `starter_flow_spi::event_dto::NodeSlotValue`.
 * Kept structural so this package stays free of an engine-side type
 * dependency — the rubix UI does the same elsewhere (see
 * `flow-ops.ts`).
 */
export interface NodeSlotValue {
  /** Run id (ULID/UUID string). */
  run: string;
  /** Engine node id (reverse-DNS). */
  node: string;
  /** Output slot name on the emitting node. */
  slot: string;
  /** JSON-projected slot value. */
  value: unknown;
}

/**
 * Per-node run state, mirrored structurally from
 * `@nube/starter-ui-flow`'s `RunOverlay.nodes` entry type so this
 * package can stay dep-free. Cast at the `<FlowCanvas>` mount site.
 */
export type NodeRunState =
  | "idle"
  | "ready"
  | "running"
  | "ok"
  | "error"
  | "cancelled"
  | "skipped";

/**
 * Live overlay aggregate. Same shape as
 * `@nube/starter-ui-flow`'s `RunOverlay` so callers can pass it
 * straight to `<FlowCanvas overlay={runOverlay} />` (with a single
 * cast at the call site to bridge the structural mirror).
 */
export interface FlowRunOverlay {
  nodes: Record<string, NodeRunState>;
  slotValues: Record<string, Record<string, unknown>>;
}

const EMPTY_OVERLAY: FlowRunOverlay = { nodes: {}, slotValues: {} };

export interface UseFlowEventsOptions {
  /** Maximum frames retained in `events`. Default 200. */
  bufferSize?: number;
  /** Pause subscription. Default `true`. */
  enabled?: boolean;
  /** Test seam — forwarded to `useEventStream`. */
  eventSourceCtor?: typeof EventSource;
  /** Test seam — forwarded to `useEventStream`. */
  forceFetch?: boolean;
}

export interface UseFlowEventsResult {
  /** Most-recent-last buffer of slot-value frames. Length ≤ `bufferSize`. */
  events: NodeSlotValue[];
  /** Most recent frame, if any. */
  latest: NodeSlotValue | null;
  /**
   * Aggregated `{ nodes, slotValues }` overlay ready to feed to
   * `<FlowCanvas overlay>`. Each incoming `NodeEmitted` marks its
   * node `"ok"` and stashes the latest value under
   * `slotValues[node][slot]`. Future `NodeStarted` / `NodeFailed`
   * frames promote to `"running"` / `"error"` respectively (the
   * server does not emit these yet — see module docstring).
   */
  runOverlay: FlowRunOverlay;
  status: EventStreamStatus;
  error: Error | null;
  /** Drop the buffer + overlay and reopen the connection. Stable identity. */
  reconnect(): void;
}

/**
 * Subscribe to the per-flow live event stream and produce a
 * runtime-overlay aggregate suitable for `<FlowCanvas>`.
 */
export function useFlowEvents(
  flowId: string,
  options: UseFlowEventsOptions = {},
): UseFlowEventsResult {
  const bufferSize = options.bufferSize ?? 200;
  const path = useMemo(() => flowEventsPath(flowId), [flowId]);
  const stream = useEventStream<NodeSlotValue>(path, {
    enabled: options.enabled,
    eventSourceCtor: options.eventSourceCtor,
    forceFetch: options.forceFetch,
  });

  const [events, setEvents] = useState<NodeSlotValue[]>([]);
  const [runOverlay, setRunOverlay] = useState<FlowRunOverlay>(EMPTY_OVERLAY);
  const lastFrameRef = useRef<NodeSlotValue | null>(null);

  // Reset accumulators when the flow id changes so a remount onto a
  // different flow doesn't show stale overlay state.
  useEffect(() => {
    setEvents([]);
    setRunOverlay(EMPTY_OVERLAY);
    lastFrameRef.current = null;
  }, [path]);

  // Accumulate frames as they arrive. `useEventStream` only surfaces
  // the latest snapshot, so we promote each new reference into our
  // own ring buffer + overlay.
  useEffect(() => {
    const frame = stream.data;
    if (!frame || frame === lastFrameRef.current) return;
    lastFrameRef.current = frame;
    setEvents((prev) => {
      const next =
        prev.length >= bufferSize ? prev.slice(prev.length - bufferSize + 1) : prev;
      return [...next, frame];
    });
    setRunOverlay((prev) => {
      const nodeSlots = prev.slotValues[frame.node] ?? {};
      return {
        nodes: { ...prev.nodes, [frame.node]: "ok" },
        slotValues: {
          ...prev.slotValues,
          [frame.node]: { ...nodeSlots, [frame.slot]: frame.value },
        },
      };
    });
  }, [stream.data, bufferSize]);

  const reconnect = () => {
    setEvents([]);
    setRunOverlay(EMPTY_OVERLAY);
    lastFrameRef.current = null;
    stream.reconnect();
  };

  return {
    events,
    latest: events.length > 0 ? events[events.length - 1]! : null,
    runOverlay,
    status: stream.status,
    error: stream.error,
    reconnect,
  };
}
