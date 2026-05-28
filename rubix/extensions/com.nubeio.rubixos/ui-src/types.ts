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
