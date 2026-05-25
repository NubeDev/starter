// connection/get.ts — verb. Read one connection by id.

import type { Database } from '../open';
import { type Connection, type ConnectionRow, rowToConnection } from './types';

export async function getConnection(db: Database, id: string): Promise<Connection | null> {
  const row = await db.getFirstAsync<ConnectionRow>(
    'SELECT * FROM connection WHERE id = ?',
    [id],
  );
  return row ? rowToConnection(row) : null;
}
