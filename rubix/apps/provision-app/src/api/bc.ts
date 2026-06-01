// `bc.ts` — typed, named wrappers over the bc_* tools. Each forwards to
// transport.dispatch under the hood so callers never hand-spell tool ids.
// Mirrors the extension's ui-src/provision/bc-api.ts: reads go through
// warehouse_query (always fresh), mutations bump the shared refresh signal.

import { transport } from '../transport'
import { bumpRefresh } from './refresh'
import type {
  AlarmRow,
  AssignPageInput,
  AssignPageResult,
  DeviceRow,
  LabelRender,
  LocationRow,
  LogRow,
  MutationResult,
  PageRow,
  PointRow,
  ProvisionInput,
  ProvisionResult,
  ScannedIdentity,
  SiteRow,
  TemplateRow,
  TemplateYaml,
  WarehouseQueryResponse,
  WidgetRow,
} from './bc-types'

// The owning extension id — tool ids are `${EXTENSION_ID}.${name}`.
export const EXTENSION_ID = 'com.nubeio.rubixos'
const tool = (name: string) => `${EXTENSION_ID}.${name}`

// Every write goes through here so the shared refresh signal bumps the moment
// the server confirms — sibling list views then re-fetch and converge.
function mutate<T>(toolId: string, params: unknown): Promise<T> {
  return transport.dispatch<T>(toolId, params, { fresh: true }).then((res) => {
    bumpRefresh()
    return res
  })
}

// A list read: warehouse_query envelope, always fresh (never a coalesced stale
// in-flight read after a mutation or navigation).
async function query<R>(
  template: string,
  params: Record<string, unknown> = {},
): Promise<ReadonlyArray<R>> {
  const res = await transport.dispatch<WarehouseQueryResponse<R>>(
    tool('warehouse_query'),
    { template, params },
    { fresh: true },
  )
  return res.rows
}

/* ------------------------------- mutations ------------------------------- */

export function decode(barcode: string): Promise<ScannedIdentity> {
  // read-only — no refresh bump
  return transport.dispatch<ScannedIdentity>(tool('bc_decode'), { barcode })
}

export function provision(input: ProvisionInput): Promise<ProvisionResult> {
  return mutate<ProvisionResult>(tool('bc_provision'), input)
}

// Assign a (pending) device to a page — existing page_id or a new_page name.
// Generates widgets and flips status to 'provisioned'. Bumps refresh so the
// device list / detail re-fetch the new status + page_id.
export function assignPage(input: AssignPageInput): Promise<AssignPageResult> {
  return mutate<AssignPageResult>(tool('bc_device_assign_page'), input)
}

export function deviceUpdate(
  row: { device_id: string } & Record<string, unknown>,
): Promise<MutationResult> {
  return mutate<MutationResult>(tool('bc_device_update'), { row })
}

export function decommission(
  device_ids: ReadonlyArray<string>,
  hard = false,
): Promise<MutationResult> {
  return mutate<MutationResult>(tool('bc_device_decommission'), { device_ids, hard })
}

export function siteCreate(row: { site_id: string; name: string }): Promise<MutationResult> {
  return mutate<MutationResult>(tool('bc_site_create'), { row })
}

export function locationCreate(
  row: { location_id: string; site_id: string; name: string },
): Promise<MutationResult> {
  return mutate<MutationResult>(tool('bc_location_create'), { row })
}

export function pageCreate(
  row: { page_id: string; site_id: string; name: string },
): Promise<MutationResult> {
  return mutate<MutationResult>(tool('bc_page_create'), { row })
}

export function templateUpsert(yaml: string): Promise<MutationResult> {
  return mutate<MutationResult>(tool('bc_template_upsert'), { yaml })
}

export function labelRender(device_id: string): Promise<LabelRender> {
  return transport.dispatch<LabelRender>(tool('bc_label_render'), { device_id })
}

/* --------------------------------- reads --------------------------------- */

export function devicesList(
  params: { site_id?: string; status?: string; limit?: number } = {},
): Promise<ReadonlyArray<DeviceRow>> {
  return query<DeviceRow>(tool('bc_devices_list'), params)
}

export function sitesList(limit = 200): Promise<ReadonlyArray<SiteRow>> {
  return query<SiteRow>(tool('bc_sites_list'), { limit })
}

export function locationsList(
  params: { site_id?: string; limit?: number } = {},
): Promise<ReadonlyArray<LocationRow>> {
  return query<LocationRow>(tool('bc_locations_list'), params)
}

export function pagesList(siteId?: string, limit = 200): Promise<ReadonlyArray<PageRow>> {
  return query<PageRow>(tool('bc_pages_list'), siteId ? { site_id: siteId, limit } : { limit })
}

export function templatesList(limit = 200): Promise<ReadonlyArray<TemplateRow>> {
  return query<TemplateRow>(tool('bc_templates_list'), { limit })
}

export function templateYaml(template: string): Promise<ReadonlyArray<TemplateYaml>> {
  return query<TemplateYaml>(tool('bc_template_yaml'), { template })
}

export function pointsByDevice(device_id: string): Promise<ReadonlyArray<PointRow>> {
  return query<PointRow>(tool('bc_points_by_device'), { device_id })
}

export function widgetsByPage(page_id: string): Promise<ReadonlyArray<WidgetRow>> {
  return query<WidgetRow>(tool('bc_widgets_by_page'), { page_id })
}

export function alarmsByDevice(device_id: string): Promise<ReadonlyArray<AlarmRow>> {
  return query<AlarmRow>(tool('bc_alarms_by_device'), { device_id })
}

export function provisionLogRecent(
  params: { device_id?: string; limit?: number } = {},
): Promise<ReadonlyArray<LogRow>> {
  return query<LogRow>(tool('bc_provision_log_recent'), params)
}
