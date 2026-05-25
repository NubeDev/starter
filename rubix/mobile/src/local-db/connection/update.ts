// connection/update.ts — verb. Rename / recolour / re-base-url.

import type { Database } from '../open';

export interface UpdateConnectionInput {
  label?: string;
  baseUrl?: string;
  colour?: string;
  notes?: string;
}

export async function updateConnection(
  db: Database,
  id: string,
  patch: UpdateConnectionInput,
): Promise<void> {
  const fields: string[] = [];
  const values: unknown[] = [];
  if (patch.label !== undefined) {
    fields.push('label = ?');
    values.push(patch.label.trim());
  }
  if (patch.baseUrl !== undefined) {
    fields.push('base_url = ?');
    values.push(patch.baseUrl.replace(/\/+$/, ''));
  }
  if (patch.colour !== undefined) {
    fields.push('colour = ?');
    values.push(patch.colour.trim());
  }
  if (patch.notes !== undefined) {
    fields.push('notes = ?');
    values.push(patch.notes.trim());
  }
  if (fields.length === 0) return;
  values.push(id);
  await db.runAsync(
    `UPDATE connection SET ${fields.join(', ')} WHERE id = ?`,
    values as never[],
  );
}
