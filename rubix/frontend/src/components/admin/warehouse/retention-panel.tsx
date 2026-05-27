// Warehouse retention panel. Lists every `system.tables` row
// from `useWarehouseTablesList`, exposes inline TTL editing via
// `useRetentionSet`.

import { useState } from 'react'
import { useIntl } from 'react-intl'
import {
  Button,
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  Input,
  Skeleton,
} from '@nube/starter-ui-kit'
import { Clock, Save } from 'lucide-react'
import {
  useWarehouseTablesList,
  useRetentionSet,
} from '@nube/rubix-client-react'

export function WarehouseRetentionPanel() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const list = useWarehouseTablesList()
  const setRetention = useRetentionSet()
  const [drafts, setDrafts] = useState<Record<string, string>>({})

  function draftFor(name: string, current: number | undefined): string {
    return drafts[name] ?? (current != null ? String(current) : '')
  }

  async function save(name: string) {
    const raw = drafts[name]
    if (raw == null) return
    const days = Number.parseInt(raw, 10)
    if (!Number.isFinite(days) || days < 0) return
    await setRetention.mutateAsync({ table_name: name, days })
    setDrafts((d) => {
      const next = { ...d }
      delete next[name]
      return next
    })
  }

  if (list.isLoading) {
    return (
      <div className="space-y-3">
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
      </div>
    )
  }

  const rows = list.data?.tables ?? []

  if (rows.length === 0) {
    return (
      <Empty>
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <Clock />
          </EmptyMedia>
          <EmptyTitle>
            {tr('admin.warehouse.retention.empty.title', 'No tables')}
          </EmptyTitle>
          <EmptyDescription>
            {tr(
              'admin.warehouse.retention.empty.body',
              'The warehouse has no user-owned tables yet.',
            )}
          </EmptyDescription>
        </EmptyHeader>
      </Empty>
    )
  }

  return (
    <div className="overflow-hidden rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)]">
      <table className="w-full text-left text-sm">
        <thead className="bg-[color:var(--color-surface-2)] text-[11px] uppercase tracking-[0.18em] text-[color:var(--color-muted)]">
          <tr>
            <th className="px-4 py-3 font-medium">
              {tr('admin.warehouse.retention.col.name', 'Table')}
            </th>
            <th className="px-4 py-3 font-medium">
              {tr('admin.warehouse.retention.col.engine', 'Engine')}
            </th>
            <th className="px-4 py-3 font-medium">
              {tr('admin.warehouse.retention.col.rows', 'Rows')}
            </th>
            <th className="px-4 py-3 font-medium">
              {tr('admin.warehouse.retention.col.ttl', 'TTL (days)')}
            </th>
            <th className="px-4 py-3" />
          </tr>
        </thead>
        <tbody>
          {rows.map((t) => {
            const draft = drafts[t.table_name]
            const dirty = draft != null && draft !== String(t.retention_days ?? '')
            return (
              <tr
                key={t.table_name}
                className="border-t border-[color:var(--color-border)]"
              >
                <td className="px-4 py-3 font-mono text-xs">{t.table_name}</td>
                <td className="px-4 py-3 text-[color:var(--color-muted)]">
                  {t.engine ?? '—'}
                </td>
                <td className="px-4 py-3 text-[color:var(--color-muted)]">
                  {t.row_count?.toLocaleString() ?? '—'}
                </td>
                <td className="px-4 py-3">
                  <Input
                    className="h-8 w-24"
                    type="number"
                    min={0}
                    value={draftFor(t.table_name, t.retention_days)}
                    onChange={(e) =>
                      setDrafts((d) => ({
                        ...d,
                        [t.table_name]: e.target.value,
                      }))
                    }
                  />
                </td>
                <td className="px-4 py-3 text-right">
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={!dirty || setRetention.isPending}
                    onClick={() => save(t.table_name)}
                  >
                    <Save className="h-3.5 w-3.5" />
                    {tr('admin.warehouse.common.save', 'Save')}
                  </Button>
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
