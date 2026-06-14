// Members panel — manage memberships of one tenant. The server
// has no `GET /v1/tenants/{id}/members` listing endpoint today, so
// the panel works in two modes:
//
//   1. **uncontrolled** — pure write form (add / patch role /
//      remove by user id). Used when the host has no user
//      directory to enumerate.
//   2. **controlled** — pass `members` from the host (e.g. a
//      `useTenantMembers()` hook backed by the host's own users
//      API). The panel renders the row list and the row-level
//      actions exactly as before.
//
// Adding a member lives behind a "+" button that opens a dialog (rather than
// a permanent inline form). Hosts can inject extra add-paths into that dialog
// via `addExtra` — e.g. an "invite a brand-new user account" form.

import { useState, type FormEvent, type ReactNode } from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  Label,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit";
import { Plus } from "lucide-react";
import { useAuthzMessages } from "../i18n/context.js";
import { mergeAuthzMessages, type AuthzMessages } from "../i18n/messages.js";
import type { MembershipView, TenantRole } from "@nube/starter-client-ts";
import {
  useAddTenantMember,
  usePatchTenantMember,
  useRemoveTenantMember,
} from "../hooks/index.js";
import { ActionsCell, DataTable, StateRow, Td } from "./_common.js";
import { UserPicker, UserPickerFallback, useUserDirectory } from "./user-picker.js";

const ROLES: TenantRole[] = ["reader", "writer", "admin"];

export interface MembersPanelProps {
  /** Tenant whose members are being managed. */
  tenantId: string | null;
  /** Optional host-supplied membership list. When `undefined` the
   * panel renders write-only (no row list). */
  members?: MembershipView[];
  /** Loading + error pass-through for host-supplied lists. */
  membersLoading?: boolean;
  membersError?: Error | null;
  /** Extra content rendered inside the "Add member" dialog — e.g. a host's
   * "invite a brand-new user account" form. Shown below the add-existing form. */
  addExtra?: ReactNode;
  /** i18n override. */
  i18n?: Partial<AuthzMessages>;
}

export function MembersPanel({
  tenantId,
  members,
  membersLoading,
  membersError,
  addExtra,
  i18n,
}: MembersPanelProps) {
  const ctx = useAuthzMessages();
  const m = i18n ? mergeAuthzMessages({ ...ctx, ...i18n }) : ctx;

  const patch = usePatchTenantMember();
  const remove = useRemoveTenantMember();

  if (!tenantId) {
    return (
      <section className="grid gap-6">
        <Header title={m.members.title} description={m.members.description} />
        <StateRow variant="empty">{m.members.selectTenantPrompt}</StateRow>
      </section>
    );
  }

  return (
    <section className="grid gap-6">
      <div className="flex items-start justify-between gap-4">
        <Header title={m.members.title} description={m.members.description} />
        <AddMemberDialog tenantId={tenantId} m={m} addExtra={addExtra} />
      </div>

      {members === undefined ? null : membersLoading ? (
        <StateRow variant="loading">{m.common.loading}</StateRow>
      ) : membersError ? (
        <StateRow variant="error">{membersError.message || m.common.error}</StateRow>
      ) : members.length === 0 ? (
        <StateRow variant="empty">{m.common.empty}</StateRow>
      ) : (
        <DataTable
          label={m.members.title}
          headers={[m.members.columns.user, m.common.role, ""]}
          rows={members.map((mem) => (
            <tr key={`${mem.tenant_id}:${mem.user_id}`}>
              <Td>
                {mem.email ? mem.email : <code className="text-xs">{mem.user_id}</code>}
              </Td>
              <Td>
                <Select
                  value={mem.role}
                  onValueChange={(v) => {
                    void patch.mutateAsync({
                      tenantId,
                      userId: mem.user_id,
                      body: { role: v as TenantRole },
                    });
                  }}
                >
                  <SelectTrigger className="h-8 w-32">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {ROLES.map((r) => (
                      <SelectItem key={r} value={r}>
                        {m.roleLabels?.[r] ?? r}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Td>
              <ActionsCell>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    if (!window.confirm(m.common.confirmDelete)) return;
                    void remove.mutateAsync({ tenantId, userId: mem.user_id });
                  }}
                  disabled={remove.isPending}
                >
                  {m.common.delete}
                </Button>
              </ActionsCell>
            </tr>
          ))}
        />
      )}
    </section>
  );
}

function Header({ title, description }: { title: string; description: string }) {
  return (
    <header>
      <h2 className="text-xl font-semibold tracking-tight">{title}</h2>
      <p className="text-sm text-[color:var(--color-subtle,#6b7280)]">{description}</p>
    </header>
  );
}

function AddMemberDialog({
  tenantId,
  m,
  addExtra,
}: {
  tenantId: string;
  m: AuthzMessages;
  addExtra?: ReactNode;
}) {
  const directory = useUserDirectory();
  const add = useAddTenantMember();
  const [open, setOpen] = useState(false);
  const [userId, setUserId] = useState("");
  const [role, setRole] = useState<TenantRole>("reader");

  async function onAdd(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!userId.trim()) return;
    await add.mutateAsync({ tenantId, body: { user_id: userId.trim(), role } });
    setUserId("");
    setRole("reader");
    setOpen(false);
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button size="sm">
          <Plus className="size-4" />
          {m.members.form.submit}
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{m.members.form.submit}</DialogTitle>
          <DialogDescription>{m.members.description}</DialogDescription>
        </DialogHeader>
        <form onSubmit={onAdd} className="grid gap-4">
          <div className="grid gap-1">
            <Label htmlFor="m-user">{m.members.form.userIdLabel}</Label>
            {directory ? (
              <UserPicker
                id="m-user"
                value={userId || null}
                onChange={(sel) => setUserId(sel?.kind === "user" ? sel.id : "")}
                userDirectory={directory}
                enableGlobMode={false}
                placeholder={m.members.form.userIdPlaceholder}
              />
            ) : (
              <UserPickerFallback
                id="m-user"
                value={userId}
                onChange={setUserId}
                placeholder={m.members.form.userIdPlaceholder}
                required
              />
            )}
          </div>
          <div className="grid gap-1">
            <Label htmlFor="m-role">{m.members.form.roleLabel}</Label>
            <Select value={role} onValueChange={(v) => setRole(v as TenantRole)}>
              <SelectTrigger id="m-role">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ROLES.map((r) => (
                  <SelectItem key={r} value={r}>
                    {m.roleLabels?.[r] ?? r}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {add.error ? (
            <p className="text-xs text-[color:var(--color-danger,#dc2626)]">{add.error.message}</p>
          ) : null}
          <DialogFooter>
            <Button type="submit" disabled={add.isPending}>
              {add.isPending ? `${m.members.form.submit}…` : m.members.form.submit}
            </Button>
          </DialogFooter>
        </form>

        {addExtra ? (
          <div className="mt-2 border-t pt-4">{addExtra}</div>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
