// connection/touch.ts — verb. Record a successful health probe.
//
// Per LOCAL-DB.md §Health probe, this is the single update site for
// `last_seen_at` + `agent_version`. Three call sites:
//
//   1. connection/create.ts after the first /healthz on add.
//   2. <SduiPage> boot path (first /api/v1/ui/resolve response).
//   3. connections/index.tsx pull-to-refresh.

import type { Database } from '../open';

export async function touchConnection(
  db: Database,
  id: string,
  agentVersion: string | null,
): Promise<void> {
  await db.runAsync(
    `UPDATE connection
        SET last_seen_at = ?,
            agent_version = COALESCE(?, agent_version)
      WHERE id = ?`,
    [Date.now(), agentVersion, id],
  );
}
