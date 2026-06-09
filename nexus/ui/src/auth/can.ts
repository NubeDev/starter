import type { MeResponse } from "@/api/types";

export type Action = "read" | "write" | "admin";

// Pure authorization core. Decides whether a principal may perform an
// action, given the coarse role plus optional fine-grained checks:
//   - `scope`   — an explicit scope string that grants the action on its
//                 own, even for a reader (e.g. "datasources:write").
//   - `team`    — require membership of a named team.
// Role ladder: admin ⊇ writer ⊇ reader. A null/absent principal is always
// denied (fail closed) — the UI shows loading/empty until `/me` resolves,
// never an optimistic allow.
export function can(
  principal: MeResponse | null | undefined,
  action: Action,
  scope?: string,
  team?: string,
): boolean {
  if (!principal) return false;
  if (team && !principal.teams?.includes(team)) return false;
  if (scope) return principal.scopes?.includes(scope) ?? false;

  switch (principal.role) {
    case "admin":
      return true;
    case "writer":
      return action === "read" || action === "write";
    default:
      return action === "read";
  }
}
