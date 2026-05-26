// `usePageLiveness` — per-page consumer of the tenant-scoped dashboard
// SSE stream emitted by `rubix-agent` at `GET /api/v1/dashboards/events`.
// Implements §B1 of `rubix/docs/scope/dashboards/11-live-canvas-sse.md`:
// reuses the same `useEventStream` wrapper the sidebar (scope 09) uses
// and filters frames client-side by `page_id`. This keeps a single
// EventSource open per browser tab regardless of how many surfaces
// (sidebar, editor canvas, read route) subscribe.
//
// What this hook does NOT do:
//   - fetch the body. Consumers decide what to do on a change (the
//     editor shows a banner; a future read route re-resolves).
//   - validate the body. Wire is authoritative by construction
//     (server-side schema check at write time — see scope §B6).
//   - debounce. v1 keeps fan-out simple; multi-step AI runs may
//     bump `changeToken` more than once per visible change. Open
//     question Q5 in the scope doc tracks the v1.5 hardening.

import { useEffect, useRef, useState } from "react";

import {
  useEventStream,
  type EventStreamStatus,
} from "@nube/starter-client-react";

import type { DashboardSidebarFrame } from "./use-dashboard-sidebar.js";

const DASHBOARD_EVENTS_PATH = "/api/v1/dashboards/events";

export interface UsePageLivenessOptions {
  /** Pause subscription. Default `true`. */
  enabled?: boolean;
  /** Test seam — forwarded to `useEventStream`. */
  eventSourceCtor?: typeof EventSource;
  /** Test seam — forwarded to `useEventStream`. */
  forceFetch?: boolean;
}

export interface UsePageLivenessResult {
  /** Last revision id the server announced for this `pageRef`. */
  latestRevisionId: string | undefined;
  /** True while the SSE channel is connected. */
  connected: boolean;
  /** Bumps every time the server emits a `created` / `updated` /
   *  `deleted` frame matching `pageRef`. Consumers `useEffect` on
   *  this token to react to changes — `latestRevisionId` alone is
   *  not enough because a delete frame carries no revision id. */
  changeToken: number;
  /** Underlying SSE status — exposed mostly so the editor can hint
   *  "live updates unavailable" if the channel is permanently broken. */
  status: EventStreamStatus;
  /** Optional `actor_kind` from the most recent matching frame. The
   *  current server wire does not yet carry this; scope §B4 adds it
   *  as an additive enrichment, and we surface it so the editor can
   *  branch copy ("AI updated this page" vs "Operator updated"). */
  actorKind?: "operator" | "ai" | "system";
}

// Loose-typed view of the wire frame with the §B4-extended fields.
// The base `DashboardSidebarFrame` doesn't carry these yet; we accept
// either shape so the hook works against today's server *and* a future
// enriched one without a wire migration.
type LiveFrame =
  | DashboardSidebarFrame
  | { kind: "snapshot"; items: Array<{ page_id: string; revision_id?: string }> }
  | {
      kind: "created" | "updated";
      page_id: string;
      revision_id?: string;
      tenant_id?: string;
      actor_kind?: "operator" | "ai" | "system";
    }
  | { kind: "deleted"; page_id: string; tenant_id?: string; actor_kind?: "operator" | "ai" | "system" };

/** Subscribe to per-page liveness for a single dashboard `pageRef`. */
export function usePageLiveness(
  pageRef: string,
  options: UsePageLivenessOptions = {},
): UsePageLivenessResult {
  const stream = useEventStream<LiveFrame>(DASHBOARD_EVENTS_PATH, {
    enabled: options.enabled,
    eventSourceCtor: options.eventSourceCtor,
    forceFetch: options.forceFetch,
  });

  const [latestRevisionId, setLatestRevisionId] = useState<string | undefined>(
    undefined,
  );
  const [changeToken, setChangeToken] = useState(0);
  const [actorKind, setActorKind] = useState<
    "operator" | "ai" | "system" | undefined
  >(undefined);
  const lastFrameRef = useRef<LiveFrame | null>(null);

  useEffect(() => {
    const frame = stream.data;
    if (!frame || frame === lastFrameRef.current) return;
    lastFrameRef.current = frame;

    switch (frame.kind) {
      case "snapshot": {
        // Seed `latestRevisionId` so the editor knows what the server
        // currently considers authoritative for this page even before
        // any delta arrives. Snapshots do NOT bump `changeToken` —
        // they're a baseline, not an event.
        const item = frame.items.find((it) => it.page_id === pageRef);
        if (item?.revision_id) setLatestRevisionId(item.revision_id);
        return;
      }
      case "created":
      case "updated": {
        if (frame.page_id !== pageRef) return;
        if (frame.revision_id) setLatestRevisionId(frame.revision_id);
        // `actor_kind` is additive (scope §B4). Older servers omit it;
        // we just leave the prior value unchanged in that case so the
        // editor can render a neutral "page updated" copy.
        const ak = (frame as { actor_kind?: "operator" | "ai" | "system" })
          .actor_kind;
        if (ak) setActorKind(ak);
        setChangeToken((n) => n + 1);
        return;
      }
      case "deleted": {
        if (frame.page_id !== pageRef) return;
        const ak = (frame as { actor_kind?: "operator" | "ai" | "system" })
          .actor_kind;
        if (ak) setActorKind(ak);
        // A delete leaves `latestRevisionId` pointing at the last known
        // revision — there is no successor — but bumps the change
        // token so the editor can render the "page was deleted" path
        // (scope open question Q3).
        setChangeToken((n) => n + 1);
        return;
      }
    }
  }, [stream.data, pageRef]);

  return {
    latestRevisionId,
    connected: stream.status === "open",
    changeToken,
    status: stream.status,
    actorKind,
  };
}
