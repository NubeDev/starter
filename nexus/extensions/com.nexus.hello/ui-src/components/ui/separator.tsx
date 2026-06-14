// `separator.tsx` — a minimal shadcn/ui Separator (plain div; the shadcn
// version wraps Radix, which we avoid to keep the bundle dependency-free).
import * as React from "react";

import { cn } from "../../lib/utils";

function Separator({
  className,
  orientation = "horizontal",
  ...props
}: React.ComponentProps<"div"> & { orientation?: "horizontal" | "vertical" }) {
  return (
    <div
      data-slot="separator"
      role="separator"
      aria-orientation={orientation}
      className={cn(
        "shrink-0 bg-border",
        orientation === "horizontal" ? "h-px w-full" : "h-full w-px",
        className,
      )}
      {...props}
    />
  );
}

export { Separator };
