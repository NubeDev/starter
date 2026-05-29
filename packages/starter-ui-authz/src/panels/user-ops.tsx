// `UserOps` / `UsersAdminOps` — host-supplied adapter for rubix
// system user CRUD. Mirrors the `userDirectory` pattern: this
// package never imports `@nube/rubix-client-react` directly; the
// rubix host wires its `useUserList` / `useUserCreate` etc hooks
// into this shape and passes it to `<AuthzAdmin>`.

import { createContext, useContext, type ReactNode } from "react";

export interface UserRecord {
  user_id: string;
  email: string;
  role: string;
  disabled_at_ms?: number | null;
}

export interface UserOps {
  get(userId: string): UserRecord | undefined;
  disable?(userId: string): Promise<void>;
  undoLast?(): Promise<void>;
  isDisabling?: boolean;
  isUndoing?: boolean;
}

export interface UsersAdminOps extends UserOps {
  list(): { users: UserRecord[] } | undefined;
  create?(input: { email: string; role: string }): Promise<void>;
  isCreating?: boolean;
}

const UserOpsContext = createContext<UsersAdminOps | null>(null);

export interface UserOpsProviderProps {
  value: UsersAdminOps | null | undefined;
  children: ReactNode;
}

export function UserOpsProvider({ value, children }: UserOpsProviderProps) {
  return (
    <UserOpsContext.Provider value={value ?? null}>
      {children}
    </UserOpsContext.Provider>
  );
}

export function useUserOps(): UsersAdminOps | null {
  return useContext(UserOpsContext);
}
