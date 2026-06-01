// Page-detail drawer.
//
// G2 shipped the read-only summary. G3 (this file) wires the
// mutations:
//   - share-scope radios fire `setShareScope`
//   - `+ Add team or person` opens the `<GrantsCombobox>` and
//     fires `createGrant` on submit
//   - the tier dropdown on each grant row fires `patchGrant`
//   - the `×` button on each row fires `deleteGrant`
//
// The drawer is intentionally optimistic-by-invalidation: every
// mutation calls `qc.invalidateQueries` for the grants + instances
// keys, and the engine.reload server-side keeps `check()` in sync.

import { useMemo, useState } from "react";
import {
  Badge,
  Button,
  Label,
  RadioGroup,
  RadioGroupItem,
  ScrollArea,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@nube/starter-ui-kit";
import { Lock, Plus, ShieldAlert, Trash2, UserRound, Users } from "lucide-react";
import type {
  EffectiveAcl,
  Grant,
  GrantSubject,
  GrantSummary,
  PermissionTier,
  ResourceInstance,
  ShareScope,
  SubjectRef,
} from "@nube/starter-client-ts";
import {
  useCreateGrant,
  useDeleteGrant,
  useGrants,
  usePatchGrant,
  useSetShareScope,
} from "../hooks/index.js";
import { GrantsCombobox } from "./grants-combobox.js";

const PAGE_KIND = "rubix.dashboard.page";
const TIER_OPTIONS: PermissionTier[] = ["View", "Edit", "Manage"];

export interface PageDetailDrawerProps {
  /** The page to inspect; `null` keeps the drawer closed. */
  page: ResourceInstance | null;
  /** Tenant id — drives the team/member picker and the grant scope. */
  tenantId: string;
  onClose: () => void;
}

export function PageDetailDrawer({
  page,
  tenantId,
  onClose,
}: PageDetailDrawerProps) {
  const open = page !== null;
  return (
    <Sheet open={open} onOpenChange={(o) => !o && onClose()}>
      <SheetContent
        side="right"
        className="flex w-full flex-col gap-0 p-0 sm:max-w-lg"
      >
        {page ? <Body page={page} tenantId={tenantId} /> : null}
      </SheetContent>
    </Sheet>
  );
}

function Body({
  page,
  tenantId,
}: {
  page: ResourceInstance;
  tenantId: string;
}) {
  const acl = page.effective_acl;
  return (
    <>
      <SheetHeader className="border-b border-border px-6 py-4">
        <SheetTitle className="text-lg">{page.label}</SheetTitle>
        <SheetDescription>
          Manage who can access this page.
        </SheetDescription>
      </SheetHeader>
      <ScrollArea className="flex-1">
        <div className="grid gap-6 px-6 py-5">
          <dl className="grid gap-3 text-sm">
            <div className="grid gap-1">
              <dt className="text-xs uppercase tracking-wide text-muted-foreground">
                Owner
              </dt>
              <dd>
                {page.owner ? (
                  subjectLabel(page.owner)
                ) : (
                  <em className="text-muted-foreground">Unowned</em>
                )}
              </dd>
            </div>
            <div className="grid gap-1">
              <dt className="text-xs uppercase tracking-wide text-muted-foreground">
                Page ID
              </dt>
              <dd>
                <code className="text-xs">{page.id}</code>
              </dd>
            </div>
            {page.updated_at ? (
              <div className="grid gap-1">
                <dt className="text-xs uppercase tracking-wide text-muted-foreground">
                  Updated
                </dt>
                <dd>{page.updated_at}</dd>
              </div>
            ) : null}
          </dl>

          <ShareScopeSection
            scope={acl.share_scope}
            pageId={page.id}
            tenantId={tenantId}
          />
          <GrantsSection
            pageId={page.id}
            tenantId={tenantId}
            fallback={acl}
          />
        </div>
      </ScrollArea>
    </>
  );
}

function ShareScopeSection({
  scope,
  pageId,
  tenantId,
}: {
  scope: ShareScope;
  pageId: string;
  tenantId: string;
}) {
  const setScope = useSetShareScope();
  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold">Share with</h3>
      <RadioGroup
        value={scope}
        onValueChange={(v) =>
          setScope.mutate({
            kind: PAGE_KIND,
            resourceId: pageId,
            body: { scope: v as ShareScope, tenant_id: tenantId },
          })
        }
        disabled={setScope.isPending}
        className="grid gap-2 rounded-2xl border border-border bg-muted/20 p-3"
      >
        <label className="flex items-center gap-2 text-sm">
          <RadioGroupItem value="private" id="ps-private" />
          <Label htmlFor="ps-private">Private</Label>
        </label>
        <label className="flex items-center gap-2 text-sm">
          <RadioGroupItem value="tenant" id="ps-tenant" />
          <Label htmlFor="ps-tenant">Anyone in this tenant</Label>
        </label>
        <label className="flex items-center gap-2 text-sm">
          <RadioGroupItem value="specific" id="ps-specific" />
          <Label htmlFor="ps-specific">Specific teams or people</Label>
        </label>
      </RadioGroup>
    </section>
  );
}

