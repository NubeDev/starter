import * as React from "react";
import { cn, shortHash } from "../lib/utils.js";

export interface SkillHashProps extends React.HTMLAttributes<HTMLSpanElement> {
  hash: string;
  /** Show full hash on hover via `title`. Default: true. */
  tooltip?: boolean;
  /** Prefix label, e.g. `"bundle"`. */
  label?: string;
}

export function SkillHash({
  hash,
  tooltip = true,
  label,
  className,
  ...props
}: SkillHashProps): React.ReactElement {
  return (
    <span
      data-slot="skill-hash"
      title={tooltip ? hash : undefined}
      className={cn(
        "inline-flex items-center gap-1 rounded-md bg-muted px-1.5 py-0.5 font-mono text-[11px] text-muted-foreground",
        className,
      )}
      {...props}
    >
      {label ? <span className="opacity-60">{label}</span> : null}
      <span>{shortHash(hash)}</span>
    </span>
  );
}
