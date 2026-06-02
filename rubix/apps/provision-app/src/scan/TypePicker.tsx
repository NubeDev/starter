import { useEffect, useState } from 'react'
import { useRefreshKey } from '../api/refresh'
import { templatesList } from '../api/bc'
import { mintId } from '../api/ids'
import { Field, Picker } from '../components/FormKit'
import { PrimaryButton } from '../components/ui'
import { useLook } from '../theme/useLook'
import type { TemplateRow } from '../api/bc-types'

// "Pick a device type" path — no barcode needed. Choose a template, we mint a
// serial and synthesise the canonical rubix://add?… string, then run the same
// bc_decode → place → provision flow. Mirrors the wizard's "Choose a type" mode.
export function TypePicker({ onSynthesized }: { onSynthesized: (raw: string) => void }) {
  const look = useLook()
  const refresh = useRefreshKey()
  const [templates, setTemplates] = useState<ReadonlyArray<TemplateRow>>([])
  const [chosen, setChosen] = useState('')

  useEffect(() => {
    templatesList().then(setTemplates).catch(() => {})
  }, [refresh])

  const go = () => {
    const t = templates.find((x) => x.template === chosen)
    if (!t) return
    const serial = mintId(t.template.slice(0, 3).toUpperCase()).replace(/_/g, '-')
    const raw = `rubix://add?id=${serial}&model=${encodeURIComponent(t.template)}&network=${encodeURIComponent(
      t.network,
    )}&v=1`
    onSynthesized(raw)
  }

  return (
    <div className="glass rounded-2xl p-4">
      <Field label="Device type">
        <Picker
          value={chosen}
          placeholder="Select a template"
          options={templates.map((t) => ({ value: t.template, label: t.display_name }))}
          onChange={setChosen}
        />
      </Field>
      <div className="mt-3">
        <PrimaryButton accent={look.accent} disabled={!chosen} onClick={go}>
          Use this type
        </PrimaryButton>
      </div>
    </div>
  )
}
