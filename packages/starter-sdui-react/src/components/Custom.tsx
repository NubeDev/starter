/**
 * `custom` — the escape hatch dispatch. The IR carries
 * `{ "type": "custom", "kind": "<plugin>", "props": {...},
 *    "subscribe": [...] }`; the renderer looks `kind` up in the
 * `customRegistry` and hands `props` + `subscribe` to the
 * registered component.
 *
 * The capability-handshake threat model (**R7**) means a custom
 * renderer that crashes or throws should not take the page down.
 * Unknown kinds render a neutral placeholder; the rendered
 * component itself is responsible for its own error boundary.
 */
import type { ComponentSpec } from "../registry/types.js";
import { useSdui } from "../context.js";
import type { UiComponent } from "../types.js";

export interface CustomNode extends UiComponent {
  type: "custom";
  /** Registry key — the same string a plugin passed to `registerCustomRenderer`. */
  kind: string;
  props?: unknown;
  /** Subjects the custom renderer subscribes to — keys into the
   *  subscription plan emitted by the resolver. */
  subscribe?: string[];
}

export const customSpec: ComponentSpec<CustomNode> = {
  kind: "custom",
  Component: ({ node }) => {
    const { customRegistry } = useSdui();
    const Comp = customRegistry.get(node.kind);
    if (!Comp) {
      return (
        <div className="rounded border border-dashed border-muted-foreground/40 px-3 py-2 text-xs text-muted-foreground">
          Unknown custom renderer: {node.kind}
        </div>
      );
    }
    return <Comp props={node.props} subscribe={node.subscribe ?? []} />;
  },
};
