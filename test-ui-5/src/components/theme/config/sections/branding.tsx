import { ImagePlus, Upload } from 'lucide-react'
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

function DisabledDropzone({
  icon: Icon,
  hint,
}: {
  icon: typeof Upload
  hint: string
}) {
  return (
    <div className='flex cursor-not-allowed flex-col items-center justify-center gap-2 rounded-md border border-dashed border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/40 px-3 py-6 text-center'>
      <Icon className='size-5 text-[color:var(--color-subtle)]' aria-hidden='true' />
      <div className='text-xs text-[color:var(--color-muted)]'>{hint}</div>
      <div className='text-[10px] text-[color:var(--color-subtle)]'>
        PNG, SVG up to 1 MB
      </div>
    </div>
  )
}

export function BrandingTab() {
  return (
    <div className='space-y-6'>
      <SectionTitle
        title='Branding'
        description='Customize how your workspace appears to users.'
      />
      <ComingSoonField
        label='Logo'
        description='Shown in the top header and login screen.'
        control={
          <DisabledDropzone icon={Upload} hint='Drop a logo here or click to upload' />
        }
      />
      <ComingSoonField
        label='Favicon'
        description='Displayed in the browser tab and bookmarks.'
        control={
          <DisabledDropzone icon={ImagePlus} hint='Drop a favicon (32×32 recommended)' />
        }
      />
      <ComingSoonField
        label='Browser tab name'
        description='Overrides the default document title.'
        control={<DisabledInput placeholder='Nube IoT Console' />}
      />
    </div>
  )
}
