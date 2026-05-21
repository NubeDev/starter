import * as React from "react";
import { cn, formatBytes, formatRelative } from "../lib/utils.js";
import type { Skill } from "../types/index.js";
import { SkillTrustBadge } from "./skill-trust-badge.js";
import { SkillHash } from "./skill-hash.js";

export interface SkillDetailProps extends React.HTMLAttributes<HTMLDivElement> {
  skill: Skill;
  actions?: React.ReactNode;
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
        <header className="flex flex-col gap-4 border-b px-6 py-5 pr-12">
          <div className="flex items-start gap-4">
            <div className="flex min-w-0 flex-1 flex-col gap-1.5">
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="truncate font-mono text-base font-semibold leading-tight">
                  {skill.id}
                </h2>
                <SkillTrustBadge trust={skill.trust} />
                {skill.source === "extension" ? (
                  <span className="inline-flex rounded-full border px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
                    Extension
                  </span>
                ) : null}
              </div>
              <p className="text-sm leading-relaxed text-muted-foreground">
                {skill.description}
              </p>
            </div>
            {actions ? (
              <div className="flex shrink-0 items-center gap-2">{actions}</div>
            ) : null}
          </div>
          <dl className="grid grid-cols-2 gap-x-6 gap-y-3 text-sm sm:grid-cols-4">
            <Meta label="Bundle hash">
              <SkillHash hash={skill.bundleHash} />
            </Meta>
            {skill.modelHint ? (
              <Meta label="Model hint">
                <span className="font-mono text-xs">{skill.modelHint}</span>
              </Meta>
            ) : null}
            {skill.approvedAt ? (
              <Meta label="Approved">
                <span className="text-xs" title={skill.approvedAt}>
                  {formatRelative(skill.approvedAt)}
                </span>
                {skill.approvedBy ? (
                  <span className="text-xs text-muted-foreground">
                    {" "}
                    by {skill.approvedBy}
                  </span>
                ) : null}
              </Meta>
            ) : null}
            {skill.quarantineReason ? (
              <Meta label="Quarantined">
                <span className="text-xs">
                  {humanReason(skill.quarantineReason)}
                </span>
              </Meta>
            ) : null}
          </dl>
          {skill.allowedTools.length > 0 && (
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="text-xs text-muted-foreground">Tools:</span>
              {skill.allowedTools.map((t) => (
                <span
                  key={t}
                  className="rounded-md bg-muted px-2 py-0.5 font-mono text-[11px] text-muted-foreground"
                >
                  {t}
                </span>
              ))}
            </div>
          )}
        </header>

        <div className="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto px-6 py-5">
          <section data-slot="skill-detail-body" className="flex flex-col gap-2">
            <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              SKILL.md
            </h3>
            <div className="rounded-lg border bg-muted/30 p-4 text-sm leading-relaxed">
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
            <section data-slot="skill-detail-resources" className="flex flex-col gap-2">
              <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Resources ({skill.resources.length})
              </h3>
              <ul className="flex flex-col gap-1.5">
                {skill.resources.map((r) => (
                  <li
                    key={r.uri}
                    className="flex items-center gap-3 rounded-md border bg-card/60 px-3 py-2 text-sm"
                  >
                    <span className="flex size-8 shrink-0 items-center justify-center rounded-md border bg-muted/50 text-[10px] font-semibold uppercase text-muted-foreground">
                      {extOf(r.name ?? r.uri)}
                    </span>
                    <div className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate font-mono text-xs">
                        {r.name ?? r.uri}
                      </span>
                      {r.sizeBytes ? (
                        <span className="text-[11px] text-muted-foreground">
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
    <div className="flex flex-col gap-1">
      <dt className="text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
        {label}
      </dt>
      <dd>{children}</dd>
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
