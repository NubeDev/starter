// ClickHouse marts panel. Lists materialised marts, supports
// creating a new mart via `useMartCreate` (modal with name + DDL),
// and dropping a mart via `useWarehouseMartDrop` with a hard
// data-loss warning surfaced through the kit's `<AlertDialog>`
// (no `window.confirm`, per the explorer scope's code-standard
// checklist).

import { useState } from 'react'
import { useIntl } from 'react-intl'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  Input,
  Label,
  Skeleton,
  Textarea,
} from '@nube/starter-ui-kit'
import { Boxes, Plus, Trash2, AlertTriangle } from 'lucide-react'
import {
  useWarehouseMartDrop,
  useWarehouseMartsList,
  useMartCreate,
} from '@nube/rubix-client-react'

export function WarehouseMartsPanel() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const list = useWarehouseMartsList()
  const create = useMartCreate()
  const drop = useWarehouseMartDrop()

  const [open, setOpen] = useState(false)
  const [name, setName] = useState('')
  const [ddl, setDdl] = useState('')
  const [pendingDrop, setPendingDrop] = useState<string | null>(null)

  async function submit() {
    if (!name.trim() || !ddl.trim()) return
    await create.mutateAsync({ mart_name: name.trim(), ddl })
    setOpen(false)
    setName('')
    setDdl('')
  }

  async function confirmDrop() {
    if (!pendingDrop) return
    const target = pendingDrop
    setPendingDrop(null)
    await drop.mutateAsync({ mart_name: target })
  }

  const rows = list.data?.marts ?? []

  return (
    <div className="space-y-4">
      <div className="flex justify-end">
        <Button size="sm" onClick={() => setOpen(true)}>
          <Plus className="h-3.5 w-3.5" />
          {tr('admin.warehouse.marts.create', 'New mart')}
        </Button>
      </div>

      {list.isLoading ? (
        <div className="space-y-3">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : rows.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <Boxes />
            </EmptyMedia>
            <EmptyTitle>
              {tr('admin.warehouse.marts.empty.title', 'No marts')}
            </EmptyTitle>
            <EmptyDescription>
              {tr(
                'admin.warehouse.marts.empty.body',
                'Create a mart to materialise an L1–L3 aggregate.',
              )}
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <div className="space-y-3">
          {rows.map((m) => (
            <div
              key={m.mart_name}
              className="flex items-start justify-between gap-3 rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)] p-4"
            >
              <div className="min-w-0 flex-1">
                <div className="font-mono text-sm font-medium">
                  {m.mart_name}
                </div>
                {m.ddl ? (
                  <pre className="mt-2 max-h-32 overflow-auto rounded-lg bg-[color:var(--color-surface-2)] p-2 font-mono text-xs text-[color:var(--color-muted)]">
                    {m.ddl}
                  </pre>
                ) : null}
              </div>
              <Button
                size="sm"
                variant="destructive"
                onClick={() => setPendingDrop(m.mart_name)}
              >
                <Trash2 className="h-3.5 w-3.5" />
                {tr('admin.warehouse.marts.drop', 'Drop')}
              </Button>
            </div>
          ))}
        </div>
      )}

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {tr('admin.warehouse.marts.createTitle', 'Create mart')}
            </DialogTitle>
            <DialogDescription className="flex items-center gap-2 text-xs">
              <AlertTriangle className="h-3.5 w-3.5" />
              {tr(
                'admin.warehouse.marts.createWarning',
                'DDL is executed verbatim against ClickHouse.',
              )}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div>
              <Label>{tr('admin.warehouse.marts.nameLabel', 'Name')}</Label>
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="my_mart_l1"
              />
            </div>
            <div>
              <Label>{tr('admin.warehouse.marts.ddlLabel', 'DDL')}</Label>
              <Textarea
                className="font-mono text-xs"
                rows={10}
                value={ddl}
                onChange={(e) => setDdl(e.target.value)}
                placeholder="CREATE TABLE my_mart_l1 ..."
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setOpen(false)}>
              {tr('admin.warehouse.common.cancel', 'Cancel')}
            </Button>
            <Button onClick={submit} disabled={create.isPending}>
              {tr('admin.warehouse.common.create', 'Create')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog
        open={pendingDrop !== null}
        onOpenChange={(o) => {
          if (!o) setPendingDrop(null)
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {tr(
                'admin.warehouse.marts.confirmDropTitle',
                'Drop mart "{name}"?',
              ).replace('{name}', pendingDrop ?? '')}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {tr(
                'admin.warehouse.marts.confirmDropBody',
                'This deletes the underlying table and all its data. This action cannot be undone.',
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>
              {tr('admin.warehouse.common.cancel', 'Cancel')}
            </AlertDialogCancel>
            <AlertDialogAction onClick={confirmDrop}>
              {tr('admin.warehouse.marts.drop', 'Drop')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
