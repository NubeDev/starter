// Read-only detail drawer for a dashboard page (G2). Editing the
// share scope, adding teams/people, and removing grants all land in
// G3 — for now the controls render disabled with a tooltip pointing
// at the next stage.

import {
  Badge,
  Button,
  Label,
  RadioGroup,
  RadioGroupItem,
  ScrollArea,
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@nube/starter-ui-kit";
import { Lock, Plus, ShieldAlert, UserRound, Users } from "lucide-react";
import type {
  EffectiveAcl,
  GrantSummary,
  ResourceInstance,
  ShareScope,
  SubjectRef,
} from "@nube/starter-client-ts";

const NEXT_STAGE_HINT = "Editing access lands in the next stage";

export interface PageDetailDrawerProps {
  /** The page to inspect; `null` keeps the drawer closed. */
  page: ResourceInstance | null;
  onClose: () => void;
}

export function PageDetailDrawer({ page, onClose }: PageDetailDrawerProps) {
  const open = page !== null;
  return (
    <TooltipProvider>
      <Sheet open={open} onOpenChange={(o) => !o && onClose()}>
        <SheetContent
          side="right"
          className="flex w-full flex-col gap-0 p-0 sm:max-w-lg"
        >
          {page ? <Body page={page} /> : null}
        </SheetContent>
      </Sheet>
    </TooltipProvider>
  );
}

function Body({ page }: { page: ResourceInstance }) {
  const acl = page.effective_acl;
  return (
    <>
      <SheetHeader className="border-b border-border px-6 py-4">
        <SheetTitle className="text-lg">{page.label}</SheetTitle>
        <SheetDescription>
          Read-only preview of who can access this page.
        </SheetDescription>
      </SheetHeader>
      <ScrollArea className="flex-1">
        <div className="grid gap-6 px-6 py-5">
          <dl className="grid gap-3 text-sm">
            <div className="grid gap-1">
              <dt className="text-xs uppercase tracking-wide text-muted-foreground">
                Owner
              </dt>
              <dd>{page.owner ? subjectLabel(page.owner) : <em className="text-muted-foreground">Unowned</em>}</dd>
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

          <ShareScopeSection scope={acl.share_scope} />
          <GrantsSection acl={acl} />
        </div>
      </ScrollArea>
    </>
  );
}

function ShareScopeSection({ scope }: { scope: ShareScope }) {
  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold">Share with</h3>
      <Tooltip>
        <TooltipTrigger asChild>
          <div>
            <RadioGroup
              value={scope}
              disabled
              className="grid gap-2 rounded-2xl border border-border bg-muted/20 p-3"
            >
              <label className="flex items-center gap-2 text-sm">
                <RadioGroupItem value="private" id="ps-private" disabled />
                <Label htmlFor="ps-private" className="cursor-not-allowed">
                  Private
                </Label>
              </label>
              <label className="flex items-center gap-2 text-sm">
                <RadioGroupItem value="tenant" id="ps-tenant" disabled />
                <Label htmlFor="ps-tenant" className="cursor-not-allowed">
                  Anyone in this tenant
                </Label>
              </label>
              <label className="flex items-center gap-2 text-sm">
                <RadioGroupItem value="specific" id="ps-specific" disabled />
                <Label htmlFor="ps-specific" className="cursor-not-allowed">
                  Specific teams or people
                </Label>
              </label>
            </RadioGroup>
          </div>
        </TooltipTrigger>
        <TooltipContent>{NEXT_STAGE_HINT}</TooltipContent>
      </Tooltip>
    </section>
  );
}

function GrantsSection({ acl }: { acl: EffectiveAcl }) {
  return (
    <section className="grid gap-2">
      <h3 className="text-sm font-semibold">Who can access</h3>
      {acl.has_legacy_rules ? (
        <Badge
          variant="outline"
          className="w-fit border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300"
        >
          <ShieldAlert className="mr-1 size-3" aria-hidden /> Legacy rules
        </Badge>
      ) : null}
      <div className="grid gap-2 rounded-2xl border border-border bg-card p-3">
        {acl.grants.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No direct grants — access is governed by the share scope above.
          </p>
        ) : (
          acl.grants.map((g, i) => <GrantRow key={i} grant={g} />)
        )}
      </div>
      <Tooltip>
        <TooltipTrigger asChild>
          <span className="w-fit">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled
              className="gap-2"
            >
              <Plus className="size-4" aria-hidden />
              Add team or person
            </Button>
          </span>
        </TooltipTrigger>
        <TooltipContent>{NEXT_STAGE_HINT}</TooltipContent>
      </Tooltip>
    </section>
  );
}

function GrantRow({ grant }: { grant: GrantSummary }) {
  const Icon =
    grant.subject.kind === "team"
      ? Users
      : grant.subject.kind === "user"
        ? UserRound
        : Lock;
  return (
    <div className="flex items-center justify-between gap-3 text-sm">
      <span className="flex items-center gap-2">
        <Icon className="size-4 text-muted-foreground" aria-hidden />
        {subjectLabel(grant.subject)}
      </span>
      <Badge variant="secondary">{grant.tier}</Badge>
    </div>
  );
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
