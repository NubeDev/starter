// `PuckBuilder` save seam — keeps the package rubix-agnostic.
//
// Per scope §B4 the editor writes via `rubix.dashboard.update`, but
// the actual call lives in the consumer (the rubix frontend route
// — §B5 — wires this seam to `client.dashboardUpdate`). The package
// owns the *UX contract* (Save button, 409 conflict modal,
// loading state) and delegates the *transport* to the host.
//
// Why the indirection: `@nube/starter-ui-sdui-puck` lives under
// `packages/`, which the starter promises never depends on
// `rubix-*`. The seam preserves that boundary.

import type { ComponentTree } from "./adapter.js";

/** What `PuckBuilder` hands the consumer's `onSave` callback. */
export interface PuckSaveRequest {
  /** `"dashboard.<slug>"` — echoed from `PuckBuilder.pageRef`. */
  pageRef: string;
  /** IR `ComponentTree` derived from the live Puck `Data`. */
  body: ComponentTree;
  /** Optimistic-concurrency token from the loader. `undefined`
   *  means the operator is creating a brand-new page (no prior
   *  revision to guard against) — most consumers will refuse that
   *  in their `onSave` and surface a "create instead" path. */
  expectedRevisionId?: string;
  /** Replacement title pulled from `data.root.props.title`. May be
   *  absent if the operator hasn't authored one. */
  title?: string;
  /** Replacement tag list, when authored. */
  tags?: string[];
}

/** Discriminated outcome returned by the consumer's `onSave`.
 *
 *  - `saved`: write went through; the editor adopts the new
 *    `revision_id` so the next save guards against it.
 *  - `conflict`: the server's live revision no longer matches
 *    `expectedRevisionId` (HTTP 409 with diagnostic
 *    `rubix.dashboard.update.conflict`). The editor shows the
 *    Discard / Keep editing modal.
 *  - `error`: every other failure (network, 4xx, 5xx). The editor
 *    surfaces `message` in an inline banner; the operator's tree
 *    is untouched so they can retry. */
export type PuckSaveOutcome =
  | { kind: "saved"; revisionId: string }
  | { kind: "conflict"; currentRevisionId: string; message?: string }
  | { kind: "error"; message: string };

/** Consumer-supplied save transport. Must always resolve — throwing
 *  is treated as `{ kind: "error", message: e.message }`. */
export type PuckSaveTransport = (req: PuckSaveRequest) => Promise<PuckSaveOutcome>;

/** Convenience: rubix-frontend (or any consumer) can build a
 *  transport that calls `rubix.dashboard.update` and maps the
 *  result into a `PuckSaveOutcome`. The package exposes this as
 *  a type, not an implementation, so the rubix wiring lives next
 *  to the route in §B5 where the auth / cookie context is set up. */
export interface DashboardUpdateLikeClient {
  dashboardUpdate(req: {
    page_id: string;
    expected_revision_id: string;
    title?: string;
    tags?: string[];
    body_json: unknown;
  }): Promise<{
    summary: { code: string };
    page_id: string;
    revision_id: string;
  }>;
}

/** Build a `PuckSaveTransport` from a rubix-client-shaped object.
 *  Optional helper for consumers; the seam itself doesn't require it. */
export function makeRubixSaveTransport(
  client: DashboardUpdateLikeClient,
): PuckSaveTransport {
  return async (req) => {
    if (!req.expectedRevisionId) {
      return {
        kind: "error",
        message:
          "Cannot save without an expected_revision_id — load the page via rubix.dashboard.get first, or use rubix.dashboard.create for a brand-new page.",
      };
    }
    try {
      const res = await client.dashboardUpdate({
        page_id: req.pageRef,
        expected_revision_id: req.expectedRevisionId,
        title: req.title,
        tags: req.tags,
        body_json: req.body as unknown,
      });
      return { kind: "saved", revisionId: res.revision_id };
    } catch (e) {
      // The rubix client throws `StarterError` with a `.status` and
      // `.problem.title`. The 409 path encodes the live revision id
      // in the diagnostic title — `"rubix.dashboard.update.conflict:
      // page_id=X current_revision_id=Y"` — so we parse it out here.
      const err = e as {
        status?: number;
        message?: string;
        problem?: { title?: string; detail?: string };
      };
      if (err.status === 409) {
        const text = err.problem?.title ?? err.message ?? "";
        const match = text.match(/current_revision_id=([^\s,;]+)/);
        return {
          kind: "conflict",
          currentRevisionId: match ? match[1]! : "",
          message: text,
        };
      }
      return {
        kind: "error",
        message: err.message ?? String(e),
      };
    }
  };
}
