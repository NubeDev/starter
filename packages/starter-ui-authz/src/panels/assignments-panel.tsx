// Assignments panel — bind a subject (user id or glob) to a role.
// Smaller surface than rules: no conditions, no priorities, no
// per-resource scoping. The Rust handler reloads the engine cache
// after every write.

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
import {
  useAuthzAssignments,
  useCreateAuthzAssignment,
  useDeleteAuthzAssignment,
} from "../hooks/index.js";
import { ActionsCell, DataTable, StateRow, Td } from "./_common.js";

export interface AssignmentsPanelProps {
  i18n?: Partial<AuthzMessages>;
}

export function AssignmentsPanel({ i18n }: AssignmentsPanelProps) {
  const ctx = useAuthzMessages();
  const m = i18n ? mergeAuthzMessages({ ...ctx, ...i18n }) : ctx;

  const list = useAuthzAssignments();
  const create = useCreateAuthzAssignment();
  const del = useDeleteAuthzAssignment();

  const [subject, setSubject] = useState("");
  const [role, setRole] = useState("reader");

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!subject.trim() || !role.trim()) return;
    await create.mutateAsync({ subject: subject.trim(), role: role.trim() });
    setSubject("");
  }

  return (
    <section className="grid gap-6">
      <header>
        <h2 className="text-xl font-semibold tracking-tight">{m.assignments.title}</h2>
        <p className="text-sm text-[color:var(--color-subtle,#6b7280)]">{m.assignments.description}</p>
      </header>

      <Card>
        <CardHeader>
          <CardTitle>{m.assignments.form.submit}</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="grid grid-cols-1 gap-3 sm:grid-cols-[2fr_1fr_auto] sm:items-end">
            <div className="grid gap-1">
              <Label htmlFor="a-subj">{m.assignments.form.subjectLabel}</Label>
              <Input
                id="a-subj"
                value={subject}
                onChange={(e) => setSubject(e.currentTarget.value)}
                placeholder={m.assignments.form.subjectPlaceholder}
                required
              />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="a-role">{m.assignments.form.roleLabel}</Label>
              <Input id="a-role" value={role} onChange={(e) => setRole(e.currentTarget.value)} required />
            </div>
            <Button type="submit" disabled={create.isPending}>{m.assignments.form.submit}</Button>
          </form>
          {create.error ? <p className="mt-2 text-xs text-[color:var(--color-danger,#dc2626)]">{create.error.message}</p> : null}
        </CardContent>
      </Card>

      {list.isLoading ? (
        <StateRow variant="loading">{m.common.loading}</StateRow>
      ) : list.error ? (
        <StateRow variant="error">{list.error.message || m.common.error}</StateRow>
      ) : (list.data?.assignments.length ?? 0) === 0 ? (
        <StateRow variant="empty">{m.common.empty}</StateRow>
      ) : (
        <DataTable
          label={m.assignments.title}
          headers={[m.common.subject, m.common.role, m.common.createdBy, ""]}
          rows={(list.data?.assignments ?? []).map((a) => (
            <tr key={a.id}>
              <Td><code className="text-xs">{a.subject}</code></Td>
              <Td>{a.role}</Td>
              <Td><code className="text-xs">{a.created_by}</code></Td>
              <ActionsCell>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    if (!window.confirm(m.common.confirmDelete)) return;
                    void del.mutateAsync(a.id);
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
