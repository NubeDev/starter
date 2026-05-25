// errors.ts — typed local-db failures.
//
// LOCAL-DB.md keeps this surface small: callers either get a row back or
// a `LocalDbError`. No leaking the raw `SQLiteError` shape — that lets us
// swap engines (op-sqlite, SQLCipher) later without touching call sites.

export class LocalDbError extends Error {
  readonly kind: 'open' | 'migrate' | 'query' | 'not-found' | 'conflict';
  readonly cause?: unknown;

  constructor(kind: LocalDbError['kind'], message: string, cause?: unknown) {
    super(message);
    this.name = 'LocalDbError';
    this.kind = kind;
    this.cause = cause;
  }
}
