export const EXTENSION_ID = "com.nubeio.rubixos";

export interface WarehouseQueryResponse<R = Record<string, unknown>> {
  template: string;
  rows: ReadonlyArray<R>;
  count: number;
}

export interface PointRow {
  uuid: string;
  name: string | null;
  description: string | null;
  device_uuid: string | null;
  device_name: string | null;
  network_uuid: string | null;
  network_name: string | null;
  host_uuid: string;
  host_name: string | null;
}

export interface DeviceOverviewRow {
  device_uuid: string;
  device_name: string | null;
  device_description: string | null;
  network_uuid: string | null;
  network_name: string | null;
  host_uuid: string | null;
  host_name: string | null;
  point_count: number;
}

export interface NetworkOverviewRow {
  network_uuid: string;
  network_name: string | null;
  network_description: string | null;
  host_uuid: string | null;
  host_name: string | null;
  device_count: number;
  point_count: number;
}

export interface HostOverviewRow {
  host_uuid: string;
  host_name: string | null;
  host_description: string | null;
  network_count: number;
  device_count: number;
  point_count: number;
}

export interface HistoriesSummaryRow {
  sample_count: number;
  point_count: number;
  earliest: string | null;
  latest: string | null;
}

export interface HistoryRecentRow {
  timestamp: string;
  value: number | string | null;
  host_uuid: string;
}

export interface HistoryBucketRow {
  bucket: string;
  min_value: number | null;
  max_value: number | null;
  avg_value: number | null;
  sample_count: number;
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

/** Convert a numeric/string field (Postgres NUMERIC arrives as string) to a number. */
export function asNumber(v: unknown): number | null {
  if (v === null || v === undefined || v === "") return null;
  const n = typeof v === "number" ? v : Number(v);
  return Number.isFinite(n) ? n : null;
}

/** Convert a warehouse-template TIMESTAMPTZ cell to epoch milliseconds.
 *  The contributed-template bridge encodes TIMESTAMPTZ as a JSON number
 *  (`timestamp_millis`), but defensively accept ISO strings too. */
export function asEpochMs(v: unknown): number | null {
  if (v === null || v === undefined) return null;
  if (typeof v === "number") return Number.isFinite(v) ? v : null;
  if (typeof v === "string") {
    const t = Date.parse(v);
    return Number.isFinite(t) ? t : null;
  }
  return null;
}

// -----------------------------------------------------------------
// Dashboard ("Energy & Water Overview") row shapes.
// -----------------------------------------------------------------

export type MeterKind = "elec" | "water";

export interface MeterRow {
  uuid: string;
  name: string | null;
  device_uuid: string | null;
  device_name: string | null;
  network_uuid: string | null;
  network_name: string | null;
  host_uuid: string;
  host_name: string | null;
  unit: string | null;
}

export interface UsageSiteTotalRow {
  host_uuid: string;
  host_name: string | null;
  total_value: number | string | null;
  point_count: number | string;
  sample_count: number | string;
}

export interface UsageBucketRow {
  /** TIMESTAMPTZ — encoded as epoch ms by the warehouse bridge. */
  bucket: number | string;
  host_uuid: string;
  avg_value: number | string | null;
  sample_count: number | string;
}

export interface UsagePerMeterRow {
  point_uuid: string;
  name: string | null;
  host_uuid: string | null;
  host_name: string | null;
  device_name: string | null;
  avg_value: number | string | null;
  min_value: number | string | null;
  max_value: number | string | null;
  sample_count: number | string;
}
