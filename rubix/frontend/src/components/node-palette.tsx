// `<NodePalette>` — left-rail kind catalog for `/flows/$flowId`.
//
// Fetches every registered node kind via `useFlowKinds()` (backed by
// the `rubix.flow_ops.kinds` tool, surfaced through
// `@nube/rubix-client-react`). Two ways to add a node:
//
//   1. **Click** — appends to the YAML body and calls `flowDeploy`.
//   2. **Drag** — palette items are `draggable`; on `dragstart` the
//      `kind_id` is written to the `FLOW_KIND_DRAG_MIME` slot of
//      `DataTransfer`. The route owns the canvas-side `onDrop` /
//      `onDragOver` handlers and runs the same append-and-deploy
//      pipeline via `appendFlowNode`.
//
// Lives under `components/` (not `routes/flows/`) so the TanStack
// Router file-based plugin doesn't scaffold it as a route.

import { useState } from 'react'
import { useIntl } from 'react-intl'
import { Plus } from 'lucide-react'
import { Button, Skeleton } from '@nube/starter-ui-kit'
import { useFlowDeploy, useFlowKinds } from '@nube/rubix-client-react'
import type { FlowKindItem } from '@nube/rubix-client-ts'
import { appendFlowNode, FLOW_KIND_DRAG_MIME } from '@/lib/append-flow-node'

export interface NodePaletteProps {
  flowId: string
  /** Current YAML body of the live revision — the source of truth we mutate. */
  bodyYaml: string
  /** Called after a successful deploy so the parent can refresh local state. */
  onAdded?: (newBodyYaml: string) => void
}

export function NodePalette({ flowId, bodyYaml, onAdded }: NodePaletteProps) {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const kinds = useFlowKinds()
  const deploy = useFlowDeploy()

  const [pendingKindId, setPendingKindId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const handleAdd = async (kind: FlowKindItem) => {
    if (!bodyYaml) {
      setError(tr('flows.palette.noBody', 'Flow body not loaded yet — try again.'))
      return
    }
    setError(null)
    setPendingKindId(kind.kind_id)
    try {
      const nextYaml = appendFlowNode(bodyYaml, kind)
      await deploy.mutateAsync({ flow_id: flowId, body_yaml: nextYaml })
      onAdded?.(nextYaml)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setPendingKindId(null)
    }
  }

  return (
    <aside className="space-y-3 rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)] p-4">
      <header className="space-y-1">
        <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-[color:var(--color-leaf)]">
          {tr('flows.palette.eyebrow', 'Palette')}
        </p>
        <h2 className="text-base font-medium">
          {tr('flows.palette.title', 'Add node')}
        </h2>
        <p className="text-xs text-[color:var(--color-muted)]">
          {tr(
            'flows.palette.help',
            'Click or drag a kind onto the canvas. Edit its config on the right after it appears.',
          )}
        </p>
      </header>

      {kinds.isLoading ? (
        <Skeleton className="h-40 w-full" />
      ) : kinds.isError ? (
        <p className="text-xs text-red-500">
          {tr('flows.palette.loadError', 'Failed to load kinds.')}
        </p>
      ) : !kinds.data || kinds.data.kinds.length === 0 ? (
        <p className="text-xs text-[color:var(--color-muted)]">
          {tr('flows.palette.empty', 'No node kinds registered.')}
        </p>
      ) : (
        <ul className="space-y-2">
          {kinds.data.kinds.map((k) => (
            <li key={k.kind_id}>
              <Button
                variant="outline"
                size="sm"
                className="w-full cursor-grab justify-start gap-2 text-left active:cursor-grabbing"
                disabled={pendingKindId !== null}
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData(FLOW_KIND_DRAG_MIME, k.kind_id)
                  // Plain-text fallback so the payload is still
                  // readable in environments that ignore the custom
                  // MIME (e.g. dragging onto a non-canvas target).
                  e.dataTransfer.setData('text/plain', k.kind_id)
                  e.dataTransfer.effectAllowed = 'copy'
                }}
                onClick={() => handleAdd(k)}
              >
                <Plus className="h-4 w-4 shrink-0" />
                <span className="flex flex-col items-start">
                  <span className="text-sm font-medium">{k.default_label}</span>
                  <span className="font-mono text-[10px] text-[color:var(--color-muted)]">
                    {k.kind_id}
                  </span>
                </span>
              </Button>
            </li>
          ))}
        </ul>
      )}

      {error ? <p className="text-xs text-red-500">{error}</p> : null}
    </aside>
  )
}
