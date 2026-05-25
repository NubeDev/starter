// migrations/index.ts — ordered list of migration sources.
//
// One file per migration; numbered, monotonic, never edited after merge
// (LOCAL-DB.md §Schema). The migrations live as `.sql` files (not inline
// strings) so they grep cleanly and snapshot side-by-side with the agent-
// side migrations folder layout. The `.sql` ambient module declaration
// in `../../../app.d.ts` keeps TypeScript happy; metro resolves them at
// runtime via Expo's asset pipeline.

import sql0001 from './0001_connections.sql';
import sql0002 from './0002_active_connection.sql';
import sql0003 from './0003_per_connection_state.sql';

export interface Migration {
  /** Monotonic file-name prefix, e.g. `'0001_connections'`. */
  readonly id: string;
  /** Raw SQL — may contain multiple statements (semicolon-separated). */
  readonly sql: string;
}

/**
 * Ordered list. Append-only. NEVER reorder or delete an entry once a
 * migration has shipped to a user's device — its id is the on-device
 * record of "what has been applied".
 */
export const MIGRATIONS: readonly Migration[] = [
  { id: '0001_connections', sql: sql0001 },
  { id: '0002_active_connection', sql: sql0002 },
  { id: '0003_per_connection_state', sql: sql0003 },
];
