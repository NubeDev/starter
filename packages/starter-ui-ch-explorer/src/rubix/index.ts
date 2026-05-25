// Rubix overlays for the ClickHouse explorer. These components
// depend on the rubix-agent surfaces (warehouse REST status,
// `rubix.clickhouse.*` verbs) and are intended for hosts that
// mount `<RubixClientProvider>`. The demo binary
// (`examples/ch-explorer`) does not mount these.

export {
  FreshnessTiles,
  type FreshnessTilesProps,
  type FreshnessTilesMessages,
  type WarehouseStatus,
} from "./freshness-tiles.js";
export {
  MartTree,
  type MartTreeProps,
  type MartTreeMessages,
} from "./mart-tree.js";
