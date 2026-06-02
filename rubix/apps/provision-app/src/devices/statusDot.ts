import type { DeviceRow } from '../api/bc-types'

// Map a device status string → a status color token. Keeps the dot color
// logic in one place for the list and detail views. `pending` shares the amber
// (warning) token with `pairing`.
export function statusColor(status: string): string {
  switch (status) {
    case 'online':
    case 'provisioned':
    case 'active':
      return '#36e2c4'
    case 'pairing':
    case 'pending':
      return '#ffc24b'
    case 'fault':
    case 'error':
      return '#ff5a52'
    default:
      return '#7c8a8a'
  }
}

// A device is placeable on a page when it's commissioned but not yet on one —
// status `pending` or simply missing a page_id.
export function isPlaceable(d: DeviceRow): boolean {
  return d.status === 'pending' || d.page_id === null
}
