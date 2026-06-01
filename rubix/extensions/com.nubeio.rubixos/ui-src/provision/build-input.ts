// `build-input.ts` — fold a Placement + barcode + toggles into a ProvisionInput.
import type { Placement } from "../pwa/place";
import type { ProvisionInput } from "./bc-types";

export function buildProvisionInput(
  barcode: string,
  placement: Placement,
  toggles: { trend: boolean; alarm: boolean },
  name?: string,
): ProvisionInput {
  const input: ProvisionInput = { barcode, trend: toggles.trend, alarm: toggles.alarm };
  if (placement.siteId) input.site_id = placement.siteId;
  if (placement.locationId) input.location_id = placement.locationId;
  else if (placement.newLocation.trim()) input.new_location = { name: placement.newLocation.trim() };
  if (placement.pageId) input.page_id = placement.pageId;
  else if (placement.newPage.trim()) input.new_page = { name: placement.newPage.trim() };
  if (name?.trim()) input.name = name.trim();
  return input;
}
