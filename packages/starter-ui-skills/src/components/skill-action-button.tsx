import * as React from "react";
import { cn } from "../lib/utils.js";

type Variant = "default" | "destructive" | "outline" | "secondary" | "ghost";
type Size = "default" | "sm" | "xs";

const VARIANTS: Record<Variant, string> = {
  default: "bg-primary text-primary-foreground hover:bg-primary/90",
  destructive: "bg-destructive text-white hover:bg-destructive/90",
  outline:
    "border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:border-input dark:bg-input/30 dark:hover:bg-input/50",
  secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
  ghost: "hover:bg-accent hover:text-accent-foreground",
};

const SIZES: Record<Size, string> = {
  default: "h-9 px-4 py-2 [&>svg]:size-4",
  sm: "h-8 gap-1.5 rounded-md px-3 [&>svg]:size-3.5",
  xs: "h-7 gap-1 rounded-md px-2 text-[11px] [&>svg]:size-3",
};

export interface SkillActionButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  loading?: boolean;
  variant?: Variant;
  size?: Size;
  confirmMessage?: string;
}

export const SkillActionButton = React.forwardRef<
  HTMLButtonElement,
  SkillActionButtonProps
>(function SkillActionButton(
  {
    className,
    loading,
    variant = "default",
    size = "default",
    confirmMessage,
    onClick,
    children,
    disabled,
    ...props
  },
  ref,
) {
  const handleClick: React.MouseEventHandler<HTMLButtonElement> = (e) => {
    if (confirmMessage) {
      const ok =
        typeof window !== "undefined" ? window.confirm(confirmMessage) : true;
      if (!ok) {
        e.preventDefault();
        return;
      }
    }
    onClick?.(e);
  };
  return (
    <button
      ref={ref}
      type="button"
      data-slot="button"
      data-loading={loading ? "" : undefined}
      data-variant={variant}
      data-size={size}
      disabled={disabled || loading}
      onClick={handleClick}
      className={cn(
        "inline-flex shrink-0 items-center justify-center gap-1.5 rounded-md font-medium whitespace-nowrap transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring/40 disabled:pointer-events-none disabled:opacity-50 [&>svg]:shrink-0",
        VARIANTS[variant],
        SIZES[size],
        className,
      )}
      {...props}
    >
      {loading ? (
        <span
          aria-hidden
          className="inline-block size-3 animate-spin rounded-full border-2 border-current border-r-transparent"
        />
      ) : null}
      {children}
    </button>
  );
});
