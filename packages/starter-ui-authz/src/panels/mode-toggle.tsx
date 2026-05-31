// `<ModeToggle>` — Simple / Advanced pill for the access admin shell.
//
// Simple mode is the default for non-engineer operators: only the
// resource-centric tabs are mounted (Teams / Members / Pages). Advanced
// adds the engine primitives (Rules / Assignments / Audit log).
//
// Persisted per-browser via `localStorage["rubix.authz.admin.mode"]` so
// engineers who flip to Advanced stay there across reloads.

import { useCallback, useEffect, useState } from "react";
import { Button, cn } from "@nube/starter-ui-kit";

export type AuthzAdminMode = "simple" | "advanced";

const STORAGE_KEY = "rubix.authz.admin.mode";

function readStoredMode(): AuthzAdminMode {
  if (typeof window === "undefined") return "simple";
  try {
    const v = window.localStorage.getItem(STORAGE_KEY);
    return v === "advanced" ? "advanced" : "simple";
  } catch {
    return "simple";
  }
}

function writeStoredMode(mode: AuthzAdminMode) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(STORAGE_KEY, mode);
  } catch {
    // ignore — quota / private mode; toggle still works for the session.
  }
}

export function useAuthzAdminMode(): [AuthzAdminMode, (m: AuthzAdminMode) => void] {
  const [mode, setMode] = useState<AuthzAdminMode>("simple");

  // Hydrate after mount so SSR / first paint matches `simple`, then upgrade
  // to the persisted value without a flash of the wrong tab set.
  useEffect(() => {
    setMode(readStoredMode());
  }, []);

  const update = useCallback((next: AuthzAdminMode) => {
    setMode(next);
    writeStoredMode(next);
  }, []);

  return [mode, update];
}

export interface ModeToggleProps {
  mode: AuthzAdminMode;
  onChange: (mode: AuthzAdminMode) => void;
  className?: string;
}

export function ModeToggle({ mode, onChange, className }: ModeToggleProps) {
  return (
    <div
      role="group"
      aria-label="Access UI mode"
      className={cn(
        "inline-flex items-center rounded-md border border-border bg-muted/50 p-0.5",
        className,
      )}
    >
      <ModeButton
        active={mode === "simple"}
        onClick={() => onChange("simple")}
        label="Simple"
      />
      <ModeButton
        active={mode === "advanced"}
        onClick={() => onChange("advanced")}
        label="Advanced"
      />
    </div>
  );
}

function ModeButton({
  active,
  onClick,
  label,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <Button
      type="button"
      size="sm"
      variant={active ? "default" : "ghost"}
      aria-pressed={active}
      onClick={onClick}
      className={cn(
        "h-7 px-3 text-xs",
        active ? "shadow-sm" : "text-muted-foreground",
      )}
    >
      {label}
    </Button>
  );
}
