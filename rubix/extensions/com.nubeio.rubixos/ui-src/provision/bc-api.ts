// `bc-api.ts` — thin, named wrappers over the bc_* tools + read templates.
// Each function just forwards to callTool / fetchTemplate from the shared
// extension api so callers never hand-spell tool ids or template names.

import { callTool, fetchTemplate } from "../api";
import { EXTENSION_ID } from "../types";
import { bumpRefresh } from "./refresh";
import type {
  AlarmRow,
  AssignPageInput,
  AssignPageResult,
  DeviceRow,
  LabelRender,
  LocationRow,
  LogRow,
  MutationResult,
  PageDeleteResult,
  PageRow,
  PointRow,
  ProvisionInput,
  ProvisionResult,
  ScannedIdentity,
  SiteRow,
  TemplateRow,
  TemplateYaml,
  WidgetRow,
} from "./bc-types";

const tool = (name: string) => `${EXTENSION_ID}.${name}`;

// Every write goes through here so the shared data version bumps as
// soon as the server confirms — sibling list tabs then re-fetch and
// converge without a page reload. `decode` is read-only (no bump).
function mutate<T>(toolId: string, params: unknown): Promise<T> {
  return callTool<T>(toolId, params).then((res) => {
    bumpRefresh();
    return res;
  });
}

/* ------------------------------- mutations ------------------------------- */

export function decode(barcode: string): Promise<ScannedIdentity> {
  return callTool<ScannedIdentity>(tool("bc_decode"), { barcode });
}

export function provision(input: ProvisionInput): Promise<ProvisionResult> {
  return mutate<ProvisionResult>(tool("bc_provision"), input);
}

// Place an already-commissioned (pending) device on a page: generates
// its widgets and flips status to provisioned. One of page_id / new_page
// is required. Mutation → bumps refresh so list tabs converge.
export function assignPage(input: AssignPageInput): Promise<AssignPageResult> {
  return mutate<AssignPageResult>(tool("bc_device_assign_page"), input);
}

export function deviceUpdate(
  row: { device_id: string } & Record<string, unknown>,
): Promise<MutationResult> {
  return mutate<MutationResult>(tool("bc_device_update"), { row });
}

export function decommission(
  device_ids: ReadonlyArray<string>,
  hard = false,
): Promise<MutationResult> {
  return mutate<MutationResult>(tool("bc_device_decommission"), { device_ids, hard });
}

export function siteCreate(row: { site_id: string; name: string }): Promise<MutationResult> {
  return mutate<MutationResult>(tool("bc_site_create"), { row });
}

export function locationCreate(
  row: { location_id: string; site_id: string; name: string },
): Promise<MutationResult> {
  return mutate<MutationResult>(tool("bc_location_create"), { row });
}

export function pageCreate(
  row: { page_id: string; name: string; site_id?: string; location_id?: string },
): Promise<MutationResult> {
  return mutate<MutationResult>(tool("bc_page_create"), { row });
}

// Update a page by `page_id` — rename, or re-pin its site/location.
export function pageUpdate(
  row: { page_id: string; name?: string; site_id?: string; location_id?: string },
): Promise<MutationResult> {
  return mutate<MutationResult>(tool("bc_page_update"), { row });
}

// Delete a page by `page_id`. Its widgets are dropped; devices placed on
// it are kept but detached (page_id cleared, status → pending).
export function pageDelete(page_id: string): Promise<PageDeleteResult> {
  return mutate<PageDeleteResult>(tool("bc_page_delete"), { row: { page_id } });
}

export function templateUpsert(yaml: string): Promise<MutationResult> {
  return mutate<MutationResult>(tool("bc_template_upsert"), { yaml });
}

export function labelRender(device_id: string): Promise<LabelRender> {
  return callTool<LabelRender>(tool("bc_label_render"), { device_id });
}

/* --------------------------------- reads --------------------------------- */

export function listDevices(
  params: { site_id?: string; status?: string; limit?: number } = {},
): Promise<ReadonlyArray<DeviceRow>> {
  return fetchTemplate<DeviceRow>(tool("bc_devices_list"), params);
}

export function listSites(limit = 200): Promise<ReadonlyArray<SiteRow>> {
  return fetchTemplate<SiteRow>(tool("bc_sites_list"), { limit });
}

export function listLocations(
  params: { site_id?: string; limit?: number } = {},
): Promise<ReadonlyArray<LocationRow>> {
  return fetchTemplate<LocationRow>(tool("bc_locations_list"), params);
}

export function listPages(
  params: { site_id?: string; location_id?: string; limit?: number } = {},
): Promise<ReadonlyArray<PageRow>> {
  return fetchTemplate<PageRow>(tool("bc_pages_list"), { limit: 200, ...params });
}

export function listTemplates(limit = 200): Promise<ReadonlyArray<TemplateRow>> {
  return fetchTemplate<TemplateRow>(tool("bc_templates_list"), { limit });
}

export function getTemplateYaml(template: string): Promise<ReadonlyArray<TemplateYaml>> {
  return fetchTemplate<TemplateYaml>(tool("bc_template_yaml"), { template });
}

export function pointsByDevice(device_id: string): Promise<ReadonlyArray<PointRow>> {
  return fetchTemplate<PointRow>(tool("bc_points_by_device"), { device_id });
}

export function widgetsByPage(page_id: string): Promise<ReadonlyArray<WidgetRow>> {
  return fetchTemplate<WidgetRow>(tool("bc_widgets_by_page"), { page_id });
}

export function alarmsByDevice(device_id: string): Promise<ReadonlyArray<AlarmRow>> {
  return fetchTemplate<AlarmRow>(tool("bc_alarms_by_device"), { device_id });
}

export function provisionLog(
  params: { device_id?: string; limit?: number } = {},
): Promise<ReadonlyArray<LogRow>> {
  return fetchTemplate<LogRow>(tool("bc_provision_log_recent"), params);
}
