// connection/delete.ts — verb. Remove a connection.
//
// Cascade-clears `connection_state` via the FK (LOCAL-DB.md). Also clears
// the secure-store token for this connection — bearer secrets and their
// connection row have the same lifetime. Caller passes the secure-store
// handle so this verb stays unit-testable against a mock.

import type { Database } from '../open';
import type { SecureTokenStore } from '../token/contract';

export async function deleteConnection(
  db: Database,
  secureStore: SecureTokenStore,
  id: string,
): Promise<void> {
  await db.runAsync('DELETE FROM connection WHERE id = ?', [id]);
  // Clear the secure-store token AFTER the row is gone so a crash between
  // the two steps leaves the system in the safer of the two intermediate
  // states (orphan secret, no row referencing it — gets evicted next boot
  // via `tokens/clear-orphans.ts` in the follow-up that adds it).
  await secureStore.clear(id);
}
