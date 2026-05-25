// token/contract.ts — secure-token surface.
//
// One small interface so the verbs in this folder are unit-testable
// against an in-memory mock and so an op-sqlite-backed alternative
// (when/if SQLCipher lands per LOCAL-DB.md §Secret handling) can be
// dropped in without touching call sites.

export interface SecureTokenStore {
  get(connectionId: string): Promise<string | null>;
  put(connectionId: string, token: string): Promise<void>;
  clear(connectionId: string): Promise<void>;
}
