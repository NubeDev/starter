import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type {
  CleanupPreview,
  EnablementResponse,
  PurgeResponse,
  UninstallResponse,
} from "@/api/extensions/types";

// Lifecycle controls for one extension (all admin-gated server-side):
// enable/disable flip the enablement row, restart bounces a process
// runtime, cleanup previews what a purge would remove, and uninstall
// removes the extension (optionally purging everything it left behind).

export function enableExtension(
  client: StarterClient,
  id: string,
): Promise<EnablementResponse> {
  return fetchJson<EnablementResponse>(
    client,
    `${client.apiPrefix}/extensions/${encodeURIComponent(id)}/enable`,
    { method: "POST", headers: readCsrfHeader() },
  );
}

export function disableExtension(
  client: StarterClient,
  id: string,
): Promise<EnablementResponse> {
  return fetchJson<EnablementResponse>(
    client,
    `${client.apiPrefix}/extensions/${encodeURIComponent(id)}/disable`,
    { method: "POST", headers: readCsrfHeader() },
  );
}

// Restart is treated as opaque — the caller invalidates the list and
// re-reads the state.
export function restartExtension(
  client: StarterClient,
  id: string,
): Promise<unknown> {
  return fetchJson<unknown>(
    client,
    `${client.apiPrefix}/extensions/${encodeURIComponent(id)}/restart`,
    { method: "POST", headers: readCsrfHeader() },
  );
}

// Dry-run manifest of what a purge would delete (tables, caches, the
// bundle on disk). Always fetched before the destructive confirm.
export function cleanupPreview(
  client: StarterClient,
  id: string,
): Promise<CleanupPreview> {
  return fetchJson<CleanupPreview>(
    client,
    `${client.apiPrefix}/extensions/${encodeURIComponent(id)}/cleanup`,
  );
}

// `DELETE …?purge=true` — uninstall and purge everything from the
// cleanup manifest.
export function purgeExtension(
  client: StarterClient,
  id: string,
): Promise<PurgeResponse> {
  return fetchJson<PurgeResponse>(
    client,
    `${client.apiPrefix}/extensions/${encodeURIComponent(id)}?purge=true`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
}

// Plain uninstall — keeps the data the extension created.
export function uninstallExtension(
  client: StarterClient,
  id: string,
): Promise<UninstallResponse> {
  return fetchJson<UninstallResponse>(
    client,
    `${client.apiPrefix}/extensions/${encodeURIComponent(id)}`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
}
