export const EXTENSION_ID = "com.nubeio.ce";

export interface WarehouseQueryResponse<R = Record<string, unknown>> {
  template: string;
  rows: ReadonlyArray<R>;
  count: number;
}

/** Generic write result returned by the device CRUD + engine-push tools. */
export interface WriteResult {
  operation: string;
  affected: number;
}

// -----------------------------------------------------------------
// Control-engine catalog (`ce_devices`).
// -----------------------------------------------------------------

/** A registered control engine. `password` is never returned by the
 *  list read — only the server-side engine proxy ever sees it. */
export interface DeviceRow {
  device_id: string;
  name: string | null;
  description: string | null;
  engine_kind: string | null;
  ip: string;
  port: number;
  username: string | null;
  status: string | null;
  created_at: string | null;
  updated_at: string | null;
}

/** Input shape for `device_create` / `device_update`. */
export interface DeviceInput {
  device_id: string;
  name?: string;
  description?: string;
  engine_kind?: string;
  ip?: string;
  port?: number;
  username?: string;
  password?: string;
  status?: string;
}

// -----------------------------------------------------------------
// Wiresheet — node/edge graph mirrored from `engine_wiresheet_*`.
// Shapes map onto `@nube/starter-ui-flow`'s BaseNode vocabulary.
// -----------------------------------------------------------------

export interface WiresheetNode {
  id: string;
  /** Node-kind id in the BaseNode registry. */
  kind: string;
  label?: string;
  position?: { x: number; y: number };
  config?: Record<string, unknown>;
}

export interface WiresheetEdge {
  id: string;
  source: string;
  target: string;
  sourceSlot?: string;
  targetSlot?: string;
}

export interface WiresheetGraph {
  device_id: string;
  nodes: WiresheetNode[];
  edges: WiresheetEdge[];
}

export interface ExtensionDetail {
  id: string;
  enabled: string;
  state: string;
  manifest: {
    id?: string;
    version?: string;
    contributes?: {
      tools?: ReadonlyArray<{ id: string }>;
      warehouse_tables?: ReadonlyArray<{ name: string }>;
      warehouse_templates?: ReadonlyArray<{ name: string }>;
      ui?: { entry: string; exposes?: ReadonlyArray<{ slot: string }> };
    };
  } | null;
}

/** Postgres NUMERIC/INT may arrive as string — coerce to number. */
export function asNumber(v: unknown): number | null {
  if (v === null || v === undefined || v === "") return null;
  const n = typeof v === "number" ? v : Number(v);
  return Number.isFinite(n) ? n : null;
}
