// G3 — subject picker for the page-detail drawer.
//
// Lists this tenant's teams first, then members. Submitting emits a
// `GrantSubject` (team or user) plus a `PermissionTier`. The drawer
// turns that into a `createGrant` request.

import { useMemo, useState } from "react";
import {
  Button,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit";
import type {
  GrantSubject,
  PermissionTier,
} from "@nube/starter-client-ts";
import { useTeams, useTenantMembers } from "../hooks/index.js";

export interface GrantsComboboxProps {
  /** Tenant whose teams + members are listed. */
  tenantId: string;
  /** Fired on submit; the drawer wraps this in a `createGrant`. */
  onSubmit: (subject: GrantSubject, tier: PermissionTier) => void;
  /** Disable while a mutation is in flight. */
  disabled?: boolean;
}

const TIER_OPTIONS: PermissionTier[] = ["View", "Edit", "Manage"];

export function GrantsCombobox({
  tenantId,
  onSubmit,
  disabled,
}: GrantsComboboxProps) {
  const teams = useTeams(tenantId);
  const members = useTenantMembers(tenantId);
  const [subjectKey, setSubjectKey] = useState<string>("");
  const [tier, setTier] = useState<PermissionTier>("View");

  // subjectKey encodes `team:<slug>` or `user:<id>` so the Select
  // value stays a single string.
  const subjects = useMemo(() => {
    const items: Array<{ key: string; label: string; subject: GrantSubject }> =
      [];
    for (const t of teams.data ?? []) {
      items.push({
        key: `team:${t.slug}`,
        label: `Team — ${t.display_name || t.slug}`,
        subject: { kind: "team", slug: t.slug },
      });
    }
    for (const m of members.data ?? []) {
      items.push({
        key: `user:${m.user_id}`,
        label: `User — ${m.user_id}`,
        subject: { kind: "user", sub: m.user_id },
      });
    }
    return items;
  }, [teams.data, members.data]);

  const subject = subjects.find((s) => s.key === subjectKey)?.subject;
  const canSubmit = !!subject && !disabled;

  return (
    <div className="grid gap-2 rounded-2xl border border-border bg-muted/20 p-3">
      <div className="grid gap-2 sm:grid-cols-[2fr_1fr_auto]">
        <Select value={subjectKey} onValueChange={setSubjectKey}>
          <SelectTrigger aria-label="Team or person">
            <SelectValue placeholder="Select a team or person" />
          </SelectTrigger>
          <SelectContent>
            {subjects.length === 0 ? (
              <SelectItem value="__empty" disabled>
                No teams or members
              </SelectItem>
            ) : (
              subjects.map((s) => (
                <SelectItem key={s.key} value={s.key}>
                  {s.label}
                </SelectItem>
              ))
            )}
          </SelectContent>
        </Select>
        <Select
          value={tier}
          onValueChange={(v) => setTier(v as PermissionTier)}
        >
          <SelectTrigger aria-label="Tier">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {TIER_OPTIONS.map((t) => (
              <SelectItem key={t} value={t}>
                {t}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button
          type="button"
          size="sm"
          disabled={!canSubmit}
          onClick={() => {
            if (subject) onSubmit(subject, tier);
          }}
        >
          Add
        </Button>
      </div>
    </div>
  );
}
