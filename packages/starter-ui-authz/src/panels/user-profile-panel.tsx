// `<UserProfilePanel>` — rubix system-level user detail card.
// Mounted by Agent B's master-detail shell on the "Profile" tab
// of a selected user node. Reads its data via `userOps.get()`
// and supports Disable + Undo last actions.

import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@nube/starter-ui-kit";
import { useUserOps, type UserOps, type UserRecord } from "./user-ops.js";
import { StateRow } from "./_common.js";

export interface UserProfilePanelProps {
  userId: string;
  /** Optional explicit override; falls back to `useUserOps()`. */
  userOps?: UserOps;
}

export function UserProfilePanel({ userId, userOps }: UserProfilePanelProps) {
  const ctxOps = useUserOps();
  const ops = userOps ?? ctxOps;

  if (!ops) {
    return <StateRow variant="error">UserOps not provided.</StateRow>;
  }

  const user: UserRecord | undefined = ops.get(userId);
  if (!user) {
    return <StateRow variant="empty">User not found.</StateRow>;
  }

  const disabled = user.disabled_at_ms != null;

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between gap-4">
        <CardTitle>{user.email}</CardTitle>
        <div className="flex gap-2">
          <Button
            size="sm"
            variant="outline"
            disabled={!ops.undoLast || ops.isUndoing}
            onClick={() => ops.undoLast?.()}
          >
            Undo last
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={disabled || !ops.disable || ops.isDisabling}
            onClick={() => ops.disable?.(user.user_id)}
            title={
              disabled
                ? "User already disabled. Re-enable is not yet supported by the backend."
                : undefined
            }
          >
            {disabled ? "Disabled" : "Disable"}
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <dl className="grid grid-cols-[120px_1fr] gap-y-3 text-sm">
          <dt className="text-[color:var(--color-subtle)]">User id</dt>
          <dd className="font-mono">{user.user_id}</dd>
          <dt className="text-[color:var(--color-subtle)]">Email</dt>
          <dd>{user.email}</dd>
          <dt className="text-[color:var(--color-subtle)]">
            Role
            <span
              className="ml-1 cursor-help text-xs"
              title="Rubix system role (separate from per-tenant membership role)."
            >
              (?)
            </span>
          </dt>
          <dd>{user.role}</dd>
          <dt className="text-[color:var(--color-subtle)]">Status</dt>
          <dd>{disabled ? "Disabled" : "Active"}</dd>
        </dl>
      </CardContent>
    </Card>
  );
}
