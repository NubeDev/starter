import { Root as Radio, Item } from '@radix-ui/react-radio-group'
import { CircleCheck } from 'lucide-react'
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
import { RadioGroupItem, RadioTile, SectionTitle } from '../shared'

const PALETTE_SWATCHES: Record<Palette, string> = {
  nube: 'linear-gradient(135deg,#339999,#184171)',
  ocean: 'linear-gradient(135deg,#3b82f6,#1e3a8a)',
  sunset: 'linear-gradient(135deg,#f97316,#b21368)',
}

function ThemeConfig() {
  const { mode, setMode } = useTheme()
  return (
    <div>
      <SectionTitle
        title='Theme'
        showReset={mode !== DEFAULT_MODE}
        onReset={() => setMode(DEFAULT_MODE)}
        resetAriaLabel='Reset theme preference to default'
      />
      <Radio
        value={mode}
        onValueChange={(v) => setMode(v as Mode)}
        className='grid w-full max-w-md grid-cols-3 gap-4'
        aria-label='Select theme preference'
      >
        {[
          { value: 'system', label: 'System', icon: IconThemeSystem },
          { value: 'light', label: 'Light', icon: IconThemeLight },
          { value: 'dark', label: 'Dark', icon: IconThemeDark },
        ].map((item) => (
          <RadioGroupItem key={item.value} item={item} isTheme />
        ))}
      </Radio>
    </div>
  )
}

function PaletteConfig() {
  const { palette, setPalette } = useTheme()
  return (
    <div>
      <SectionTitle
        title='Palette'
        showReset={palette !== DEFAULT_PALETTE}
        onReset={() => setPalette(DEFAULT_PALETTE)}
        resetAriaLabel='Reset palette to default'
      />
      <Radio
        value={palette}
        onValueChange={(v) => setPalette(v as Palette)}
        className='grid w-full max-w-md grid-cols-3 gap-4'
        aria-label='Select palette'
      >
        {(Object.keys(PALETTE_SWATCHES) as Palette[]).map((id) => (
          <Item
            key={id}
            value={id}
            className='group outline-none'
            aria-label={`Select ${id} palette`}
          >
            <div
              className={cn(
                'relative rounded-[6px] ring-[1px] ring-[color:var(--color-border)] p-3',
                'group-data-[state=checked]:shadow-2xl group-data-[state=checked]:ring-[color:var(--color-leaf)]',
              )}
            >
              <CircleCheck
                className={cn(
                  'size-6 fill-[color:var(--color-leaf)] stroke-white',
                  'group-data-[state=unchecked]:hidden',
                  'absolute top-0 right-0 translate-x-1/2 -translate-y-1/2',
                )}
              />
              <div className='h-10 w-full rounded' style={{ background: PALETTE_SWATCHES[id] }} />
            </div>
            <div className='mt-1 text-xs capitalize'>{id}</div>
          </Item>
        ))}
      </Radio>
    </div>
  )
}

function FontConfig() {
  const { font, setFont } = useTheme()
  const fonts: Font[] = ['geist', 'inter', 'manrope', 'system']
  return (
    <div>
      <SectionTitle
        title='Font'
        showReset={font !== DEFAULT_FONT}
        onReset={() => setFont(DEFAULT_FONT)}
        resetAriaLabel='Reset font to default'
      />
      <Radio
        value={font}
        onValueChange={(v) => setFont(v as Font)}
        className='grid w-full max-w-md grid-cols-2 gap-2'
        aria-label='Select font'
      >
        {fonts.map((id) => (
          <Item
            key={id}
            value={id}
            className='group outline-none'
            aria-label={`Select ${id} font`}
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
              <div className='text-sm font-medium capitalize text-[color:var(--color-text)]'>{id}</div>
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
  const items: { value: FontSize; label: string; px: string }[] = [
    { value: 'sm', label: 'Small', px: '14px' },
    { value: 'md', label: 'Medium', px: '16px' },
    { value: 'lg', label: 'Large', px: '18px' },
  ]
  return (
    <div>
      <SectionTitle
        title='Base font size'
        showReset={fontSize !== DEFAULT_FONT_SIZE}
        onReset={() => setFontSize(DEFAULT_FONT_SIZE)}
        resetAriaLabel='Reset font size to default'
      />
      <Radio
        value={fontSize}
        onValueChange={(v) => setFontSize(v as FontSize)}
        className='grid w-full max-w-md grid-cols-3 gap-2'
        aria-label='Select base font size'
      >
        {items.map((item) => (
          <RadioTile
            key={item.value}
            value={item.value}
            label={item.label}
            ariaLabel={`Select ${item.label.toLowerCase()} font size`}
          >
            <div className='font-medium text-[color:var(--color-text)]' style={{ fontSize: item.px }}>
              Aa
            </div>
            <div className='text-[10px] text-[color:var(--color-subtle)]'>{item.label}</div>
          </RadioTile>
        ))}
      </Radio>
    </div>
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
