// Tiny shared primitives — used by virtually every dashboard
// section. Kept in one file so we don't grow a folder of one-liner
// modules.

import * as React from "react";

export function SectionHeader({
  title, subtitle, right, icon,
}: {
  title: React.ReactNode;
  subtitle?: React.ReactNode;
  right?: React.ReactNode;
  /** Optional leading glyph. Use one of the Lucide-style icons
   *  from `./icons`. Coloured by surrounding text. */
  icon?: React.ReactNode;
}): React.ReactElement {
  return (
    <header className="flex items-baseline justify-between gap-3 mb-3">
      <div className="flex items-start gap-2 min-w-0">
        {icon ? (
          <span className="text-muted-foreground/70 mt-0.5 shrink-0">
            {icon}
          </span>
        ) : null}
        <div className="min-w-0">
          <h4 className="text-sm font-semibold tracking-tight">{title}</h4>
          {subtitle ? <p className="text-xs text-muted-foreground">{subtitle}</p> : null}
        </div>
      </div>
      {right ? <div className="shrink-0">{right}</div> : null}
    </header>
  );
}

export function Field({
  label, icon, children,
}: {
  label: string;
  /** Optional small glyph rendered before the label in the eyebrow row. */
  icon?: React.ReactNode;
  children: React.ReactNode;
}): React.ReactElement {
  return (
    <div className="flex flex-col gap-1 min-w-[12rem]">
      <span className="ext-eyebrow flex items-center gap-1.5">
        {icon ? <span className="opacity-70">{icon}</span> : null}
        {label}
      </span>
      {children}
    </div>
  );
}

export function SegBtn({
  active, onClick, children, title,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
  title?: string;
}): React.ReactElement {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      className={
        "px-3 py-1 text-xs rounded-md border cursor-pointer transition-colors " +
        (active
          ? "bg-primary text-primary-foreground border-primary shadow-[0_0_18px_-6px_var(--color-primary)]"
          : "bg-transparent text-foreground border-border/60 hover:bg-accent")
      }
    >
      {children}
    </button>
  );
}

export function PillBtn({
  active, onClick, children,
}: {
  active?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}): React.ReactElement {
  return (
    <button
      type="button"
      onClick={onClick}
      className={
        "px-2 py-0.5 text-xs rounded-full border cursor-pointer transition-colors " +
        (active
          ? "bg-primary/15 text-foreground border-primary/50"
          : "bg-transparent text-muted-foreground border-border/40 hover:bg-accent")
      }
    >
      {children}
    </button>
  );
}

export function Empty({ children }: { children: React.ReactNode }): React.ReactElement {
  return <p className="text-sm text-muted-foreground italic">{children}</p>;
}

export function LoadingToast({
  show, onClose,
}: {
  show: boolean;
  onClose: () => void;
}): React.ReactElement | null {
  if (!show) return null;
  return (
    <div
      role="status"
      aria-live="polite"
      className={
        "ext-glass flex items-center gap-2 px-3 py-2 text-xs " +
        "border border-primary/40 bg-primary/10 shadow-[0_0_24px_-8px_var(--color-primary)] " +
        "animate-in fade-in slide-in-from-top-2"
      }
    >
      <span
        aria-hidden="true"
        className="inline-block h-3.5 w-3.5 rounded-full border-2 border-primary/30 border-t-primary animate-spin"
      />
      <span className="text-foreground/90">
        Loading dashboard data… you can keep working.
      </span>
      <button
        type="button"
        onClick={onClose}
        aria-label="Dismiss loading indicator"
        className={
          "ml-auto inline-flex h-5 w-5 items-center justify-center rounded-full " +
          "text-muted-foreground hover:text-foreground hover:bg-accent " +
          "focus:outline-none focus-visible:ring-2 focus-visible:ring-primary cursor-pointer"
        }
      >
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round">
          <path d="M1.5 1.5 L8.5 8.5 M8.5 1.5 L1.5 8.5" />
        </svg>
      </button>
    </div>
  );
}
