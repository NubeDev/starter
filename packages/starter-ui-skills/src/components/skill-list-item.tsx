import * as React from "react";
import { cn, formatRelative } from "../lib/utils.js";
import type { SkillSummary } from "../types/index.js";
import { SkillTrustBadge } from "./skill-trust-badge.js";
import { SkillHash } from "./skill-hash.js";

export interface SkillListItemProps
  extends Omit<React.HTMLAttributes<HTMLButtonElement>, "onSelect"> {
  skill: SkillSummary;
  selected?: boolean;
  onSelect?: (skill: SkillSummary) => void;
}

export const SkillListItem = React.forwardRef<
  HTMLButtonElement,
  SkillListItemProps
>(({ skill, selected, onSelect, className, ...props }, ref) => {
  return (
    <button
      ref={ref}
      type="button"
      data-slot="skill-list-item"
      data-selected={selected ? "" : undefined}
      data-trust={skill.trust}
      aria-current={selected ? "true" : undefined}
      onClick={() => onSelect?.(skill)}
      className={cn(
        "group flex w-full flex-col gap-2 rounded-lg border border-border/40 bg-background/60 p-3 text-left transition hover:border-primary/40 hover:bg-background hover:shadow-sm",
        selected && "border-primary/60 bg-background shadow-sm",
        className,
      )}
      {...props}
    >
      <div className="flex items-start gap-2">
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <div className="truncate font-mono text-sm font-medium">
            {skill.id}
          </div>
          <div className="line-clamp-2 text-xs text-muted-foreground">
            {skill.description}
          </div>
        </div>
        <SkillTrustBadge trust={skill.trust} />
      </div>
      <div className="flex flex-wrap items-center gap-1.5 text-[10px] text-muted-foreground">
        <SkillHash hash={skill.bundleHash} label="bundle" />
        {skill.source === "extension" ? (
          <span className="rounded-md border border-border/40 px-1.5 py-0.5">
            ext
          </span>
        ) : null}
        {skill.modelHint ? (
          <span className="rounded-md border border-border/40 px-1.5 py-0.5 font-mono">
            {skill.modelHint}
          </span>
        ) : null}
        {skill.approvedAt ? (
          <span className="ml-auto">approved {formatRelative(skill.approvedAt)}</span>
        ) : null}
      </div>
    </button>
  );
});
SkillListItem.displayName = "SkillListItem";
