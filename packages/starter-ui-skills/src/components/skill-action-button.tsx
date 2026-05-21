import * as React from "react";
import { cn } from "../lib/utils.js";

type Variant = "primary" | "destructive" | "ghost";

const VARIANTS: Record<Variant, string> = {
  primary:
    "bg-primary text-primary-foreground hover:enabled:opacity-90 disabled:opacity-40",
  destructive:
    "bg-destructive text-destructive-foreground hover:enabled:opacity-90 disabled:opacity-40",
  ghost:
    "bg-transparent text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-40",
};

export interface SkillActionButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  loading?: boolean;
  variant?: Variant;
  confirmMessage?: string;
}

export const SkillActionButton = React.forwardRef<
  HTMLButtonElement,
  SkillActionButtonProps
>(function SkillActionButton(
  {
    className,
    loading,
    variant = "ghost",
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
      data-loading={loading ? "" : undefined}
      disabled={disabled || loading}
      onClick={handleClick}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs font-medium transition disabled:cursor-not-allowed",
        VARIANTS[variant],
        className,
      )}
      {...props}
    >
      {loading ? (
        <span
          aria-hidden
          className="inline-block h-3 w-3 animate-spin rounded-full border-2 border-current border-r-transparent"
        />
      ) : null}
      {children}
    </button>
  );
});
