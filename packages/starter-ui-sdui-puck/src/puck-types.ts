// Narrow structural mirrors of `@measured/puck`'s Config / Field /
// ComponentConfig types. We re-export the real types from
// `@measured/puck` so consumers get the canonical shapes, but the
// generator only relies on the structural subset captured here.
//
// Why a shim: the generator is pure data-in / data-out and shouldn't
// pull React types into the test-time module graph; tests can build
// configs without instantiating Puck. The real `Config` type from
// `@measured/puck` is assignable from this shape.

import type { ComponentType } from "react";

/** Subset of Puck's field union we emit in PR1. */
export type PuckFieldStub =
  | { type: "text" }
  | { type: "textarea" }
  | { type: "number" }
  | { type: "select"; options: { label: string; value: string | number }[] }
  | { type: "radio"; options: { label: string; value: string | number | boolean }[] }
  | { type: "array"; arrayFields: Record<string, PuckFieldStub>; defaultItemProps?: Record<string, unknown> }
  | { type: "object"; objectFields: Record<string, PuckFieldStub> }
  | { type: "slot" }
  // `external`/`custom` selector fields land in PR2 (B3 data-source
  // selectors). PR1 falls through to plain text for `$ref`-typed
  // leaves; this variant is reserved so consumers can pattern-match.
  | {
      type: "custom";
      render: (props: {
        name: string;
        value: unknown;
        onChange: (v: unknown) => void;
      }) => unknown;
      /**
       * Optional tag used by the §B3 catalogue-backed pickers so
       * tests/devtools can identify which catalogue kind a custom
       * field is bound to. Puck itself ignores extra keys.
       */
      catalogueKind?: string;
    };

/** Subset of Puck's `ComponentConfig` the generator emits. */
export interface PuckComponentConfigStub {
  /** Field schema. */
  fields: Record<string, PuckFieldStub>;
  /** Default props for a freshly dropped instance. */
  defaultProps?: Record<string, unknown>;
  /** Renderer. PR1 emits a stringify placeholder; PR3 wires real renderers. */
  render: ComponentType<Record<string, unknown>>;
  /** Optional UI label override; defaults to the snake-case variant name. */
  label?: string;
}

/** Subset of Puck's `Config` the generator emits. */
export interface PuckConfigStub {
  components: Record<string, PuckComponentConfigStub>;
  categories?: Record<
    string,
    { title?: string; components: string[] }
  >;
  root?: PuckComponentConfigStub;
}
