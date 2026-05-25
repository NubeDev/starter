// state/last-page.ts — verb. Per-connection resume state.

import type { Database } from '../open';

export async function getLastPage(
  db: Database,
  connectionId: string,
): Promise<string | null> {
  const row = await db.getFirstAsync<{ last_opened_page_ref: string | null }>(
    'SELECT last_opened_page_ref FROM connection_state WHERE connection_id = ?',
    [connectionId],
  );
  return row?.last_opened_page_ref ?? null;
}

export async function setLastPage(
  db: Database,
  connectionId: string,
  pageRef: string,
): Promise<void> {
  await db.runAsync(
    `INSERT INTO connection_state (connection_id, last_opened_page_ref)
     VALUES (?, ?)
     ON CONFLICT(connection_id) DO UPDATE SET last_opened_page_ref = excluded.last_opened_page_ref`,
    [connectionId, pageRef],
  );
}
