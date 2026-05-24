// Branding tab. Uses the packaged `SectionTitle` + `ComingSoonField`
// primitives from ui-kit; the disabled dropzone / input shapes are
// kept local since they're cosmetic decoration for a not-yet-wired
// feature.

import { ImagePlus, Upload } from 'lucide-react'
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

function DisabledDropzone({
  icon: Icon,
  hint,
  fileLimit,
}: {
  icon: typeof Upload
  hint: string
  fileLimit: string
}) {
  return (
    <div className='flex cursor-not-allowed flex-col items-center justify-center gap-2 rounded-md border border-dashed border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/40 px-3 py-6 text-center'>
      <Icon className='size-5 text-[color:var(--color-subtle)]' aria-hidden='true' />
      <div className='text-xs text-[color:var(--color-muted)]'>{hint}</div>
      <div className='text-[10px] text-[color:var(--color-subtle)]'>{fileLimit}</div>
    </div>
  )
}

export function BrandingTab() {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  return (
    <div className='space-y-6'>
      <SectionTitle
        title={tr('config.branding.title')}
        description={tr('config.branding.description')}
      />
      <ComingSoonField
        label={tr('config.branding.logo.label')}
        description={tr('config.branding.logo.description')}
        badgeLabel={tr('config.comingSoon')}
        control={
          <DisabledDropzone
            icon={Upload}
            hint={tr('config.branding.logo.hint')}
            fileLimit={tr('config.branding.fileLimit')}
          />
        }
      />
      <ComingSoonField
        label={tr('config.branding.favicon.label')}
        description={tr('config.branding.favicon.description')}
        badgeLabel={tr('config.comingSoon')}
        control={
          <DisabledDropzone
            icon={ImagePlus}
            hint={tr('config.branding.favicon.hint')}
            fileLimit={tr('config.branding.fileLimit')}
          />
        }
      />
      <ComingSoonField
        label={tr('config.branding.tabName.label')}
        description={tr('config.branding.tabName.description')}
        badgeLabel={tr('config.comingSoon')}
        control={<DisabledInput placeholder={tr('config.branding.tabName.placeholder')} />}
      />
    </div>
  )
}
