import { useAppTheme } from './themeContext'
import { STATUSES } from './statuses'

// The RESOLVED look every component reads. Merges two layers:
//   1. app theme   → base palette, shape personality (persistent)
//   2. live status → tints the accent on top (transient: online/pairing/…)
// Components never merge these themselves; they just call useLook().
export interface Look {
  accent: string // live accent — status wins if set, else theme accent
  accent2: string
  ink: string
  inkSoft: string
  base: string
  radius: string
  glowAlpha: number
  baseGradient: [string, string, string]
  // an optional extra gradient wash contributed by the active status (null if none)
  statusTint: string | null
  themeAccent: string
  statusAccent: string | null
}

export function useLook(): Look {
  const { theme, status } = useAppTheme()
  const statusColor = status ? STATUSES[status].accent : null
  const accent = statusColor ?? theme.accent

  const statusTint = statusColor
    ? `radial-gradient(120% 80% at 50% 50%, ${hexA(statusColor, 0.1)}, transparent 70%)`
    : null

  return {
    accent,
    accent2: theme.accent2,
    ink: theme.ink,
    inkSoft: theme.inkSoft,
    base: theme.base,
    radius: theme.radius,
    glowAlpha: theme.glowAlpha,
    baseGradient: theme.gradient,
    statusTint,
    themeAccent: theme.accent,
    statusAccent: statusColor,
  }
}

// "#rrggbb" + alpha → "rgba(...)". Falls back to the raw color if not 6-digit hex.
export function hexA(hex: string, alpha: number): string {
  const m = /^#?([\da-f]{2})([\da-f]{2})([\da-f]{2})$/i.exec(hex)
  if (!m) return hex
  const [r, g, b] = [m[1], m[2], m[3]].map((h) => parseInt(h, 16))
  return `rgba(${r}, ${g}, ${b}, ${alpha})`
}
