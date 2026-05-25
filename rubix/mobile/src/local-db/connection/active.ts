// connection/active.ts — verb. Read the active connection (one row).
//
// Returns `null` when the active id is empty (fresh install) OR when the
// id refers to a connection that has since been deleted — defensive
// because `app_state` does not have a foreign key into `connection`.

import type { Database } from '../open';
import { getConnection } from './get';
import { ACTIVE_KEY } from './set-active';
import type { Connection } from './types';

export async function getActiveConnection(db: Database): Promise<Connection | null> {
  const row = await db.getFirstAsync<{ v: string }>(
    'SELECT v FROM app_state WHERE k = ?',
    [ACTIVE_KEY],
  );
  const id = row?.v ?? '';
  if (!id) return null;
  return getConnection(db, id);
}

export async function getActiveConnectionId(db: Database): Promise<string | null> {
  const row = await db.getFirstAsync<{ v: string }>(
    'SELECT v FROM app_state WHERE k = ?',
    [ACTIVE_KEY],
  );
  return row?.v ? row.v : null;
}
