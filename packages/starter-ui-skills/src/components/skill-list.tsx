import * as React from "react";
import { cn } from "../lib/utils.js";
import type { SkillSummary } from "../types/index.js";
import { SkillListItem } from "./skill-list-item.js";

export interface SkillListProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "onSelect"> {
  skills: SkillSummary[];
  selectedId?: string | null;
  busyId?: string | null;
  onSelect?: (skill: SkillSummary) => void;
  onApprove?: (skill: SkillSummary) => void;
  onRevoke?: (skill: SkillSummary) => void;
  onInspect?: (skill: SkillSummary) => void;
  emptyMessage?: React.ReactNode;
}

export const SkillList = React.forwardRef<HTMLDivElement, SkillListProps>(
  function SkillList(
    {
      skills,
      selectedId,
      busyId,
      onSelect,
      onApprove,
      onRevoke,
      onInspect,
      emptyMessage,
      className,
      ...props
    },
    ref,
  ) {
    if (!skills.length) {
      return (
        <div
          ref={ref}
          className={cn(
            "rounded-lg border border-dashed border-border/60 bg-card/30 px-4 py-6 text-center text-[11px] text-muted-foreground",
            className,
          )}
          {...props}
        >
          {emptyMessage ?? "No skills to show."}
        </div>
      );
    }
    return (
      <div
        ref={ref}
        data-slot="skill-list"
        className={cn(
          "grid grid-cols-1 gap-2 sm:grid-cols-2",
          className,
        )}
        {...props}
      >
        {skills.map((s) => (
          <SkillListItem
            key={s.id}
            skill={s}
            selected={s.id === selectedId}
            busy={busyId === s.id}
            onSelect={onSelect}
            onApprove={onApprove}
            onRevoke={onRevoke}
            onInspect={onInspect}
          />
        ))}
      </div>
    );
  },
);
SkillList.displayName = "SkillList";
