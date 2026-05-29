// Assignments panel — bind a subject (user id or glob) to a role.
// Smaller surface than rules: no conditions, no priorities, no
// per-resource scoping. The Rust handler reloads the engine cache
// after every write.

import { useEffect, useState, type FormEvent } from "react";
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
  useTeams,
} from "../hooks/index.js";
import { ActionsCell, DataTable, StateRow, Td } from "./_common.js";
import { UserPicker, UserPickerFallback, useUserDirectory } from "./user-picker.js";

export interface AssignmentsPanelProps {
  i18n?: Partial<AuthzMessages>;
  /** Optional tenant scope — used to populate the team mode of
   *  the subject picker. */
  tenantId?: string | null;
  /** When true the picker exposes a Team segment. */
  enableTeamMode?: boolean;
  /** Prefill the subject picker (master-detail scope: user/team detail panes). */
  defaultSubject?: string | null;
}

export function AssignmentsPanel({ i18n, tenantId, enableTeamMode, defaultSubject }: AssignmentsPanelProps) {
  const ctx = useAuthzMessages();
  const m = i18n ? mergeAuthzMessages({ ...ctx, ...i18n }) : ctx;

  const list = useAuthzAssignments();
  const create = useCreateAuthzAssignment();
  const del = useDeleteAuthzAssignment();
  const directory = useUserDirectory();
  const teamsQuery = useTeams(enableTeamMode ? tenantId ?? null : null);

  const [subject, setSubject] = useState(defaultSubject ?? "");
  const [role, setRole] = useState("reader");

  useEffect(() => {
    if (defaultSubject) setSubject(defaultSubject);
  }, [defaultSubject]);

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
              {directory ? (
                <UserPicker
                  id="a-subj"
                  value={subject || null}
                  onChange={(sel) => setSubject(sel?.id ?? "")}
                  userDirectory={directory}
                  teams={(teamsQuery.data ?? []).map((t) => ({
                    id: t.id,
                    slug: t.slug,
                    displayName: t.display_name,
                  }))}
                  enableTeamMode={!!enableTeamMode}
                  enableGlobMode
                  placeholder={m.assignments.form.subjectPlaceholder}
                />
              ) : (
                <UserPickerFallback
                  id="a-subj"
                  value={subject}
                  onChange={setSubject}
                  placeholder={m.assignments.form.subjectPlaceholder}
                  required
                />
              )}
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
          rows={(list.data?.assignments ?? [])
            .filter((a) => !defaultSubject || a.subject === defaultSubject)
            .map((a) => (
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
