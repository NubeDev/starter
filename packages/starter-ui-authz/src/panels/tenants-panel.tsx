// Tenants panel — list + create + edit-name + audit-sample
// override. Read endpoint `GET /v1/tenants`; admin-only.
//
// Selection callback: callers (especially `<AuthzAdmin>`) listen
// to `onSelectTenant` so the Members/Teams panels can scope to a
// selected tenant id.

import { useState, type FormEvent } from "react";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Input,
  Label,
} from "@nube/starter-ui-kit";
import { useAuthzMessages } from "../i18n/context.js";
import { mergeAuthzMessages, type AuthzMessages } from "../i18n/messages.js";
import { useCreateTenant, usePatchTenant, useTenants } from "../hooks/index.js";
import { ActionsCell, DataTable, StateRow, Td } from "./_common.js";

export interface TenantsPanelProps {
  /** Optional i18n override merged on top of context messages. */
  i18n?: Partial<AuthzMessages>;
  /** Called when an operator clicks a tenant row. */
  onSelectTenant?: (tenantId: string) => void;
  /** Currently selected tenant id (highlights the row). */
  selectedTenantId?: string | null;
}

export function TenantsPanel({ i18n, onSelectTenant, selectedTenantId }: TenantsPanelProps) {
  const ctx = useAuthzMessages();
  const messages = i18n ? mergeAuthzMessages({ ...ctx, ...i18n }) : ctx;
  const m = messages;

  const list = useTenants();
  const create = useCreateTenant();
  const patch = usePatchTenant();

  const [slug, setSlug] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState("");

  async function onCreate(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!slug.trim() || !displayName.trim()) return;
    await create.mutateAsync({ slug: slug.trim(), display_name: displayName.trim() });
    setSlug("");
    setDisplayName("");
  }

  async function commitEdit() {
    if (!editingId) return;
    await patch.mutateAsync({ id: editingId, body: { display_name: editingName } });
    setEditingId(null);
  }

  return (
    <section className="grid gap-6">
      <header>
        <h2 className="text-xl font-semibold tracking-tight">{m.tenants.title}</h2>
        <p className="text-sm text-[color:var(--color-subtle,#6b7280)]">{m.tenants.description}</p>
      </header>

      <Card>
        <CardHeader>
          <CardTitle>{m.tenants.form.submit}</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={onCreate} className="grid grid-cols-1 gap-3 sm:grid-cols-[1fr_1fr_auto] sm:items-end">
            <div className="grid gap-1">
              <Label htmlFor="t-slug">{m.tenants.form.slugLabel}</Label>
              <Input
                id="t-slug"
                value={slug}
                onChange={(e) => setSlug(e.currentTarget.value)}
                placeholder={m.tenants.form.slugPlaceholder}
                required
              />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="t-name">{m.tenants.form.displayNameLabel}</Label>
              <Input
                id="t-name"
                value={displayName}
                onChange={(e) => setDisplayName(e.currentTarget.value)}
                placeholder={m.tenants.form.displayNamePlaceholder}
                required
              />
            </div>
            <Button type="submit" disabled={create.isPending}>{m.tenants.form.submit}</Button>
          </form>
          {create.error ? <p className="mt-2 text-xs text-[color:var(--color-danger,#dc2626)]">{create.error.message}</p> : null}
        </CardContent>
      </Card>

      {list.isLoading ? (
        <StateRow variant="loading">{m.common.loading}</StateRow>
      ) : list.error ? (
        <StateRow variant="error">{list.error.message || m.common.error}</StateRow>
      ) : (list.data?.length ?? 0) === 0 ? (
        <StateRow variant="empty">{m.common.empty}</StateRow>
      ) : (
        <DataTable
          label={m.tenants.title}
          headers={[
            m.tenants.columns.slug,
            m.tenants.columns.displayName,
            m.tenants.columns.auditSample,
            "",
          ]}
          rows={(list.data ?? []).map((t) => {
            const isSel = selectedTenantId === t.id;
            return (
              <tr
                key={t.id}
                className={
                  isSel
                    ? "bg-[color:var(--color-accent-soft,#eef2ff)] cursor-pointer"
                    : "cursor-pointer hover:bg-[color:var(--color-muted,#f9fafb)]"
                }
                onClick={() => onSelectTenant?.(t.id)}
              >
                <Td>
                  <code className="text-xs">{t.slug}</code>
                </Td>
                <Td>
                  {editingId === t.id ? (
                    <Input
                      autoFocus
                      value={editingName}
                      onChange={(e) => setEditingName(e.currentTarget.value)}
                      onClick={(e) => e.stopPropagation()}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") void commitEdit();
                        if (e.key === "Escape") setEditingId(null);
                      }}
                    />
                  ) : (
                    t.display_name
                  )}
                </Td>
                <Td>{t.audit_allow_sample ?? m.common.any}</Td>
                <ActionsCell>
                  {editingId === t.id ? (
                    <>
                      <Button
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          void commitEdit();
                        }}
                        disabled={patch.isPending}
                      >
                        {m.common.save}
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={(e) => {
                          e.stopPropagation();
                          setEditingId(null);
                        }}
                      >
                        {m.common.cancel}
                      </Button>
                    </>
                  ) : (
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={(e) => {
                        e.stopPropagation();
                        setEditingId(t.id);
                        setEditingName(t.display_name);
                      }}
                    >
                      {m.common.edit}
                    </Button>
                  )}
                </ActionsCell>
              </tr>
            );
          })}
        />
      )}
    </section>
  );
}
