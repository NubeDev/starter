import { fetchJson, StarterError } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { ProcessStats } from "@/api/extensions/types";

// `GET /api/v1/extensions/{id}/process` — live stats for a process-flavour
// extension's running child. The server returns `404 ext.process.not_running`
// for builtin/wasm/stopped/never-spawned; we map that single case to `null`
// ("no live process") so callers render a muted placeholder instead of an
// error. Any other failure propagates as a real error.
export async function getProcessStats(
  client: StarterClient,
  id: string,
): Promise<ProcessStats | null> {
  try {
    return await fetchJson<ProcessStats>(
      client,
      `${client.apiPrefix}/extensions/${encodeURIComponent(id)}/process`,
    );
  } catch (err) {
    if (StarterError.is(err, 404)) return null;
    throw err;
  }
}
