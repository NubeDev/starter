import {
  Leaf,
  Droplet,
  Wind,
  Sun,
  Sprout,
  Recycle,
  Sparkles,
  type LucideIcon,
} from 'lucide-react'

export interface NavItem {
  label: string
  href: string
  icon: LucideIcon
  badge?: string
}

export interface NavGroup {
  title: string
  items: NavItem[]
}

export const NAV_GROUPS: NavGroup[] = [
  {
    title: 'Living systems',
    items: [
      { label: 'Air', href: '#air', icon: Wind, badge: '12' },
      { label: 'Water', href: '#water', icon: Droplet, badge: 'pH 7.2' },
      { label: 'Energy', href: '#energy', icon: Sun, badge: '+42 kWh' },
      { label: 'Garden', href: '#garden', icon: Sprout },
    ],
  },
  {
    title: 'Cycle',
    items: [
      { label: 'Compost', href: '#compost', icon: Leaf },
      { label: 'Greywater', href: '#greywater', icon: Recycle },
      { label: 'Insights', href: '#insights', icon: Sparkles },
    ],
  },
]
