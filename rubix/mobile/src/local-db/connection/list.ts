// connection/list.ts — verb. Read all connections, oldest first.

import type { Database } from '../open';
import { type Connection, type ConnectionRow, rowToConnection } from './types';

export async function listConnections(db: Database): Promise<Connection[]> {
  const rows = await db.getAllAsync<ConnectionRow>(
    'SELECT * FROM connection ORDER BY created_at ASC',
  );
  return rows.map(rowToConnection);
}
