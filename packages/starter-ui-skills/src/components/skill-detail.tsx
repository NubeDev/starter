import * as React from "react";
import { cn, formatBytes, formatRelative } from "../lib/utils.js";
import type { Skill } from "../types/index.js";
import { SkillTrustBadge } from "./skill-trust-badge.js";
import { SkillHash } from "./skill-hash.js";

export interface SkillDetailProps extends React.HTMLAttributes<HTMLDivElement> {
  skill: Skill;
  /** Operator actions, rendered top-right (e.g. Approve / Revoke buttons). */
  actions?: React.ReactNode;
  /** Render the SKILL.md body. Default: pre-wrap text. Plug a markdown
   *  renderer here if your app has one. */
  renderBody?: (body: string) => React.ReactNode;
}

export const SkillDetail = React.forwardRef<HTMLDivElement, SkillDetailProps>(
  ({ skill, actions, renderBody, className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        data-slot="skill-detail"
        data-trust={skill.trust}
        className={cn("flex h-full min-h-0 flex-col", className)}
        {...props}
      >
        <header className="flex flex-col gap-3 border-b border-border/40 p-4">
          <div className="flex items-start gap-3">
            <div className="flex min-w-0 flex-1 flex-col gap-1">
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="truncate font-mono text-base font-semibold">
                  {skill.id}
                </h2>
                <SkillTrustBadge trust={skill.trust} />
                {skill.source === "extension" ? (
                  <span className="rounded-full border border-border/40 bg-muted/40 px-2 py-0.5 text-[10px] uppercase tracking-wide text-muted-foreground">
                    extension
                  </span>
                ) : null}
              </div>
              <p className="text-sm text-muted-foreground">{skill.description}</p>
            </div>
            {actions ? (
              <div className="flex shrink-0 items-center gap-2">{actions}</div>
            ) : null}
          </div>
          <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-xs sm:grid-cols-4">
            <Meta label="Bundle hash">
              <SkillHash hash={skill.bundleHash} />
            </Meta>
            {skill.modelHint ? (
              <Meta label="Model hint">
                <span className="font-mono">{skill.modelHint}</span>
              </Meta>
            ) : null}
            {skill.approvedAt ? (
              <Meta label="Approved">
                <span title={skill.approvedAt}>
                  {formatRelative(skill.approvedAt)}
                </span>
                {skill.approvedBy ? (
                  <span className="text-muted-foreground"> by {skill.approvedBy}</span>
                ) : null}
              </Meta>
            ) : null}
            {skill.quarantineReason ? (
              <Meta label="Quarantined">
                <span>{humanReason(skill.quarantineReason)}</span>
              </Meta>
            ) : null}
          </dl>
          {skill.allowedTools.length > 0 && (
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="text-xs text-muted-foreground">Tools:</span>
              {skill.allowedTools.map((t) => (
                <span
                  key={t}
                  className="rounded-md border border-border/40 bg-muted/40 px-1.5 py-0.5 font-mono text-[10px]"
                >
                  {t}
                </span>
              ))}
            </div>
          )}
        </header>

        <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-4">
          <section data-slot="skill-detail-body">
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              SKILL.md
            </h3>
            <div className="rounded-lg border border-border/40 bg-muted/30 p-3 text-sm leading-relaxed">
              {renderBody ? (
                renderBody(skill.body)
              ) : (
                <pre className="whitespace-pre-wrap break-words font-sans">
                  {skill.body}
                </pre>
              )}
            </div>
          </section>

          {skill.resources.length > 0 && (
            <section data-slot="skill-detail-resources">
              <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Resources ({skill.resources.length})
              </h3>
              <ul className="flex flex-col gap-1">
                {skill.resources.map((r) => (
                  <li
                    key={r.uri}
                    className="flex items-center gap-2 rounded-md border border-border/40 bg-background/60 px-2.5 py-1.5 text-xs"
                  >
                    <span className="flex h-7 w-7 items-center justify-center rounded-md bg-muted text-[10px] font-semibold uppercase text-muted-foreground">
                      {extOf(r.name ?? r.uri)}
                    </span>
                    <div className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate font-mono">{r.name ?? r.uri}</span>
                      {r.sizeBytes ? (
                        <span className="text-[10px] text-muted-foreground">
                          {formatBytes(r.sizeBytes)}
                        </span>
                      ) : null}
                    </div>
                    <SkillHash hash={r.contentHash} />
                  </li>
                ))}
              </ul>
            </section>
          )}
        </div>
      </div>
    );
  },
);
SkillDetail.displayName = "SkillDetail";

function Meta({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-0.5">
      <dt className="text-[10px] uppercase tracking-wide text-muted-foreground">
        {label}
      </dt>
      <dd className="text-xs">{children}</dd>
    </div>
  );
}

function extOf(s: string): string {
  const i = s.lastIndexOf(".");
  if (i < 0) return "file";
  return s.slice(i + 1).slice(0, 4);
}

function humanReason(reason: string): string {
  switch (reason) {
    case "extension-contributed":
      return "Contributed by an extension";
    case "frontmatter-opt-in":
      return "Frontmatter requested hold";
    case "no-approval-row":
      return "No approval on record";
    case "hash-mismatch":
      return "Bundle changed since approval";
    default:
      return reason;
  }
}
