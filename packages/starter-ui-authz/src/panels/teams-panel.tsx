// Teams panel — per-tenant teams (R13). Lists, creates, deletes;
// each row exposes a tiny inline form to add a team member by id.
// Team-member listing has no REST endpoint, mirroring the
// MembersPanel design (controlled-only display).

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
  useAddTeamMember,
  useCreateTeam,
  useDeleteTeam,
  useTeams,
} from "../hooks/index.js";
import { ActionsCell, DataTable, StateRow, Td } from "./_common.js";
import { UserPicker, UserPickerFallback, useUserDirectory } from "./user-picker.js";

export interface TeamsPanelProps {
  tenantId: string | null;
  i18n?: Partial<AuthzMessages>;
}

export function TeamsPanel({ tenantId, i18n }: TeamsPanelProps) {
  const ctx = useAuthzMessages();
  const m = i18n ? mergeAuthzMessages({ ...ctx, ...i18n }) : ctx;

  const directory = useUserDirectory();
  const list = useTeams(tenantId);
  const create = useCreateTeam();
  const del = useDeleteTeam();
  const addMember = useAddTeamMember();

  const [slug, setSlug] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [memberDrafts, setMemberDrafts] = useState<Record<string, string>>({});

  async function onCreate(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!tenantId || !slug.trim() || !displayName.trim()) return;
    await create.mutateAsync({
      tenantId,
      body: { slug: slug.trim(), display_name: displayName.trim() },
    });
    setSlug("");
    setDisplayName("");
  }

  async function onAddMember(teamId: string) {
    const userId = (memberDrafts[teamId] ?? "").trim();
    if (!tenantId || !userId) return;
    await addMember.mutateAsync({ tenantId, teamId, body: { user_id: userId } });
    setMemberDrafts((d) => ({ ...d, [teamId]: "" }));
  }

  if (!tenantId) {
    return (
      <section className="grid gap-6">
        <header>
          <h2 className="text-xl font-semibold tracking-tight">{m.teams.title}</h2>
          <p className="text-sm text-[color:var(--color-subtle,#6b7280)]">{m.teams.description}</p>
        </header>
        <StateRow variant="empty">{m.teams.selectTenantPrompt}</StateRow>
      </section>
    );
  }

  return (
    <section className="grid gap-6">
      <header>
        <h2 className="text-xl font-semibold tracking-tight">{m.teams.title}</h2>
        <p className="text-sm text-[color:var(--color-subtle,#6b7280)]">{m.teams.description}</p>
      </header>

      <Card>
        <CardHeader>
          <CardTitle>{m.teams.form.submit}</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={onCreate} className="grid grid-cols-1 gap-3 sm:grid-cols-[1fr_1fr_auto] sm:items-end">
            <div className="grid gap-1">
              <Label htmlFor="tm-slug">{m.teams.form.slugLabel}</Label>
              <Input id="tm-slug" value={slug} onChange={(e) => setSlug(e.currentTarget.value)} required />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="tm-name">{m.teams.form.displayNameLabel}</Label>
              <Input id="tm-name" value={displayName} onChange={(e) => setDisplayName(e.currentTarget.value)} required />
            </div>
            <Button type="submit" disabled={create.isPending}>{m.teams.form.submit}</Button>
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
          label={m.teams.title}
          headers={[m.teams.columns.slug, m.teams.columns.displayName, m.teams.teamMembers.title, ""]}
          rows={(list.data ?? []).map((t) => (
            <tr key={t.id}>
              <Td><code className="text-xs">{t.slug}</code></Td>
              <Td>{t.display_name}</Td>
              <Td>
                <div className="flex items-center gap-2">
                  <div className="w-56">
                    {directory ? (
                      <UserPicker
                        value={memberDrafts[t.id] || null}
                        onChange={(sel) =>
                          setMemberDrafts((d) => ({
                            ...d,
                            [t.id]: sel?.kind === "user" ? sel.id : "",
                          }))
                        }
                        userDirectory={directory}
                        enableGlobMode={false}
                        placeholder={m.teams.teamMembers.userIdLabel}
                      />
                    ) : (
                      <UserPickerFallback
                        value={memberDrafts[t.id] ?? ""}
                        onChange={(v) =>
                          setMemberDrafts((d) => ({ ...d, [t.id]: v }))
                        }
                        placeholder={m.teams.teamMembers.userIdLabel}
                      />
                    )}
                  </div>
                  <Button
                    size="sm"
                    onClick={() => void onAddMember(t.id)}
                    disabled={addMember.isPending}
                  >
                    {m.teams.teamMembers.add}
                  </Button>
                </div>
              </Td>
              <ActionsCell>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    if (!window.confirm(m.common.confirmDelete)) return;
                    void del.mutateAsync({ tenantId, teamId: t.id });
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
