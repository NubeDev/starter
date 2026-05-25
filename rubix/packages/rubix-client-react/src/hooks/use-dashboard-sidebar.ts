// `useDashboardSidebar` — live, tenant-scoped view of the dashboard
// pages the rubix-frontend sidebar renders. Backed by the SSE route
// `GET /api/v1/dashboards/events` (see
// `rubix/crates/rubix-agent/src/routes/dashboard_events.rs` and
// `rubix/docs/scope/dashboards/09-live-sidebar-sse.md`).
//
// Wire shape — one default `data:` frame per change, discriminated
// on `kind`. The first frame is always a `snapshot` carrying the
// current page list so a fresh connect (or any auto-reconnect) is
// authoritative; subsequent frames are `created` / `updated` /
// `deleted` deltas keyed on `page_id`. The reducer below is
// idempotent on `page_id` so a duplicate `created` from
// `snapshot ∪ tail` collapses cleanly.
//
// Reconnect semantics are inherited from `useEventStream` (which
// wraps `streamJson` from `@nube/starter-client-ts`): EventSource's
// built-in retry kicks in on transient errors, and every reconnect
// re-emits the snapshot frame so the reducer cannot drift even
// when `LISTEN/NOTIFY` drops a packet.

import { useEffect, useMemo, useReducer, useRef } from "react";

import {
  useEventStream,
  type EventStreamStatus,
} from "@nube/starter-client-react";

/** Path mounted by rubix-agent — see route docs at file top. */
const DASHBOARD_EVENTS_PATH = "/api/v1/dashboards/events";

/** One sidebar entry. Matches the server's `SnapshotItem` shape. */
export interface DashboardSidebarItem {
  page_id: string;
  title: string;
  /** Optional — present for snapshot rows and most deltas. */
  revision_id?: string;
  tags?: string[];
}

/** Wire frame surfaced by the server SSE route. */
export type DashboardSidebarFrame =
  | { kind: "snapshot"; items: DashboardSidebarItem[] }
  | {
      kind: "created";
      page_id: string;
      title: string;
      revision_id?: string;
      tenant_id: string;
    }
  | {
      kind: "updated";
      page_id: string;
      title?: string;
      revision_id?: string;
      tenant_id: string;
    }
  | { kind: "deleted"; page_id: string; tenant_id: string };

export interface UseDashboardSidebarOptions {
  /** Pause subscription. Default `true`. */
  enabled?: boolean;
  /** Test seam — forwarded to `useEventStream`. */
  eventSourceCtor?: typeof EventSource;
  /** Test seam — forwarded to `useEventStream`. */
  forceFetch?: boolean;
}

export interface UseDashboardSidebarResult {
  /** Current sidebar list. Title-sorted, stable identity per item. */
  items: DashboardSidebarItem[];
  /** Underlying EventSource status. */
  status: EventStreamStatus;
  error: Error | null;
  /** Drop the cached list and reopen the stream. */
  reconnect(): void;
}

interface State {
  /** Keyed on `page_id` so reducers are O(1) per delta. */
  byId: Record<string, DashboardSidebarItem>;
  /** Snapshot frames replace `byId`; we keep a generation so a
   *  reconnect's snapshot wins over any in-flight delta the client
   *  hasn't processed yet. */
  generation: number;
}

function reduce(state: State, frame: DashboardSidebarFrame): State {
  switch (frame.kind) {
    case "snapshot": {
      const byId: Record<string, DashboardSidebarItem> = {};
      for (const it of frame.items) byId[it.page_id] = it;
      return { byId, generation: state.generation + 1 };
    }
    case "created":
    case "updated": {
      const prev = state.byId[frame.page_id];
      const next: DashboardSidebarItem = {
        page_id: frame.page_id,
        title: frame.title ?? prev?.title ?? "",
        revision_id: frame.revision_id ?? prev?.revision_id,
        tags: prev?.tags,
      };
      return { ...state, byId: { ...state.byId, [frame.page_id]: next } };
    }
    case "deleted": {
      if (!(frame.page_id in state.byId)) return state;
      const { [frame.page_id]: _gone, ...rest } = state.byId;
      void _gone;
      return { ...state, byId: rest };
    }
  }
}

const INITIAL: State = { byId: {}, generation: 0 };

/** Subscribe to the live dashboard sidebar feed. */
export function useDashboardSidebar(
  options: UseDashboardSidebarOptions = {},
): UseDashboardSidebarResult {
  const stream = useEventStream<DashboardSidebarFrame>(DASHBOARD_EVENTS_PATH, {
    enabled: options.enabled,
    eventSourceCtor: options.eventSourceCtor,
    forceFetch: options.forceFetch,
  });

  const [state, dispatch] = useReducer(reduce, INITIAL);
  const lastFrameRef = useRef<DashboardSidebarFrame | null>(null);

  // `useEventStream` surfaces the latest frame as a stable reference;
  // promote each new reference into the reducer so the items list
  // stays in sync without re-running on every render.
  useEffect(() => {
    const frame = stream.data;
    if (!frame || frame === lastFrameRef.current) return;
    lastFrameRef.current = frame;
    dispatch(frame);
  }, [stream.data]);

  const items = useMemo(() => {
    return Object.values(state.byId).sort((a, b) =>
      a.title.localeCompare(b.title, undefined, { sensitivity: "base" }),
    );
  }, [state.byId]);

  return {
    items,
    status: stream.status,
    error: stream.error,
    reconnect: stream.reconnect,
  };
}
