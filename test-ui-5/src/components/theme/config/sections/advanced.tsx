import { ComingSoonField, SectionTitle } from '../shared'

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
  return (
    <div className='space-y-6'>
      <SectionTitle
        title='Advanced'
        description='Fine-grained control for power users and design systems.'
      />
      <ComingSoonField
        label='Custom palette'
        description='Define your own primary, accent, and surface colors.'
        control={
          <div className='space-y-2 rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/40 p-3 opacity-60'>
            <ColorRow label='Primary' value='#339999' />
            <ColorRow label='Accent' value='#67e8f9' />
            <ColorRow label='Surface' value='#0d1f4a' />
          </div>
        }
      />
      <ComingSoonField
        label='Custom font'
        description='Upload a font file or provide a Google Fonts URL.'
        control={
          <div className='space-y-2'>
            <DisabledInput placeholder='https://fonts.googleapis.com/css2?family=…' />
            <div className='text-center text-[10px] text-[color:var(--color-subtle)]'>or</div>
            <div className='flex cursor-not-allowed items-center justify-center rounded-md border border-dashed border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/40 px-3 py-4 text-xs text-[color:var(--color-muted)]'>
              Upload .woff, .woff2, or .ttf
            </div>
          </div>
        }
      />
    </div>
  )
}
