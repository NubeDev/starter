/**
 * `custom` — the escape-hatch dispatch (**R7**).
 *
 * The IR carries `{ "type": "custom", "renderer_id": "<plugin>",
 * "props": {...}, "subscribe": [...] }`. `renderer_id` is the lookup
 * key into the client-side custom-renderer registry that consumers
 * populate via {@link registerCustomRenderer}.
 *
 * ## Falls back cleanly
 *
 * When the registry has no entry for `renderer_id` (the server's
 * capability filter should have rewritten unknown ids to a
 * `Component::Dangling` stub before emission — see
 * `starter-sdui-routes::capability` — but a tree that bypasses the
 * filter, or a client whose registry lost a registration after a
 * resolve, can still land here):
 *
 * 1. A neutral placeholder renders in place of the node. The rest of
 *    the tree continues to render — one unknown renderer never takes
 *    down the whole page.
 * 2. A **structured warning** is logged on first miss, naming the
 *    unknown `renderer_id` plus the node's optional `id`. The warning
 *    fires once per id per page session so a tree with N misses for
 *    the same id doesn't spam the console.
 *
 * ## Custom is a reference, not a node
 *
 * `Component::Custom` is *not* a new IR variant — it's a forward
 * reference to a client-side component the IR doesn't model.
 * `props` is opaque to the renderer: the IR does not type-check it,
 * the binding engine does not walk it, and the capability filter does
 * not authorise it. Authorisation of `props` is the
 * **handler / resolve boundary's** responsibility (see the threat
 * model in `starter-sdui-routes` crate-level docs).
 */
import { useEffect } from "react";
import type { ComponentSpec } from "../registry/types.js";
import { useSdui } from "../context.js";
import type { UiComponent } from "../types.js";

export interface CustomNode extends UiComponent {
  type: "custom";
  /**
   * Lookup key into the custom-renderer registry — the same string a
   * consumer passed to `registerCustomRenderer(id, Component)`.
   * Matches the IR field `Component::Custom { renderer_id }`.
   */
  renderer_id: string;
  props?: unknown;
  /** Subjects the custom renderer subscribes to — keys into the
   *  subscription plan emitted by the resolver. */
  subscribe?: string[];
}

/** One-shot guard so the structured warning for an unknown id fires
 *  exactly once per process lifetime per id. The Set is module-scoped
 *  — the warning is a developer aid, not a metric. */
const _warnedIds = new Set<string>();

export const customSpec: ComponentSpec<CustomNode> = {
  kind: "custom",
  Component: ({ node }) => {
    const { customRegistry } = useSdui();
    const rendererId = node.renderer_id;
    const Comp = rendererId ? customRegistry.get(rendererId) : undefined;

    // Fire the structured warning from an effect so server-side
    // rendering and test runs that mount/unmount in a tight loop
    // still get exactly-once behaviour. Effects run only on the
    // client, but the guard `_warnedIds` is the actual dedup.
    useEffect(() => {
      if (!Comp && rendererId && !_warnedIds.has(rendererId)) {
        _warnedIds.add(rendererId);
        // Structured payload — log aggregators key off `event`.
        // eslint-disable-next-line no-console
        console.warn("sdui.custom.unknown_renderer", {
          event: "sdui.custom.unknown_renderer",
          renderer_id: rendererId,
          component_id: node.id,
        });
      }
    }, [Comp, rendererId, node.id]);

    if (!Comp) {
      return (
        <div
          data-sdui-custom-stub={rendererId ?? "<missing>"}
          className="rounded border border-dashed border-muted-foreground/40 px-3 py-2 text-xs text-muted-foreground"
        >
          Unknown custom renderer: {rendererId ?? "<missing renderer_id>"}
        </div>
      );
    }
    return <Comp props={node.props} subscribe={node.subscribe ?? []} />;
  },
};

/** Test-only — clear the once-per-id warning guard. Not exported
 *  from the package barrel. */
export function __resetCustomWarningCacheForTests(): void {
  _warnedIds.clear();
}
