import * as React from "react";
import { cn } from "../lib/utils.js";
import type { SkillTrust } from "../types/index.js";

const TRUST_STYLES: Record<SkillTrust, string> = {
  approved: "bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
  quarantined: "bg-amber-500/10 text-amber-700 dark:text-amber-300",
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
      data-slot="badge"
      data-trust={trust}
      className={cn(
        "rounded px-1 py-0.5 text-[9px] font-medium uppercase tracking-wide",
        TRUST_STYLES[trust],
        className,
      )}
      {...props}
    >
      {TRUST_LABEL[trust]}
    </span>
  );
}
