// connection/create.ts — verb. Add a new server.
//
// Per LOCAL-DB.md §Schema: no UNIQUE on base_url. Operators legitimately
// add the same URL twice with different labels (tunnels, dev/prod toggle,
// single host serving two logical tenants). We warn in the result if a
// duplicate is detected, but never reject.

import { ulid } from 'ulid';

import type { Database } from '../open';
import { listConnections } from './list';
import type { Connection } from './types';

export interface CreateConnectionInput {
  label: string;
  baseUrl: string;
  colour?: string;
  notes?: string;
}

export interface CreateConnectionResult {
  readonly connection: Connection;
  /**
   * `'ok'` for the happy path. `'duplicate-base-url'` when an existing
   * row already has this `baseUrl` — the UI should surface a soft warning
   * but the row was still inserted.
   */
  readonly status: 'ok' | 'duplicate-base-url';
}

export async function createConnection(
  db: Database,
  input: CreateConnectionInput,
): Promise<CreateConnectionResult> {
  const existing = await listConnections(db);
  const status = existing.some((c) => c.baseUrl === input.baseUrl)
    ? 'duplicate-base-url'
    : 'ok';

  const connection: Connection = {
    id: ulid(),
    label: input.label.trim(),
    baseUrl: input.baseUrl.replace(/\/+$/, ''),
    colour: (input.colour ?? '').trim(),
    createdAt: Date.now(),
    lastSeenAt: null,
    agentVersion: null,
    notes: (input.notes ?? '').trim(),
  };

  await db.runAsync(
    `INSERT INTO connection
        (id, label, base_url, colour, created_at, last_seen_at, agent_version, notes)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    [
      connection.id,
      connection.label,
      connection.baseUrl,
      connection.colour,
      connection.createdAt,
      connection.lastSeenAt,
      connection.agentVersion,
      connection.notes,
    ],
  );

  return { connection, status };
}
