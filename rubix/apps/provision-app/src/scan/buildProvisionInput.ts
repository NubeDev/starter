import type { ProvisionInput } from '../api/bc-types'
import type { Placement } from '../place/placement'

// Translate the UI placement + toggles into the bc_provision payload. New
// location/page names become `new_*` objects; chosen ids pass straight through.
export function buildProvisionInput(
  barcode: string,
  place: Placement,
  trend: boolean,
  alarm: boolean,
  name?: string,
): ProvisionInput {
  const input: ProvisionInput = { barcode, site_id: place.siteId, trend, alarm }
  if (name) input.name = name

  if (place.locationId) input.location_id = place.locationId
  else if (place.newLocation.trim()) input.new_location = { name: place.newLocation.trim() }

  if (place.pageId) input.page_id = place.pageId
  else if (place.newPage.trim()) input.new_page = { name: place.newPage.trim() }

  return input
}
