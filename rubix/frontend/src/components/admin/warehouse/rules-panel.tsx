// Warehouse projection rules panel. Lists `rubix.warehouse.rule.*`
// projection rules, supports inline DDL editing via `useRuleWrite`,
// and "soft delete" via `useRuleWrite` with an empty/`-- deleted`
// DDL marker (no dedicated drop verb exists today — see stage 9
// BLOCKED handover).

import { useState } from 'react'
import { useIntl } from 'react-intl'
import {
  Button,
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  Skeleton,
  Textarea,
} from '@nube/starter-ui-kit'
import { Database, Pencil, Trash2 } from 'lucide-react'
import {
  useWarehouseRulesList,
  useRuleWrite,
} from '@nube/rubix-client-react'

export function WarehouseRulesPanel() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const list = useWarehouseRulesList()
  const write = useRuleWrite()
  const [editing, setEditing] = useState<string | null>(null)
  const [draftDdl, setDraftDdl] = useState('')

  const rows = list.data?.rules ?? []

  function startEdit(name: string, ddl: string) {
    setEditing(name)
    setDraftDdl(ddl)
  }

  async function save(name: string) {
    await write.mutateAsync({ rule_name: name, ddl: draftDdl })
    setEditing(null)
  }

  async function softDelete(name: string) {
    if (
      !window.confirm(
        tr(
          'admin.warehouse.rules.confirmDelete',
          'Soft-delete this projection rule? The DDL will be replaced with a deleted marker.',
        ),
      )
    )
      return
    await write.mutateAsync({
      rule_name: name,
      ddl: '-- soft-deleted via warehouse admin',
    })
  }

  if (list.isLoading) {
    return (
      <div className="space-y-3">
        <Skeleton className="h-12 w-full" />
        <Skeleton className="h-12 w-full" />
      </div>
    )
  }

  if (rows.length === 0) {
    return (
      <Empty>
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <Database />
          </EmptyMedia>
          <EmptyTitle>
            {tr('admin.warehouse.rules.empty.title', 'No projection rules')}
          </EmptyTitle>
          <EmptyDescription>
            {tr(
              'admin.warehouse.rules.empty.body',
              'Use the rubix.warehouse.rule.write tool to register a projection rule.',
            )}
          </EmptyDescription>
        </EmptyHeader>
      </Empty>
    )
  }

  return (
    <div className="space-y-3">
      {rows.map((r) => (
        <div
          key={r.rule_name}
          className="rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)] p-4"
        >
          <div className="flex items-center justify-between gap-3">
            <div className="font-mono text-sm font-medium">{r.rule_name}</div>
            <div className="flex gap-2">
              {editing === r.rule_name ? (
                <>
                  <Button
                    size="sm"
                    onClick={() => save(r.rule_name)}
                    disabled={write.isPending}
                  >
                    {tr('admin.warehouse.common.save', 'Save')}
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => setEditing(null)}
                  >
                    {tr('admin.warehouse.common.cancel', 'Cancel')}
                  </Button>
                </>
              ) : (
                <>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => startEdit(r.rule_name, r.ddl ?? '')}
                  >
                    <Pencil className="h-3.5 w-3.5" />
                    {tr('admin.warehouse.common.edit', 'Edit')}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => softDelete(r.rule_name)}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                    {tr('admin.warehouse.common.delete', 'Delete')}
                  </Button>
                </>
              )}
            </div>
          </div>
          {editing === r.rule_name ? (
            <Textarea
              className="mt-3 font-mono text-xs"
              rows={8}
              value={draftDdl}
              onChange={(e) => setDraftDdl(e.target.value)}
            />
          ) : (
            <pre className="mt-3 max-h-40 overflow-auto rounded-lg bg-[color:var(--color-surface-2)] p-3 font-mono text-xs text-[color:var(--color-muted)]">
              {r.ddl ?? ''}
            </pre>
          )}
        </div>
      ))}
    </div>
  )
}
