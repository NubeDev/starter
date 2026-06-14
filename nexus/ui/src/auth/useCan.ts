import { can, type Action } from "@/auth/can";
import { usePrincipal } from "@/auth/usePrincipal";

// Authorization gate for the UI: hide/disable what the current principal
// can't do. Reads the cached principal (`usePrincipal`) and defers to the
// pure `can` core. While `/me` is loading the principal is undefined, so
// every check is denied (fail closed) — gated UI stays hidden until the
// real grant is known, never optimistically shown (F0).
export function useCan(action: Action, scope?: string, team?: string): boolean {
  const { data: principal } = usePrincipal();
  return can(principal, action, scope, team);
}
