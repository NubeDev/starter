import * as React from "react";
import { cn } from "../lib/utils.js";
import type { SkillTrust } from "../types/index.js";

const TRUST_STYLES: Record<SkillTrust, string> = {
  approved:
    "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border-emerald-500/30",
  quarantined:
    "bg-amber-500/15 text-amber-700 dark:text-amber-300 border-amber-500/30",
};

const TRUST_LABEL: Record<SkillTrust, string> = {
  approved: "Approved",
  quarantined: "Quarantined",
};

export interface SkillTrustBadgeProps
  extends React.HTMLAttributes<HTMLSpanElement> {
  trust: SkillTrust;
}

export function SkillTrustBadge({
  trust,
  className,
  ...props
}: SkillTrustBadgeProps): React.ReactElement {
  return (
    <span
      data-slot="skill-trust-badge"
      data-trust={trust}
      className={cn(
        "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide",
        TRUST_STYLES[trust],
        className,
      )}
      {...props}
    >
      <span
        className={cn(
          "inline-block h-1.5 w-1.5 rounded-full",
          trust === "approved" ? "bg-emerald-500" : "bg-amber-500",
        )}
        aria-hidden
      />
      {TRUST_LABEL[trust]}
    </span>
  );
}
