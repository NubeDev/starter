// Create a brand-new user account and add them to this tenant in one step.
//
// The Members panel below only adds EXISTING users (by id/email); this form is
// the missing "invite a new person" path. It posts to POST /v1/tenants/{id}/users
// (validate + argon2-hash + create + add membership, server-side) and refreshes
// the member list so the new person appears — and becomes pickable in the team
// dropdown — immediately.

import { useState, type FormEvent } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { UserPlus } from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";
import type { MembershipView, TenantRole } from "@nube/starter-client-ts";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@nube/starter-ui-kit/components/card";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

const ROLES: TenantRole[] = ["reader", "writer", "admin"];

export function CreateUserForm({ tenantId }: { tenantId: string }) {
  const client = useStarterClient();
  const queryClient = useQueryClient();

  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<TenantRole>("reader");

  const create = useMutation<MembershipView, Error, void>({
    mutationFn: () =>
      client.createTenantUser(tenantId, { email: email.trim(), password, role }),
    onSuccess: () => {
      // Refresh the member list (and anything keyed under it, like the picker).
      queryClient.invalidateQueries({ queryKey: ["authz"] });
      setEmail("");
      setPassword("");
      setRole("reader");
    },
  });

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (email.trim() && password) create.mutate();
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <UserPlus className="size-4" />
          Invite a new user
        </CardTitle>
      </CardHeader>
      <CardContent>
        <form
          onSubmit={onSubmit}
          className="grid grid-cols-1 gap-3 sm:grid-cols-[1.5fr_1.5fr_1fr_auto] sm:items-end"
        >
          <div className="grid gap-1">
            <Label htmlFor="cu-email">Email</Label>
            <Input
              id="cu-email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.currentTarget.value)}
              placeholder="person@company.com"
              required
            />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="cu-password">Temporary password</Label>
            <Input
              id="cu-password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.currentTarget.value)}
              placeholder="at least 12 characters"
              required
            />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="cu-role">Role</Label>
            <Select value={role} onValueChange={(v) => setRole(v as TenantRole)}>
              <SelectTrigger id="cu-role">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ROLES.map((r) => (
                  <SelectItem key={r} value={r}>
                    {r}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <Button type="submit" disabled={create.isPending}>
            {create.isPending ? "Creating…" : "Create user"}
          </Button>
        </form>
        {create.error ? (
          <p role="alert" className="mt-2 text-xs text-destructive">
            {humanError(create.error.message)}
          </p>
        ) : null}
      </CardContent>
    </Card>
  );
}

// The server returns coarse error codes; surface the common ones in plain words.
function humanError(raw: string): string {
  if (raw.includes("email_taken") || raw.includes("409"))
    return "That email is already registered.";
  if (raw.includes("invalid_input") || raw.includes("400"))
    return "Check the email and use a password of at least 12 characters.";
  return raw;
}
