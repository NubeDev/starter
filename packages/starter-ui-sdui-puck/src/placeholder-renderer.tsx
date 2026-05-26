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

import {
  createElement,
  Fragment,
  isValidElement,
  type ComponentType,
  type ReactNode,
} from "react";
import type { UiComponent } from "@nube/starter-ui-ir";
import { PlaceholderRender } from "@nube/starter-ui-sdui-react/headless";

import { SLOTS } from "./curation/slots.js";

// Puck 0.19 hands slot props to render functions as React
// components (a `<Children/>`-style render fn), NOT as arrays of
// nodes the way the IR dispatcher expects. We split the props into:
//   * `slotElements` — rendered via Puck's slot component so dropped
//     children appear inside the layout container in the canvas.
//   * `irRest` — scalar / non-slot props that round-trip into the
//     IR-shaped node the placeholder visualiser consumes.
export function makePlaceholderRenderer(
  variant: string,
): ComponentType<Record<string, unknown>> {
  const slotNames = new Set(
    SLOTS.filter((s) => s.variant === variant).map((s) => s.propertyPath),
  );

  const Placeholder = (props: Record<string, unknown>) => {
    const { id: _id, editMode: _em, puck: _p, ...rest } = props as Record<
      string,
      unknown
    >;
    void _id;
    void _em;
    void _p;

    const slotElements: Record<string, ReactNode> = {};
    const irRest: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(rest)) {
      if (slotNames.has(key) && typeof value === "function") {
        const SlotComponent = value as ComponentType;
        slotElements[key] = createElement(SlotComponent);
      } else if (
        slotNames.has(key) &&
        isValidElement(value as unknown as ReactNode)
      ) {
        slotElements[key] = value as ReactNode;
      } else {
        irRest[key] = value;
      }
    }

    const node: UiComponent = { type: variant, ...irRest } as UiComponent;
    return createElement(
      "div",
      {
        "data-puck-placeholder": variant,
        style: { margin: "0.25rem 0" },
      },
      createElement(PlaceholderRender, { node }),
      ...Object.entries(slotElements).map(([key, el]) =>
        createElement(
          Fragment,
          { key: `slot:${key}` },
          el,
        ),
      ),
    );
  };
  Placeholder.displayName = `PuckPlaceholder(${variant})`;
  return Placeholder;
}
