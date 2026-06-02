// The placement choice shared by Scan flow and the Wizard. Mirrors the
// extension's Placement shape (pwa/place.tsx).
export interface Placement {
  siteId: string
  locationId: string
  newLocation: string
  pageId: string
  newPage: string
}

export const EMPTY_PLACEMENT: Placement = {
  siteId: '',
  locationId: '',
  newLocation: '',
  pageId: '',
  newPage: '',
}

// A placement is provisionable once a SITE is chosen. A page is optional — a
// device with a site but no page is commissioned as `pending` and can be placed
// on a page later from Devices.
export function placementReady(p: Placement): boolean {
  return Boolean(p.siteId)
}

// Whether this placement also lands the device on a page (existing or new).
export function placementHasPage(p: Placement): boolean {
  return Boolean(p.pageId || p.newPage.trim())
}
