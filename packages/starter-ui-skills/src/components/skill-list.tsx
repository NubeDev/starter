import * as React from "react";
import { cn } from "../lib/utils.js";
import type { SkillSummary } from "../types/index.js";
import { SkillListItem } from "./skill-list-item.js";

export interface SkillListProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "onSelect"> {
  skills: SkillSummary[];
  selectedId?: string | null;
  onSelect?: (skill: SkillSummary) => void;
  emptyMessage?: React.ReactNode;
}

export const SkillList = React.forwardRef<HTMLDivElement, SkillListProps>(
  ({ skills, selectedId, onSelect, emptyMessage, className, ...props }, ref) => {
    if (!skills.length) {
      return (
        <div
          ref={ref}
          data-slot="skill-list-empty"
          className={cn(
            "flex h-full min-h-[8rem] items-center justify-center rounded-lg border border-dashed border-border/60 p-4 text-sm text-muted-foreground",
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
        className={cn("flex flex-col gap-2", className)}
        role="listbox"
        {...props}
      >
        {skills.map((s) => (
          <SkillListItem
            key={s.id}
            skill={s}
            selected={s.id === selectedId}
            onSelect={onSelect}
          />
        ))}
      </div>
    );
  },
);
SkillList.displayName = "SkillList";
