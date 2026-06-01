// `bc-types.ts` — domain row/result shapes for the barcode provisioning feature.
// Mirrors the JSON the bc_* tools and read templates return.

/** A widget enum from a template point — the curated renderer catalog key. */
export type WidgetKind =
  | "gauge"
  | "stat"
  | "battery"
  | "counter"
  | "led"
  | "toggle"
  | "line";

export interface TemplatePoint {
  key: string;
  name: string;
  widget: WidgetKind | string;
}

export interface ScannedTemplate {
  display_name: string;
  icon: string;
  category: string;
  points: ReadonlyArray<TemplatePoint>;
  widget_group: string;
}

/** Result of bc_decode. */
export interface ScannedIdentity {
  id: string;
  model: string;
  network: string;
  address: string;
  default_ip: string;
  hw: string;
  template: ScannedTemplate;
  known_models: ReadonlyArray<string>;
}

/** Input to bc_provision. */
export interface ProvisionInput {
  barcode: string;
  site_id?: string;
  location_id?: string;
  new_location?: { name: string };
  page_id?: string;
  new_page?: { name: string };
  name?: string;
  trend?: boolean;
  alarm?: boolean;
}

/** Result of bc_provision. */
export interface ProvisionResult {
  device_id: string;
  points: number;
  widgets: number;
  alarms: number;
  page_id: string;
  warnings: ReadonlyArray<string>;
}

/** Generic mutation envelope returned by the write tools. */
export interface MutationResult {
  operation: string;
  mode?: string;
  affected: number;
  template?: unknown;
}

export interface DeviceRow {
  device_id: string;
  template: string;
  name: string | null;
  network: string | null;
  address: string | null;
  site_id: string | null;
  location_id: string | null;
  page_id: string | null;
  status: string;
  provisioned_at: string | null;
}

export interface SiteRow {
  site_id: string;
  name: string;
  created_at: string;
}

export interface LocationRow {
  location_id: string;
  site_id: string;
  name: string;
  created_at: string;
}

export interface PageRow {
  page_id: string;
  name: string;
  created_at: string;
}

export interface TemplateRow {
  template: string;
  version: string | number;
  display_name: string;
  network: string;
  category: string;
  icon: string;
  updated_at: string;
}

export interface TemplateYaml {
  template: string;
  yaml: string;
  points_json: string;
  widget_group_json: string;
}

export interface PointRow {
  point_id: string;
  device_id: string;
  point_key: string;
  name: string;
  unit: string | null;
  kind: string;
  widget: WidgetKind | string;
  writable: boolean;
  trend_on: boolean;
  alarm_on: boolean;
  trend_interval: string | number | null;
}

export interface WidgetRow {
  widget_id: string;
  page_id: string;
  device_id: string;
  point_id: string | null;
  widget: WidgetKind | string;
  slot: string | number | null;
  role: string | null;
  title: string | null;
}

export interface AlarmRow {
  alarm_id: string;
  device_id: string;
  point_id: string;
  point_key: string;
  predicate: string;
  severity: string;
  message: string;
  enabled: boolean;
}

export interface LogRow {
  event_id: string;
  device_id: string | null;
  event: string;
  step: string | null;
  detail: string | null;
  at: string;
}

export interface LabelRender {
  device_id: string;
  serial: string;
  qr_url: string;
  code128: string;
  display_name: string;
}
