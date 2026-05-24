// Appearance tab. Theme / Palette / FontSize are powered by the
// packaged ui-kit sections; Font stays local because its preview
// shape ("Aa 123") is ui-5-specific and the enum doesn't generalise.

import { Root as Radio, Item } from '@radix-ui/react-radio-group'
import { CircleCheck } from 'lucide-react'
import { useIntl } from 'react-intl'
import {
  FontSizeSection,
  PaletteSection,
  SectionTitle,
  ThemeSection,
  type FontSizeItem,
  type PaletteItem,
  type ThemeIconItem,
} from '@nube/starter-ui-kit/theme-editor/config-drawer'
import { IconThemeDark } from '@/assets/custom/icon-theme-dark'
import { IconThemeLight } from '@/assets/custom/icon-theme-light'
import { IconThemeSystem } from '@/assets/custom/icon-theme-system'
import { cn } from '@/lib/utils'
import {
  DEFAULT_FONT,
  DEFAULT_FONT_SIZE,
  DEFAULT_MODE,
  DEFAULT_PALETTE,
  FONT_STACKS,
  useTheme,
  type Font,
  type FontSize,
  type Mode,
  type Palette,
} from '@/stores/theme-store'

function ThemeConfig() {
  const { mode, setMode } = useTheme()
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const items: ThemeIconItem<Mode>[] = [
    { value: 'system', label: tr('mode.system'), icon: IconThemeSystem, ariaLabel: tr('config.theme.system.aria') },
    { value: 'light',  label: tr('mode.light'),  icon: IconThemeLight,  ariaLabel: tr('config.theme.light.aria') },
    { value: 'dark',   label: tr('mode.dark'),   icon: IconThemeDark,   ariaLabel: tr('config.theme.dark.aria') },
  ]
  return (
    <ThemeSection
      value={mode}
      onChange={setMode}
      items={items}
      defaultValue={DEFAULT_MODE}
      i18n={{
        title: tr('config.theme.title'),
        groupAriaLabel: tr('config.theme.groupAria'),
        resetAriaLabel: tr('config.theme.resetAria'),
      }}
    />
  )
}

const PALETTE_SWATCHES: Record<Palette, string> = {
  nube: 'linear-gradient(135deg,#339999,#184171)',
  ocean: 'linear-gradient(135deg,#3b82f6,#1e3a8a)',
  sunset: 'linear-gradient(135deg,#f97316,#b21368)',
}

function PaletteConfig() {
  const { palette, setPalette } = useTheme()
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const items: PaletteItem<Palette>[] = (Object.keys(PALETTE_SWATCHES) as Palette[]).map((id) => ({
    value: id,
    label: tr(`palette.${id}`),
    swatch: PALETTE_SWATCHES[id],
  }))
  return (
    <PaletteSection
      value={palette}
      onChange={setPalette}
      items={items}
      defaultValue={DEFAULT_PALETTE}
      i18n={{
        title: tr('config.palette.title'),
        groupAriaLabel: tr('config.palette.groupAria'),
        resetAriaLabel: tr('config.palette.resetAria'),
      }}
    />
  )
}

function FontConfig() {
  const { font, setFont } = useTheme()
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const fonts: Font[] = ['geist', 'inter', 'manrope', 'system']
  return (
    <div>
      <SectionTitle
        title={tr('config.font.title')}
        showReset={font !== DEFAULT_FONT}
        onReset={() => setFont(DEFAULT_FONT)}
        resetAriaLabel={tr('config.font.resetAria')}
      />
      <Radio
        value={font}
        onValueChange={(v) => setFont(v as Font)}
        className='grid w-full max-w-md grid-cols-2 gap-2'
        aria-label={tr('config.font.groupAria')}
      >
        {fonts.map((id) => (
          <Item
            key={id}
            value={id}
            className='group outline-none'
            aria-label={`${tr('config.font.title')}: ${tr(`font.${id}`)}`}
          >
            <div
              className={cn(
                'relative rounded-[6px] ring-[1px] ring-[color:var(--color-border)] px-3 py-2 text-left',
                'group-data-[state=checked]:shadow-2xl group-data-[state=checked]:ring-[color:var(--color-leaf)]',
              )}
              style={{ fontFamily: FONT_STACKS[id] }}
            >
              <CircleCheck
                className={cn(
                  'size-5 fill-[color:var(--color-leaf)] stroke-white',
                  'group-data-[state=unchecked]:hidden',
                  'absolute top-0 right-0 translate-x-1/2 -translate-y-1/2',
                )}
              />
              <div className='text-sm font-medium text-[color:var(--color-text)]'>{tr(`font.${id}`)}</div>
              <div className='text-[10px] text-[color:var(--color-subtle)]'>Ag 123</div>
            </div>
          </Item>
        ))}
      </Radio>
    </div>
  )
}

function FontSizeConfig() {
  const { fontSize, setFontSize } = useTheme()
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const items: FontSizeItem<FontSize>[] = [
    { value: 'sm', label: tr('config.fontSize.sm'), px: '14px' },
    { value: 'md', label: tr('config.fontSize.md'), px: '16px' },
    { value: 'lg', label: tr('config.fontSize.lg'), px: '18px' },
  ]
  return (
    <FontSizeSection
      value={fontSize}
      onChange={setFontSize}
      items={items}
      defaultValue={DEFAULT_FONT_SIZE}
      i18n={{
        title: tr('config.fontSize.title'),
        groupAriaLabel: tr('config.fontSize.groupAria'),
        resetAriaLabel: tr('config.fontSize.resetAria'),
      }}
    />
  )
}

export function AppearanceTab() {
  return (
    <div className='space-y-6'>
      <ThemeConfig />
      <PaletteConfig />
      <FontConfig />
      <FontSizeConfig />
    </div>
  )
}
