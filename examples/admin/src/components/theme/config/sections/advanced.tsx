// Advanced tab. Section heading + ComingSoonField come from
// ui-kit's packaged drawer; the colour-row / disabled-input shapes
// are local decoration.

import { useIntl } from 'react-intl'
import { ComingSoonField, SectionTitle } from '@nube/starter-ui-kit/theme-editor'

function DisabledInput({ placeholder }: { placeholder: string }) {
  return (
    <input
      type='text'
      placeholder={placeholder}
      disabled
      className='w-full cursor-not-allowed rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/40 px-3 py-2 text-sm text-[color:var(--color-muted)] placeholder:text-[color:var(--color-subtle)] focus:outline-none'
    />
  )
}

function ColorRow({ label, value }: { label: string; value: string }) {
  return (
    <div className='flex items-center gap-2'>
      <div
        className='size-7 shrink-0 rounded ring-1 ring-[color:var(--color-border)]'
        style={{ background: value }}
      />
      <div className='flex-1'>
        <div className='text-xs font-medium text-[color:var(--color-text)]'>{label}</div>
        <div className='text-[10px] text-[color:var(--color-subtle)]'>{value}</div>
      </div>
      <DisabledInput placeholder={value} />
    </div>
  )
}

export function AdvancedTab() {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  return (
    <div className='space-y-6'>
      <SectionTitle
        title={tr('config.advanced.title')}
        description={tr('config.advanced.description')}
      />
      <ComingSoonField
        label={tr('config.advanced.palette.label')}
        description={tr('config.advanced.palette.description')}
        badgeLabel={tr('config.comingSoon')}
        control={
          <div className='space-y-2 rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/40 p-3 opacity-60'>
            <ColorRow label={tr('config.advanced.palette.primary')} value='#339999' />
            <ColorRow label={tr('config.advanced.palette.accent')}  value='#67e8f9' />
            <ColorRow label={tr('config.advanced.palette.surface')} value='#0d1f4a' />
          </div>
        }
      />
      <ComingSoonField
        label={tr('config.advanced.font.label')}
        description={tr('config.advanced.font.description')}
        badgeLabel={tr('config.comingSoon')}
        control={
          <div className='space-y-2'>
            <DisabledInput placeholder={tr('config.advanced.font.placeholder')} />
            <div className='text-center text-[10px] text-[color:var(--color-subtle)]'>
              {tr('config.advanced.font.or')}
            </div>
            <div className='flex cursor-not-allowed items-center justify-center rounded-md border border-dashed border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/40 px-3 py-4 text-xs text-[color:var(--color-muted)]'>
              {tr('config.advanced.font.upload')}
            </div>
          </div>
        }
      />
    </div>
  )
}
