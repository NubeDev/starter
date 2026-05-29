// `<UsersListPanel>` — the "Users" rail-node content. Lists every
// rubix system user, with a Create user form and a per-row
// Disable action. Picking a row asks the parent shell to switch
// to that user's detail.
//
// Users vs Members: a "user" is a rubix account (system-wide).
// A "member" binds a user to a role inside one tenant.

import { useState, type FormEvent } from "react";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
  Input,
  Label,
  Skeleton,
} from "@nube/starter-ui-kit";
import { useUserOps, type UsersAdminOps } from "./user-ops.js";
import { StateRow } from "./_common.js";

export interface UsersListPanelProps {
  /** Optional explicit override; falls back to `useUserOps()`. */
  userOps?: UsersAdminOps;
  /** Selected user id (for row highlight). */
  selectedUserId?: string | null;
  /** Row click handler — parent shell should switch detail. */
  onSelectUser?: (userId: string) => void;
}

export function UsersListPanel({
  userOps,
  selectedUserId,
  onSelectUser,
}: UsersListPanelProps) {
  const ctxOps = useUserOps();
  const ops = userOps ?? ctxOps;

  const [email, setEmail] = useState("");
  const [role, setRole] = useState("operator");

  if (!ops) {
    return <StateRow variant="error">UserOps not provided.</StateRow>;
  }

  const data = ops.list();
  const users = data?.users ?? [];
  const isLoading = data === undefined;

  async function onCreate(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!ops?.create) return;
    await ops.create({ email, role });
    setEmail("");
  }

  return (
    <div className="grid gap-6">
      <header className="flex items-end justify-between gap-4">
        <div>
          <h2 className="text-2xl font-semibold tracking-tight">Users</h2>
          <p className="mt-1 text-sm text-[color:var(--color-subtle)]">
            Users are rubix accounts (separate from tenant membership).
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          disabled={!ops.undoLast || ops.isUndoing}
          onClick={() => ops.undoLast?.()}
        >
          Undo last
        </Button>
      </header>

      {ops.create && (
        <Card>
          <CardHeader>
            <CardTitle>Create user</CardTitle>
          </CardHeader>
          <CardContent>
            <form
              onSubmit={onCreate}
              className="grid grid-cols-1 gap-4 sm:grid-cols-[2fr_1fr_auto] sm:items-end"
            >
              <div className="grid gap-2">
                <Label htmlFor="user-email">Email</Label>
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
                <Label htmlFor="user-role">Role</Label>
                <Input
                  id="user-role"
                  required
                  value={role}
                  onChange={(e) => setRole(e.currentTarget.value)}
                />
              </div>
              <Button type="submit" disabled={ops.isCreating}>
                Create
              </Button>
            </form>
          </CardContent>
        </Card>
      )}

      <div className="glass overflow-hidden rounded-3xl">
        <div className="grid grid-cols-[2fr_1fr_1fr_auto] gap-4 border-b border-[color:var(--color-border)] px-6 py-3 text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
          <div>Email</div>
          <div>Role</div>
          <div>Status</div>
          <div className="text-right">Actions</div>
        </div>
        {isLoading ? (
          <div className="space-y-3 p-4">
            <Skeleton className="h-12 w-full" />
            <Skeleton className="h-12 w-full" />
            <Skeleton className="h-12 w-full" />
          </div>
        ) : users.length === 0 ? (
          <Empty>
            <EmptyHeader>
              <EmptyTitle>No users yet</EmptyTitle>
              <EmptyDescription>
                Create the first user with the form above and they will
                appear here.
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        ) : (
          users.map((u) => {
            const disabled = u.disabled_at_ms != null;
            const isSelected = selectedUserId === u.user_id;
            return (
              <div
                key={u.user_id}
                className={
                  "grid cursor-pointer grid-cols-[2fr_1fr_1fr_auto] items-center gap-4 border-b border-[color:var(--color-border)]/50 px-6 py-4 last:border-b-0 hover:bg-[color:var(--color-border)]/20" +
                  (isSelected ? " bg-[color:var(--color-border)]/30" : "")
                }
                onClick={() => onSelectUser?.(u.user_id)}
              >
                <div className="font-medium">{u.email}</div>
                <div className="text-sm text-[color:var(--color-muted)]">
                  {u.role}
                </div>
                <div className="text-sm">
                  {disabled ? "Disabled" : "Active"}
                </div>
                <div className="flex justify-end">
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={disabled || !ops.disable || ops.isDisabling}
                    onClick={(e) => {
                      e.stopPropagation();
                      ops.disable?.(u.user_id);
                    }}
                  >
                    Disable
                  </Button>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
