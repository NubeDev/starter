// # @nube/starter-ui-sdui-puck
//
// Visual SDUI editor built on Puck. Pairs with
// `@nube/starter-ui-sdui-react` (read-side renderers); both surfaces
// emit / consume the same `ComponentTree` JSON. Scope:
// rubix/docs/scope/dashboards/10-puck-builder.md.

export { buildPuckConfig, type BuildPuckConfigOpts } from "./build-puck-config.js";
export { PuckBuilder, type PuckBuilderProps } from "./builder.js";
export { IR_SCHEMA } from "./schema-loader.js";

// Curated companion tables — exported so consumers (tests, harness,
// future runtime catalogue verb) can read them without re-deriving.
export { SLOTS, isSlot, type SlotTuple } from "./curation/slots.js";
export {
  OVERRIDES,
  RESOLVER_ONLY_VARIANTS,
} from "./curation/overrides.js";
export {
  BINDABLE,
  isBindable,
  type BindableTuple,
} from "./curation/bindable.js";
export {
  PALETTE_TAXONOMY,
  categoryFor,
  type PaletteCategory,
} from "./curation/palette-taxonomy.js";

export type {
  PuckConfigStub,
  PuckComponentConfigStub,
  PuckFieldStub,
} from "./puck-types.js";
