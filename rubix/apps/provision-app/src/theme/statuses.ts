// Device/connection STATUS — the repurposed "mood" layer (§11.3). A live status
// tints the app accent on top of the chosen theme, so the whole UI reflects the
// agent connection state at a glance. null status = use the theme accent.
export type StatusKey = 'online' | 'pairing' | 'fault' | 'offline'

export interface AppStatus {
  key: StatusKey
  label: string
  accent: string
}

export const STATUSES: Record<StatusKey, AppStatus> = {
  online: { key: 'online', label: 'Connected', accent: '#36e2c4' },
  pairing: { key: 'pairing', label: 'Provisioning', accent: '#ffc24b' },
  fault: { key: 'fault', label: 'Fault', accent: '#ff5a52' },
  offline: { key: 'offline', label: 'Offline', accent: '#7c8a8a' },
}
