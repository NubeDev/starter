// state/last-sync.ts — verb. Per-connection sync hint.

import type { Database } from '../open';

export async function setLastSync(
  db: Database,
  connectionId: string,
  whenMs: number = Date.now(),
): Promise<void> {
  await db.runAsync(
    `INSERT INTO connection_state (connection_id, last_synced_at)
     VALUES (?, ?)
     ON CONFLICT(connection_id) DO UPDATE SET last_synced_at = excluded.last_synced_at`,
    [connectionId, whenMs],
  );
}
