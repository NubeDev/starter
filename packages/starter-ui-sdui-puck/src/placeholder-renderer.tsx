// Placeholder renderer used by the generated `PuckConfig`. Each
// palette tile / canvas node dispatches through this. PR2 (scope
// §B2) wired this to delegate to `<PlaceholderRender>` from
// `@nube/starter-ui-sdui-react/headless`, which renders the same
// per-variant placeholder visuals the live renderer uses when its
// transport returns empty (kpi / chart / table / kpi_grid /
// repeat / form) — so the canvas is visually faithful to runtime
// without spinning up a transport.
//
// The Puck-internal props (`id`, `editMode`, `puck`) are stripped
// before building the IR node so the placeholder sees only the
// author-edited subset. Variants without a registered renderer or
// placeholder filler degrade to the dashed "variant tile" emitted
// by `<PlaceholderRender>` itself — visible breakage, not silent
// drop.

import { createElement, type ComponentType } from "react";
import type { UiComponent } from "@nube/starter-ui-ir";
import { PlaceholderRender } from "@nube/starter-ui-sdui-react/headless";

export function makePlaceholderRenderer(
  variant: string,
): ComponentType<Record<string, unknown>> {
  const Placeholder = (props: Record<string, unknown>) => {
    const { id: _id, editMode: _em, puck: _p, ...rest } = props as Record<
      string,
      unknown
    >;
    void _id;
    void _em;
    void _p;
    // Reconstruct the IR-shaped node from the Puck props. Slot
    // fields arrive as `UiComponent[]`, array fields as `object[]`,
    // scalar fields as primitives — exactly the IR shape the
    // dispatcher expects.
    const node: UiComponent = { type: variant, ...rest } as UiComponent;
    return createElement(
      "div",
      {
        "data-puck-placeholder": variant,
        style: { margin: "0.25rem 0" },
      },
      createElement(PlaceholderRender, { node }),
    );
  };
  Placeholder.displayName = `PuckPlaceholder(${variant})`;
  return Placeholder;
}
