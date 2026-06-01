import { Activity, Sun, Moon, Factory, type LucideIcon } from 'lucide-react'

// ── App theme ───────────────────────────────────────────────────────────────
// The long-term skin, re-skinned from the design system's couple themes to an
// energy/IoT domain. Every component reads the resolved look via useLook(),
// never these objects directly. The transient "mood" layer is repurposed as
// device/connection STATUS (see statuses.ts) which tints the live accent.

export type ThemeKey = 'grid' | 'solar' | 'offpeak' | 'industrial'

export interface AppTheme {
  key: ThemeKey
  label: string
  blurb: string
  icon: LucideIcon

  base: string // page base behind the gradient
  accent: string // primary accent (status may override the live accent)
  accent2: string // secondary accent
  ink: string // headline text tint
  inkSoft: string // body text tint

  // three radial-gradient stops = the ambient base look
  gradient: [string, string, string]

  radius: string
  glowAlpha: number
}

export const THEMES: Record<ThemeKey, AppTheme> = {
  grid: {
    key: 'grid',
    label: 'Grid',
    blurb: 'Electric teal · live',
    icon: Activity,
    base: '#07090b',
    accent: '#36e2c4',
    accent2: '#ffc24b',
    ink: '#e7f0ef',
    inkSoft: '#b9c7c6',
    gradient: [
      'radial-gradient(60% 40% at 80% -10%, rgba(54,226,196,0.20), transparent 70%)',
      'radial-gradient(55% 35% at 0% 8%, rgba(255,194,75,0.12), transparent 70%)',
      'radial-gradient(70% 50% at 50% 110%, rgba(31,126,111,0.18), transparent 70%)',
    ],
    radius: '1.5rem',
    glowAlpha: 0.45,
  },
  solar: {
    key: 'solar',
    label: 'Solar',
    blurb: 'Amber · daytime yield',
    icon: Sun,
    base: '#0c0a06',
    accent: '#ffc24b',
    accent2: '#ff8f5e',
    ink: '#fff3da',
    inkSoft: '#e7d6b4',
    gradient: [
      'radial-gradient(65% 45% at 78% -8%, rgba(255,194,75,0.28), transparent 70%)',
      'radial-gradient(55% 40% at 6% 10%, rgba(255,143,94,0.18), transparent 72%)',
      'radial-gradient(80% 55% at 50% 112%, rgba(255,210,122,0.16), transparent 72%)',
    ],
    radius: '1.5rem',
    glowAlpha: 0.42,
  },
  offpeak: {
    key: 'offpeak',
    label: 'Off-peak',
    blurb: 'Cool · low demand',
    icon: Moon,
    base: '#070a10',
    accent: '#8fb6ff',
    accent2: '#36e2c4',
    ink: '#e3ecff',
    inkSoft: '#bcc8e6',
    gradient: [
      'radial-gradient(70% 50% at 50% -10%, rgba(90,130,210,0.24), transparent 72%)',
      'radial-gradient(55% 45% at 14% 24%, rgba(54,226,196,0.16), transparent 72%)',
      'radial-gradient(95% 65% at 50% 116%, rgba(15,22,45,0.55), transparent 76%)',
    ],
    radius: '1.25rem',
    glowAlpha: 0.5,
  },
  industrial: {
    key: 'industrial',
    label: 'Industrial',
    blurb: 'Steel · plant floor',
    icon: Factory,
    base: '#0a0b0c',
    accent: '#9fb0b8',
    accent2: '#ffc24b',
    ink: '#e9eef0',
    inkSoft: '#bcc6ca',
    gradient: [
      'radial-gradient(65% 45% at 78% -8%, rgba(159,176,184,0.16), transparent 70%)',
      'radial-gradient(55% 40% at 6% 10%, rgba(255,194,75,0.10), transparent 72%)',
      'radial-gradient(80% 55% at 50% 112%, rgba(40,48,52,0.55), transparent 74%)',
    ],
    radius: '1rem',
    glowAlpha: 0.4,
  },
}

export const THEME_ORDER: ThemeKey[] = ['grid', 'solar', 'offpeak', 'industrial']
export const DEFAULT_THEME: ThemeKey = 'grid'
