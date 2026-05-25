// connection/set-active.ts — verb. Write the active_connection_id key.

import type { Database } from '../open';

export const ACTIVE_KEY = 'active_connection_id';

export async function setActiveConnection(
  db: Database,
  id: string | null,
): Promise<void> {
  await db.runAsync(
    'INSERT INTO app_state (k, v) VALUES (?, ?) ON CONFLICT(k) DO UPDATE SET v = excluded.v',
    [ACTIVE_KEY, id ?? ''],
  );
}
