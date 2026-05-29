// Dry-run check panel — `POST /v1/authz/check`. Lets an operator
// preview a decision before committing rule edits.

import { useEffect, useState, type FormEvent } from "react";
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
import { useAuthzCheck, useAuthzResources } from "../hooks/index.js";

type SimRole = "reader" | "writer" | "admin";

export interface CheckPanelProps {
  i18n?: Partial<AuthzMessages>;
  /** Prefill `Principal subject`. */
  defaultSubject?: string | null;
  /** Reserved for future tenant-scoping (no field today). */
  defaultTenantId?: string | null;
}

export function CheckPanel({ i18n, defaultSubject }: CheckPanelProps) {
  const ctx = useAuthzMessages();
  const m = i18n ? mergeAuthzMessages({ ...ctx, ...i18n }) : ctx;

  const resources = useAuthzResources();
  const check = useAuthzCheck();

  const [subject, setSubject] = useState(defaultSubject || "user-1");

  useEffect(() => {
    if (defaultSubject) setSubject(defaultSubject);
  }, [defaultSubject]);
  const [role, setRole] = useState<SimRole>("reader");
  const [action, setAction] = useState("read");
  const [kind, setKind] = useState("");
  const [id, setId] = useState("");
  const [owner, setOwner] = useState("");

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!kind.trim() || !action.trim()) return;
    await check.mutateAsync({
      principal: { subject: subject.trim(), role },
      action: action.trim(),
      resource: {
        kind: kind.trim(),
        id: id.trim() || null,
        owner: owner.trim() || null,
      },
    });
  }

  const decision = check.data;

  return (
    <section className="grid gap-6">
      <header>
        <h2 className="text-xl font-semibold tracking-tight">{m.check.title}</h2>
        <p className="text-sm text-[color:var(--color-subtle,#6b7280)]">{m.check.description}</p>
      </header>

      <Card>
        <CardHeader>
          <CardTitle>{m.check.submit}</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
            <div className="grid gap-1">
              <Label htmlFor="c-subj">{m.check.principalSubjectLabel}</Label>
              <Input id="c-subj" value={subject} onChange={(e) => setSubject(e.currentTarget.value)} required />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="c-role">{m.check.principalRoleLabel}</Label>
              <Select value={role} onValueChange={(v) => setRole(v as SimRole)}>
                <SelectTrigger id="c-role"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="reader">reader</SelectItem>
                  <SelectItem value="writer">writer</SelectItem>
                  <SelectItem value="admin">admin</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="grid gap-1">
              <Label htmlFor="c-act">{m.check.actionLabel}</Label>
              <Input id="c-act" value={action} onChange={(e) => setAction(e.currentTarget.value)} required />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="c-kind">{m.check.resourceKindLabel}</Label>
              <Select value={kind || "__pick__"} onValueChange={(v) => setKind(v === "__pick__" ? "" : v)}>
                <SelectTrigger id="c-kind"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="__pick__">—</SelectItem>
                  {(resources.data?.resources ?? []).map((r) => (
                    <SelectItem key={r.kind} value={r.kind}>{r.kind}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="grid gap-1">
              <Label htmlFor="c-id">{m.check.resourceIdLabel}</Label>
              <Input id="c-id" value={id} onChange={(e) => setId(e.currentTarget.value)} />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="c-own">{m.check.resourceOwnerLabel}</Label>
              <Input id="c-own" value={owner} onChange={(e) => setOwner(e.currentTarget.value)} />
            </div>
            <div className="flex items-end">
              <Button type="submit" disabled={check.isPending || !kind} className="w-full">
                {m.check.submit}
              </Button>
            </div>
          </form>
          {check.error ? (
            <p className="mt-3 text-xs text-[color:var(--color-danger,#dc2626)]">{check.error.message}</p>
          ) : null}
          {decision ? (
            <div className="mt-4 rounded-2xl border border-[color:var(--color-border,#e5e7eb)] p-4 text-sm">
              <div className="flex items-center gap-3">
                <span
                  className={
                    decision.decision === "allow"
                      ? "rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-semibold uppercase tracking-wider text-emerald-700"
                      : "rounded-full bg-rose-100 px-2 py-0.5 text-xs font-semibold uppercase tracking-wider text-rose-700"
                  }
                >
                  {decision.decision === "allow" ? m.check.decisionAllow : m.check.decisionDeny}
                </span>
                {decision.reason ? (
                  <span className="text-[color:var(--color-subtle,#6b7280)]">{decision.reason}</span>
                ) : null}
              </div>
              {decision.matched_rule ? (
                <p className="mt-2 text-xs">
                  {m.check.matchedRule}: <code>{decision.matched_rule}</code>
                </p>
              ) : null}
            </div>
          ) : null}
        </CardContent>
      </Card>
    </section>
  );
}