function GrantsSection({
  pageId,
  tenantId,
  fallback,
}: {
  pageId: string;
  tenantId: string;
  fallback: EffectiveAcl;
}) {
  const [adding, setAdding] = useState(false);
  const grants = useGrants({
    resource_kind: PAGE_KIND,
    resource_id: pageId,
    tenant_id: tenantId,
  });
  const create = useCreateGrant();

  // Until the live `listGrants` query returns, fall back to the
  // summary the parent already rendered so the UI isn't blank.
  const rows: Array<RowItem> = useMemo(() => {
    if (grants.data?.grants?.length) {
      return grants.data.grants.map((g) => ({ kind: "live" as const, g }));
    }
    return fallback.grants.map((s) => ({ kind: "summary" as const, s }));
  }, [grants.data, fallback.grants]);

  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold">Who can access</h3>
      {fallback.has_legacy_rules ? (
        <Badge
          variant="outline"
          className="w-fit border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300"
        >
          <ShieldAlert className="mr-1 size-3" aria-hidden /> Legacy rules
        </Badge>
      ) : null}
      <div className="grid gap-2 rounded-2xl border border-border bg-card p-3">
        {rows.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No direct grants — access is governed by the share scope above.
          </p>
        ) : (
          rows.map((r, i) =>
            r.kind === "live" ? (
              <LiveGrantRow key={r.g.id} grant={r.g} />
            ) : (
              <SummaryGrantRow key={`s${i}`} grant={r.s} />
            ),
          )
        )}
      </div>
      {adding ? (
        <GrantsCombobox
          tenantId={tenantId}
          disabled={create.isPending}
          onSubmit={(subject, tier) => {
            create.mutate(
              {
                subject,
                resource_kind: PAGE_KIND,
                resource_id: pageId,
                tier,
                tenant_id: tenantId,
              },
              {
                onSuccess: () => setAdding(false),
              },
            );
          }}
        />
      ) : (
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="w-fit gap-2"
          onClick={() => setAdding(true)}
        >
          <Plus className="size-4" aria-hidden />
          Add team or person
        </Button>
      )}
    </section>
  );
}

type RowItem =
  | { kind: "live"; g: Grant }
  | { kind: "summary"; s: GrantSummary };

function LiveGrantRow({ grant }: { grant: Grant }) {
  const patch = usePatchGrant();
  const remove = useDeleteGrant();
  return (
    <div className="flex items-center justify-between gap-3 text-sm">
      <span className="flex items-center gap-2">
        <SubjectIcon subject={subjectFromGrant(grant.subject)} />
        {subjectLabel(subjectFromGrant(grant.subject))}
      </span>
      <span className="flex items-center gap-2">
        <Select
          value={grant.tier}
          onValueChange={(v) =>
            patch.mutate({
              id: grant.id,
              body: { tier: v as PermissionTier },
            })
          }
          disabled={patch.isPending || remove.isPending}
        >
          <SelectTrigger className="h-7 w-24" aria-label="Tier">
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
          variant="ghost"
          size="icon"
          className="size-7"
          aria-label="Revoke"
          disabled={patch.isPending || remove.isPending}
          onClick={() => remove.mutate(grant.id)}
        >
          <Trash2 className="size-4" aria-hidden />
        </Button>
      </span>
    </div>
  );
}

function SummaryGrantRow({ grant }: { grant: GrantSummary }) {
  return (
    <div className="flex items-center justify-between gap-3 text-sm">
      <span className="flex items-center gap-2">
        <SubjectIcon subject={grant.subject} />
        {subjectLabel(grant.subject)}
      </span>
      <Badge variant="secondary">{grant.tier}</Badge>
    </div>
  );
}

function SubjectIcon({ subject }: { subject: SubjectRef }) {
  const Icon =
    subject.kind === "team"
      ? Users
      : subject.kind === "user"
        ? UserRound
        : Lock;
  return <Icon className="size-4 text-muted-foreground" aria-hidden />;
}

function subjectFromGrant(s: GrantSubject): SubjectRef {
  switch (s.kind) {
    case "team":
      return { kind: "team", slug: s.slug };
    case "user":
      return { kind: "user", sub: s.sub };
    case "wildcard":
      return { kind: "wildcard" };
  }
}

export function subjectLabel(s: SubjectRef): string {
  switch (s.kind) {
    case "team":
      return s.slug;
    case "user":
      return s.sub.length > 8 ? s.sub.slice(0, 8) : s.sub;
    case "wildcard":
      return "Everyone";
  }
}
