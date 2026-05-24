// rubix's ConfigDrawer — Stage-2 graduated. The Sheet + tab shell
// now comes from `@nube/starter-ui-kit/theme-editor`. The four tab
// contents are composed locally because some sections still depend on
// ui-5-only stores (font, radius, shell mode, direction).

import { Layers, Palette as PaletteIcon, Sliders, Sparkles } from 'lucide-react'
import { useIntl } from 'react-intl'
import { ConfigDrawer as KitConfigDrawer } from '@nube/starter-ui-kit/theme-editor'
import { useDirection } from '@nube/starter-ui-core/layout'
import { useLayout } from '@nube/starter-ui-core/layout'
import { useTheme } from '@/stores/theme-store'
import { useSidebar } from '@/components/ui/sidebar'
import { AdvancedTab } from './sections/advanced'
import { AppearanceTab } from './sections/appearance'
import { BrandingTab } from './sections/branding'
import { LayoutTab } from './sections/layout'

export function ConfigDrawer() {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const { setOpen } = useSidebar()
  const { resetDir } = useDirection()
  const { resetTheme } = useTheme()
  const { resetLayout } = useLayout()

  const handleReset = () => {
    setOpen(true)
    resetDir()
    resetTheme()
    resetLayout()
  }

  return (
    <KitConfigDrawer
      i18n={{
        title: tr('config.title'),
        description: tr('config.description'),
        triggerAriaLabel: tr('config.triggerAria'),
        tabsAriaLabel: tr('config.tabsAria'),
        resetLabel: tr('config.reset'),
        resetAriaLabel: tr('config.resetAria'),
      }}
      onReset={handleReset}
      tabs={[
        { id: 'appearance', label: tr('config.tab.appearance'), icon: PaletteIcon, content: <AppearanceTab /> },
        { id: 'layout', label: tr('config.tab.layout'), icon: Layers, content: <LayoutTab /> },
        { id: 'branding', label: tr('config.tab.branding'), icon: Sparkles, content: <BrandingTab /> },
        { id: 'advanced', label: tr('config.tab.advanced'), icon: Sliders, content: <AdvancedTab /> },
      ]}
    />
  )
}
