// Rules panel — `/v1/authz/rules`. The Rust handler validates
// effect ∈ {"allow","deny"} and reloads the engine cache after
// every write; the UI just funnels the body through.

import { useMemo, useState, type FormEvent } from "react";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Input,
  Label,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit";
import { useAuthzMessages } from "../i18n/context.js";
import { mergeAuthzMessages, type AuthzMessages } from "../i18n/messages.js";
import {
  useAuthzResources,
  useAuthzRules,
  useCreateAuthzRule,
  useDeleteAuthzRule,
  useTenants,
} from "../hooks/index.js";
import type { RuleEffect } from "@nube/starter-client-ts";
import { ActionsCell, DataTable, StateRow, Td } from "./_common.js";

const ROLE_HINTS = ["reader", "writer", "admin"];

export interface RulesPanelProps {
  i18n?: Partial<AuthzMessages>;
}

export function RulesPanel({ i18n }: RulesPanelProps) {
  const ctx = useAuthzMessages();
  const m = i18n ? mergeAuthzMessages({ ...ctx, ...i18n }) : ctx;

  const list = useAuthzRules();
  const resources = useAuthzResources();
  const tenants = useTenants();
  const create = useCreateAuthzRule();
  const del = useDeleteAuthzRule();

  const [role, setRole] = useState("reader");
  const [resource, setResource] = useState("*");
  const [actions, setActions] = useState("*");
  const [condition, setCondition] = useState("");
  const [effect, setEffect] = useState<RuleEffect>("allow");
  const [priority, setPriority] = useState(0);
  const [tenantId, setTenantId] = useState<string>("");

  const resourceOptions = useMemo(
    () => ["*", ...(resources.data?.resources?.map((r) => r.kind) ?? [])],
    [resources.data],
  );

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    await create.mutateAsync({
      role: role.trim(),
      resource: resource.trim(),
      actions: actions
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean),
      condition: condition.trim() || null,
      effect,
      priority,
      tenant_id: tenantId || null,
    });
    setActions("*");
    setCondition("");
    setPriority(0);
  }

  return (
    <section className="grid gap-6">
      <header>
        <h2 className="text-xl font-semibold tracking-tight">{m.rules.title}</h2>
        <p className="text-sm text-[color:var(--color-subtle,#6b7280)]">{m.rules.description}</p>
      </header>

      <Card>
        <CardHeader>
          <CardTitle>{m.rules.form.submit}</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <div className="grid gap-1">
              <Label htmlFor="r-role">{m.rules.form.roleLabel}</Label>
              <Input
                id="r-role"
                list="role-hints"
                value={role}
                onChange={(e) => setRole(e.currentTarget.value)}
                required
              />
              <datalist id="role-hints">
                {ROLE_HINTS.map((r) => <option key={r} value={r} />)}
              </datalist>
            </div>
            <div className="grid gap-1">
              <Label htmlFor="r-res">{m.rules.form.resourceLabel}</Label>
              <Select value={resource} onValueChange={setResource}>
                <SelectTrigger id="r-res"><SelectValue /></SelectTrigger>
                <SelectContent>
                  {resourceOptions.map((k) => (
                    <SelectItem key={k} value={k}>{k}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="grid gap-1">
              <Label htmlFor="r-act">{m.rules.form.actionsLabel}</Label>
              <Input
                id="r-act"
                value={actions}
                onChange={(e) => setActions(e.currentTarget.value)}
                placeholder={m.rules.form.actionsPlaceholder}
                required
              />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="r-eff">{m.rules.form.effectLabel}</Label>
              <Select value={effect} onValueChange={(v) => setEffect(v as RuleEffect)}>
                <SelectTrigger id="r-eff"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="allow">{m.common.allow}</SelectItem>
                  <SelectItem value="deny">{m.common.deny}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="grid gap-1">
              <Label htmlFor="r-cond">{m.rules.form.conditionLabel}</Label>
              <Input
                id="r-cond"
                value={condition}
                onChange={(e) => setCondition(e.currentTarget.value)}
                placeholder={m.rules.form.conditionPlaceholder}
              />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="r-prio">{m.rules.form.priorityLabel}</Label>
              <Input
                id="r-prio"
                type="number"
                value={priority}
                onChange={(e) => setPriority(Number(e.currentTarget.value) || 0)}
              />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="r-tenant">{m.rules.form.tenantLabel}</Label>
              <Select value={tenantId || "__global__"} onValueChange={(v) => setTenantId(v === "__global__" ? "" : v)}>
                <SelectTrigger id="r-tenant"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="__global__">{m.rules.form.tenantPlaceholderGlobal}</SelectItem>
                  {(tenants.data ?? []).map((t) => (
                    <SelectItem key={t.id} value={t.id}>{t.slug}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-end">
              <Button type="submit" disabled={create.isPending} className="w-full">
                {m.rules.form.submit}
              </Button>
            </div>
          </form>
          {create.error ? <p className="mt-2 text-xs text-[color:var(--color-danger,#dc2626)]">{create.error.message}</p> : null}
        </CardContent>
      </Card>

      {list.isLoading ? (
        <StateRow variant="loading">{m.common.loading}</StateRow>
      ) : list.error ? (
        <StateRow variant="error">{list.error.message || m.common.error}</StateRow>
      ) : (list.data?.rules.length ?? 0) === 0 ? (
        <StateRow variant="empty">{m.common.empty}</StateRow>
      ) : (
        <DataTable
          label={m.rules.title}
          headers={[
            m.common.role,
            m.common.resource,
            m.common.action,
            m.common.effect,
            m.common.priority,
            m.common.tenant,
            "",
          ]}
          rows={(list.data?.rules ?? []).map((r) => (
            <tr key={r.id}>
              <Td>{r.role}</Td>
              <Td><code className="text-xs">{r.resource}</code></Td>
              <Td><code className="text-xs">{r.actions.join(", ")}</code></Td>
              <Td>
                <span
                  className={
                    r.effect === "allow"
                      ? "rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-700"
                      : "rounded-full bg-rose-100 px-2 py-0.5 text-xs font-medium text-rose-700"
                  }
                >
                  {r.effect === "allow" ? m.common.allow : m.common.deny}
                </span>
              </Td>
              <Td>{r.priority}</Td>
              <Td>{r.tenant_id ?? m.rules.form.tenantPlaceholderGlobal}</Td>
              <ActionsCell>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    if (!window.confirm(m.common.confirmDelete)) return;
                    void del.mutateAsync(r.id);
                  }}
                  disabled={del.isPending}
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
