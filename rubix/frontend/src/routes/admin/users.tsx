// `/admin/users` — admin user-management surface.
//
// Exercises the write + undo path of the rubix-frontend-wire job:
//   - `useUserList()` reads the current roster.
//   - `useUserCreate()` mounts a small form; success invalidates the
//      `['rubix','users']` query prefix so the list re-fetches.
//   - `useUserDisable()` runs per-row and invalidates the same key.
//   - `useUndoLast()` is wired to the page header so an operator can
//     reverse the most-recent mutating tool call (create *or*
//     disable) and watch the list snap back.
//
// All four hooks throw `RubixError` on failure; the surrounding
// `<ErrorBoundary>` catches and localises the diagnostic code.

import { createFileRoute } from '@tanstack/react-router'
import { useState, type FormEvent } from 'react'
import { useIntl } from 'react-intl'
import { Undo2, UserPlus, Ban, Users } from 'lucide-react'
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  Input,
  Label,
  Skeleton,
} from '@nube/starter-ui-kit'
import {
  useUserList,
  useUserCreate,
  useUserDisable,
  useUndoLast,
} from '@nube/rubix-client-react'
import { ErrorBoundary } from '@/components/error-boundary'

function UsersPanel() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const list = useUserList()
  const create = useUserCreate()
  const disable = useUserDisable()
  const undo = useUndoLast()

  const [email, setEmail] = useState('')
  const [role, setRole] = useState('operator')

  async function onCreate(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    await create.mutateAsync({ email, role })
    setEmail('')
  }

  const users = list.data?.users ?? []

  return (
    <section className="relative mx-auto max-w-5xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <header className="mb-8 flex items-end justify-between gap-4">
        <div>
          <div className="flex items-center gap-3">
            <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
            <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
              {tr('users.eyebrow', 'Admin')}
            </span>
          </div>
          <h1 className="mt-3 text-4xl font-medium tracking-[-0.03em]">
            {tr('users.title', 'Users')}
          </h1>
        </div>
        <Button
          variant="outline"
          size="sm"
          disabled={undo.isPending}
          onClick={() => undo.mutate({})}
        >
          <Undo2 className="h-3.5 w-3.5" />
          {tr('users.undoLast', 'Undo last')}
        </Button>
      </header>

      <Card className="mb-6">
        <CardHeader>
          <CardTitle>{tr('users.create.title', 'Create user')}</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={onCreate} className="grid grid-cols-1 gap-4 sm:grid-cols-[2fr_1fr_auto] sm:items-end">
            <div className="grid gap-2">
              <Label htmlFor="user-email">{tr('users.field.email', 'Email')}</Label>
              <Input
                id="user-email"
                type="email"
                required
                value={email}
                onChange={(e) => setEmail(e.currentTarget.value)}
                placeholder="user@example.com"
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="user-role">{tr('users.field.role', 'Role')}</Label>
              <Input
                id="user-role"
                required
                value={role}
                onChange={(e) => setRole(e.currentTarget.value)}
              />
            </div>
            <Button type="submit" disabled={create.isPending}>
              <UserPlus className="h-4 w-4" />
              {tr('users.create.submit', 'Create')}
            </Button>
          </form>
        </CardContent>
      </Card>

      <div className="glass overflow-hidden rounded-3xl">
        <div className="grid grid-cols-[2fr_1fr_1fr_auto] gap-4 border-b border-[color:var(--color-border)] px-6 py-3 text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
          <div>{tr('users.col.email', 'Email')}</div>
          <div>{tr('users.col.role', 'Role')}</div>
          <div>{tr('users.col.status', 'Status')}</div>
          <div className="text-right">{tr('users.col.actions', 'Actions')}</div>
        </div>
        {list.isLoading ? (
          <div className="space-y-3 p-4">
            <Skeleton className="h-12 w-full" />
            <Skeleton className="h-12 w-full" />
            <Skeleton className="h-12 w-full" />
          </div>
        ) : users.length === 0 ? (
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <Users />
              </EmptyMedia>
              <EmptyTitle>
                {tr('users.empty.title', 'No users yet')}
              </EmptyTitle>
              <EmptyDescription>
                {tr(
                  'users.empty.body',
                  'Create the first user with the form above and they will appear here.',
                )}
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        ) : (
          users.map((u) => {
            const disabled = u.disabled_at_ms != null
            return (
              <div
                key={u.user_id}
                className="grid grid-cols-[2fr_1fr_1fr_auto] items-center gap-4 border-b border-[color:var(--color-border)]/50 px-6 py-4 last:border-b-0"
              >
                <div className="font-medium">{u.email}</div>
                <div className="text-sm text-[color:var(--color-muted)]">{u.role}</div>
                <div className="text-sm">
                  {disabled
                    ? tr('users.status.disabled', 'Disabled')
                    : tr('users.status.active', 'Active')}
                </div>
                <div className="flex justify-end">
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={disabled || disable.isPending}
                    onClick={() => disable.mutate({ user_id: u.user_id })}
                  >
                    <Ban className="h-3.5 w-3.5" />
                    {tr('users.action.disable', 'Disable')}
                  </Button>
                </div>
              </div>
            )
          })
        )}
      </div>
    </section>
  )
}

function UsersRoute() {
  return (
    <ErrorBoundary>
      <UsersPanel />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/admin/users')({ component: UsersRoute })
