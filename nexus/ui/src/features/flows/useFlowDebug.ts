import { useEffect, useRef, useState } from "react";
import { streamJson } from "@nube/starter-client-ts";
import { useStarterClient } from "@nube/starter-client-react";

import { disableFlowDebug, enableFlowDebug } from "@/api/flows/debug";
import type {
  FlowDebugEvent,
  LogLevel,
  NodeCounters,
  NodeRole,
} from "@/api/types";

// Per-node sample rows kept in view. A debug sample is a sliding window, not a
// full capture — old rows fall off as fresh batches arrive.
const SAMPLE_WINDOW = 50;
// Log lines kept in the ring; newest first when rendered.
const LOG_WINDOW = 200;

export interface NodeDebug {
  nodeIndex: number;
  role: NodeRole;
  counters?: NodeCounters;
  // Most recent sample rows that crossed this node's boundary.
  rows: Record<string, unknown>[];
}

export interface DebugLogLine {
  seq: number;
  level: LogLevel;
  nodeIndex?: number;
  message: string;
  atMs: number;
}

export type DebugConnection = "off" | "connecting" | "live" | "error";

export interface FlowDebugState {
  connection: DebugConnection;
  // Per-node debug keyed by node index (0 = source … N+1 = sink).
  byNode: Map<number, NodeDebug>;
  logs: DebugLogLine[];
  nodeCount: number;
  error?: string;
}

const EMPTY: FlowDebugState = {
  connection: "off",
  byNode: new Map(),
  logs: [],
  nodeCount: 0,
};

// Subscribes to a running flow's debug stream while `active` is true: enables
// capture (minting the SSE token), opens the `EventSource`, and reduces the
// `FlowDebugEvent` union into per-node counters + sample rows and a log ring.
// On teardown it disables capture so the flow stops sampling. The token rides
// the URL because `EventSource` can't send an Authorization header — the same
// not-Bearer path as live query streams.
export function useFlowDebug(
  flowId: string | null,
  active: boolean,
): FlowDebugState {
  const client = useStarterClient();
  const [state, setState] = useState<FlowDebugState>(EMPTY);
  // Mutable accumulators so a burst of events doesn't re-run the effect; we
  // publish snapshots into React state per event.
  const byNode = useRef<Map<number, NodeDebug>>(new Map());
  const logs = useRef<DebugLogLine[]>([]);

  useEffect(() => {
    if (!flowId || !active) {
      setState(EMPTY);
      return;
    }
    const ctrl = new AbortController();
    byNode.current = new Map();
    logs.current = [];
    setState({ ...EMPTY, connection: "connecting" });

    (async () => {
      let enabled = false;
      try {
        const minted = await enableFlowDebug(client, flowId);
        enabled = true;
        setState((s) => ({
          ...s,
          connection: "live",
          nodeCount: minted.node_count,
        }));

        for await (const event of streamJson<FlowDebugEvent>(
          client,
          minted.stream_url,
          { signal: ctrl.signal },
        )) {
          reduce(byNode.current, logs.current, event);
          setState((s) => ({
            ...s,
            connection: "live",
            byNode: new Map(byNode.current),
            logs: [...logs.current],
          }));
        }
      } catch (err) {
        if (ctrl.signal.aborted) return; // unmount / toggle off, not an error
        setState((s) => ({
          ...s,
          connection: "error",
          error: err instanceof Error ? err.message : String(err),
        }));
      } finally {
        // Best-effort: stop sampling server-side when we detach. Ignore errors
        // (the flow may already have stopped, dropping the channel).
        if (enabled) void disableFlowDebug(client, flowId).catch(() => {});
      }
    })();

    return () => ctrl.abort();
  }, [client, flowId, active]);

  return state;
}

// Fold one event into the accumulators in place.
function reduce(
  byNode: Map<number, NodeDebug>,
  logs: DebugLogLine[],
  event: FlowDebugEvent,
): void {
  switch (event.kind) {
    case "counters": {
      const node = ensureNode(byNode, event.node_index, event.role);
      node.counters = {
        node_index: event.node_index,
        role: event.role,
        rows_in: event.rows_in,
        rows_out: event.rows_out,
        batches: event.batches,
      };
      break;
    }
    case "sample": {
      const node = ensureNode(byNode, event.node_index, event.role);
      const incoming = event.rows as Record<string, unknown>[];
      node.rows = [...incoming, ...node.rows].slice(0, SAMPLE_WINDOW);
      break;
    }
    case "log": {
      logs.unshift({
        seq: event.seq,
        level: event.level,
        nodeIndex: event.node_index ?? undefined,
        message: event.message,
        atMs: event.at_ms,
      });
      if (logs.length > LOG_WINDOW) logs.length = LOG_WINDOW;
      break;
    }
  }
}

function ensureNode(
  byNode: Map<number, NodeDebug>,
  nodeIndex: number,
  role: NodeRole,
): NodeDebug {
  let node = byNode.get(nodeIndex);
  if (!node) {
    node = { nodeIndex, role, rows: [] };
    byNode.set(nodeIndex, node);
  }
  return node;
}
