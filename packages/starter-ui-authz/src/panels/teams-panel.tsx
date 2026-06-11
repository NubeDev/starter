// Teams panel — per-tenant teams (R13). Lists, creates, deletes.
//
// Create-team lives behind a "+" button that opens a dialog (rather than a
// permanent inline form). Each row's members are managed in a per-team dialog
// that lists current members (with remove) and offers an add-by-picker — this
// is the team-member CRUD surface (add, list, remove), backed by the
// `GET /v1/tenants/{id}/teams/{team_id}/members` listing endpoint.

import { useState, type FormEvent } from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  Input,
  Label,
} from "@nube/starter-ui-kit";
import { Plus, Trash2, Users } from "lucide-react";
import { useAuthzMessages } from "../i18n/context.js";
import { mergeAuthzMessages, type AuthzMessages } from "../i18n/messages.js";
import type { TeamView } from "@nube/starter-client-ts";
import {
  useAddTeamMember,
  useCreateTeam,
  useDeleteTeam,
  useRemoveTeamMember,
  useTeamMembers,
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

  const list = useTeams(tenantId);

  if (!tenantId) {
    return (
      <section className="grid gap-6">
        <Header title={m.teams.title} description={m.teams.description} />
        <StateRow variant="empty">{m.teams.selectTenantPrompt}</StateRow>
      </section>
    );
  }

  return (
    <section className="grid gap-6">
      <div className="flex items-start justify-between gap-4">
        <Header title={m.teams.title} description={m.teams.description} />
        <CreateTeamDialog tenantId={tenantId} m={m} />
      </div>

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
            <TeamRow key={t.id} tenantId={tenantId} team={t} m={m} />
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

// ---------------------------------------------------------------- create team

function CreateTeamDialog({ tenantId, m }: { tenantId: string; m: AuthzMessages }) {
  const create = useCreateTeam();
  const [open, setOpen] = useState(false);
  const [slug, setSlug] = useState("");
  const [displayName, setDisplayName] = useState("");

  async function onCreate(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!slug.trim() || !displayName.trim()) return;
    await create.mutateAsync({
      tenantId,
      body: { slug: slug.trim(), display_name: displayName.trim() },
    });
    setSlug("");
    setDisplayName("");
    setOpen(false);
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button size="sm">
          <Plus className="size-4" />
          {m.teams.form.submit}
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{m.teams.form.submit}</DialogTitle>
          <DialogDescription>{m.teams.description}</DialogDescription>
        </DialogHeader>
        <form onSubmit={onCreate} className="grid gap-4">
          <div className="grid gap-1">
            <Label htmlFor="tm-slug">{m.teams.form.slugLabel}</Label>
            <Input id="tm-slug" value={slug} onChange={(e) => setSlug(e.currentTarget.value)} required />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="tm-name">{m.teams.form.displayNameLabel}</Label>
            <Input id="tm-name" value={displayName} onChange={(e) => setDisplayName(e.currentTarget.value)} required />
          </div>
          {create.error ? (
            <p className="text-xs text-[color:var(--color-danger,#dc2626)]">{create.error.message}</p>
          ) : null}
          <DialogFooter>
            <Button type="submit" disabled={create.isPending}>
              {create.isPending ? `${m.teams.form.submit}…` : m.teams.form.submit}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

// ------------------------------------------------------------------ team rows

function TeamRow({ tenantId, team, m }: { tenantId: string; team: TeamView; m: AuthzMessages }) {
  const del = useDeleteTeam();
  return (
    <tr>
      <Td><code className="text-xs">{team.slug}</code></Td>
      <Td>{team.display_name}</Td>
      <Td>
        <ManageMembersDialog tenantId={tenantId} team={team} m={m} />
      </Td>
      <ActionsCell>
        <Button
          size="sm"
          variant="outline"
          onClick={() => {
            if (!window.confirm(m.common.confirmDelete)) return;
            void del.mutateAsync({ tenantId, teamId: team.id });
          }}
          disabled={del.isPending}
        >
          {m.common.delete}
        </Button>
      </ActionsCell>
    </tr>
  );
}

// ------------------------------------------------------------- manage members

function ManageMembersDialog({
  tenantId,
  team,
  m,
}: {
  tenantId: string;
  team: TeamView;
  m: AuthzMessages;
}) {
  const [open, setOpen] = useState(false);
  const directory = useUserDirectory();
  // Only fetch members while the dialog is open — avoids one member-list
  // request per team row when the page first loads.
  const members = useTeamMembers(tenantId, open ? team.id : null);
  const add = useAddTeamMember();
  const remove = useRemoveTeamMember();
  const [draft, setDraft] = useState<string>("");

  async function onAdd() {
    const userId = draft.trim();
    if (!userId) return;
    await add.mutateAsync({ tenantId, teamId: team.id, body: { user_id: userId } });
    setDraft("");
  }

  const rows = members.data ?? [];

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button size="sm" variant="outline">
          <Users className="size-4" />
          {m.teams.teamMembers.manage}
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{team.display_name}</DialogTitle>
          <DialogDescription>{m.teams.teamMembers.title}</DialogDescription>
        </DialogHeader>

        {/* Add a member */}
        <div className="flex items-end gap-2">
          <div className="grid flex-1 gap-1">
            <Label>{m.teams.teamMembers.userIdLabel}</Label>
            {directory ? (
              <UserPicker
                value={draft || null}
                onChange={(sel) => setDraft(sel?.kind === "user" ? sel.id : "")}
                userDirectory={directory}
                enableGlobMode={false}
                placeholder={m.teams.teamMembers.userIdLabel}
              />
            ) : (
              <UserPickerFallback
                value={draft}
                onChange={setDraft}
                placeholder={m.teams.teamMembers.userIdLabel}
              />
            )}
          </div>
          <Button onClick={() => void onAdd()} disabled={!draft.trim() || add.isPending}>
            <Plus className="size-4" />
            {m.teams.teamMembers.add}
          </Button>
        </div>
        {add.error ? (
          <p className="text-xs text-[color:var(--color-danger,#dc2626)]">{add.error.message}</p>
        ) : null}

        {/* Current members */}
        <div className="mt-2 max-h-72 overflow-y-auto rounded-md border">
          {members.isLoading ? (
            <p className="p-3 text-sm text-muted-foreground">{m.common.loading}</p>
          ) : members.error ? (
            <p className="p-3 text-sm text-[color:var(--color-danger,#dc2626)]">
              {members.error.message || m.common.error}
            </p>
          ) : rows.length === 0 ? (
            <p className="p-3 text-sm text-muted-foreground">{m.teams.teamMembers.empty}</p>
          ) : (
            <ul className="divide-y">
              {rows.map((r) => (
                <li key={r.user_id} className="flex items-center justify-between gap-2 p-2">
                  <span className="min-w-0 truncate text-sm">
                    {r.email ?? <code className="text-xs">{r.user_id}</code>}
                  </span>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() =>
                      void remove.mutateAsync({ tenantId, teamId: team.id, userId: r.user_id })
                    }
                    disabled={remove.isPending}
                    aria-label={m.teams.teamMembers.remove}
                  >
                    <Trash2 className="size-4" />
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
