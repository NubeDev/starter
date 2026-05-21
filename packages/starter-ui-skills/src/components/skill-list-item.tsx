import * as React from "react";
import { cn } from "../lib/utils.js";
import type { SkillSummary } from "../types/index.js";
import { SkillTrustBadge } from "./skill-trust-badge.js";
import { SkillActionButton } from "./skill-action-button.js";

export interface SkillListItemProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "onSelect"> {
  skill: SkillSummary;
  selected?: boolean;
  busy?: boolean;
  onSelect?: (skill: SkillSummary) => void;
  onApprove?: (skill: SkillSummary) => void;
  onRevoke?: (skill: SkillSummary) => void;
  onInspect?: (skill: SkillSummary) => void;
}

export const SkillListItem = React.forwardRef<
  HTMLDivElement,
  SkillListItemProps
>(function SkillListItem(
  {
    skill,
    selected,
    busy,
    onSelect,
    onApprove,
    onRevoke,
    onInspect,
    className,
    ...props
  },
  ref,
) {
  const approved = skill.trust === "approved";
  const handleInspect = () => (onInspect ?? onSelect)?.(skill);
  return (
    <div
      ref={ref}
      data-slot="skill-card"
      data-trust={skill.trust}
      data-selected={selected ? "" : undefined}
      className={cn(
        "group relative flex flex-col gap-1.5 rounded-lg border bg-card/60 px-3 py-2.5 transition-colors",
        selected
          ? "border-foreground/30 ring-1 ring-foreground/10"
          : "border-border/60 hover:border-border",
        className,
      )}
      {...props}
    >
      <div className="flex items-start gap-2">
        <div className="flex size-7 shrink-0 items-center justify-center rounded-md bg-muted/40 text-muted-foreground">
          <svg
            viewBox="0 0 24 24"
            width="14"
            height="14"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden
          >
            <path d="M12 3l1.9 4.6L18.5 9 14.7 12l1 4.9L12 14.6 8.3 16.9l1-4.9L5.5 9l4.6-1.4L12 3z" />
          </svg>
        </div>
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="flex items-center gap-1.5 text-[12.5px] font-medium">
            <span className="truncate font-mono">{skill.id}</span>
            <SkillTrustBadge trust={skill.trust} />
            {skill.source === "extension" ? (
              <span className="rounded bg-muted/50 px-1 py-0.5 text-[9px] uppercase tracking-wide text-muted-foreground">
                Ext
              </span>
            ) : null}
          </span>
          <span className="line-clamp-2 text-[10.5px] leading-relaxed text-muted-foreground">
            {skill.description}
          </span>
        </div>
      </div>

      <div className="mt-0.5 flex items-center justify-between gap-1">
        {approved ? (
          <SkillActionButton
            variant="outline"
            size="xs"
            loading={busy}
            onClick={() => onRevoke?.(skill)}
          >
            Revoke
          </SkillActionButton>
        ) : (
          <SkillActionButton
            variant="default"
            size="xs"
            loading={busy}
            onClick={() => onApprove?.(skill)}
          >
            Approve
          </SkillActionButton>
        )}
        <div className="flex gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
          <button
            type="button"
            onClick={handleInspect}
            title="Inspect"
            aria-label="Inspect skill"
            className="inline-flex size-6 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            <svg
              viewBox="0 0 24 24"
              width="11"
              height="11"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.75"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden
            >
              <circle cx="11" cy="11" r="7" />
              <path d="M21 21l-4.3-4.3" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
});
SkillListItem.displayName = "SkillListItem";
