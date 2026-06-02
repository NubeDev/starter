import { useState } from 'react'
import { Toggle } from '../../components/FormKit'

// Writable boolean point rendered as a switch (e.g. a relay/output).
// Optimistic local flip in the preview (no command dispatch yet).
export function ToggleWidget({
  title,
  on = false,
  accent,
}: {
  title: string
  on?: boolean
  accent: string
}) {
  const [state, setState] = useState(on)
  return (
    <div className="flex h-full flex-col justify-between">
      <p className="label">{title}</p>
      <div className="flex items-center justify-between">
        <span className="text-sm font-semibold text-ink-variant">{state ? 'On' : 'Off'}</span>
        <Toggle on={state} onToggle={() => setState((v) => !v)} accent={accent} label={title} />
      </div>
    </div>
  )
}
