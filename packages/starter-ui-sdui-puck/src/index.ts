// # @nube/starter-ui-sdui-puck
//
// Visual SDUI editor built on Puck. Pairs with
// `@nube/starter-ui-sdui-react` (read-side renderers); both surfaces
// emit / consume the same `ComponentTree` JSON. Scope:
// rubix/docs/scope/dashboards/10-puck-builder.md.

export { buildPuckConfig, type BuildPuckConfigOpts } from "./build-puck-config.js";
export { PuckBuilder, type PuckBuilderProps, type PuckBuilderHandle } from "./builder.js";
export { IR_SCHEMA } from "./schema-loader.js";
export { IR_SCHEMA_HASH, IR_SCHEMA_HASH_ALGORITHM } from "./schema-hash.js";

// IR ↔ Puck Data adapter (§B4). Exposed so the harness and the
// rubix frontend route can build / inspect either shape.
export {
  componentTreeToPuckData,
  puckDataToComponentTree,
  IR_VERSION,
  type ComponentTree,
  type IrNode,
} from "./adapter.js";

// Save seam (§B4).
export {
  makeRubixSaveTransport,
  type DashboardUpdateLikeClient,
  type PuckSaveRequest,
  type PuckSaveOutcome,
  type PuckSaveTransport,
} from "./save.js";

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
export {
  DATA_SOURCES,
  dataSourceKindOf,
  type CatalogueKind,
  type DataSourceTuple,
} from "./curation/data-sources.js";

// §B3 catalogue seam — the package stays rubix-agnostic; consumers
// supply a `Catalogue` via `<CatalogueProvider>` mounted around
// `<PuckBuilder>`.
export {
  CatalogueProvider,
  useCatalogue,
  catalogueFromMap,
  DataSourceField,
  makeDataSourceField,
  type Catalogue,
  type CatalogueEntry,
  type CatalogueProviderProps,
} from "./data-source-field.js";

export type {
  PuckConfigStub,
  PuckComponentConfigStub,
  PuckFieldStub,
} from "./puck-types.js";
