// MEMORY.md Phase M-D — frontend wiring for agent-session
// persistence. Three concerns live here, deliberately small:
//
//   1. Stable surface → session-id mapping in `localStorage`, so the
//      page builder rehydrates the SAME session across reloads.
//   2. Thin `fetch` wrappers for `POST /api/sessions`,
//      `GET /api/sessions/:id/artifacts/:key`, and
//      `GET /api/sessions/:id/artifacts/:key/versions`. These
//      endpoints live in the flow-agent server, not the merged
//      workspace `openapi.json`, so the typed client codegen does
//      NOT cover them — plain fetch is fine.
//   3. A "surface key" helper so /pages/new and /pages/:id/edit
//      bind to distinct sessions.

import type { UiComponentTree } from "@nube/starter-sdui-react";

const SESSION_KEY_PREFIX = "flow-agent:builder-session:";

/** localStorage key under which a session id is persisted for the
 *  given surface (e.g. `"new"`, `"page:01HZX..."`). */
function storageKey(surface: string): string {
  return `${SESSION_KEY_PREFIX}${surface}`;
}

/** Choose a stable surface id. New-page surfaces share one drafting
 *  session per browser; edit surfaces are keyed by the saved page id
 *  so revisiting `/pages/:id/edit` continues the prior session. */
export function builderSurfaceKey(editingId: string | undefined): string {
  return editingId ? `page:${editingId}` : "new";
}

export function loadSessionId(surface: string): string | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage.getItem(storageKey(surface));
  } catch {
    return null;
  }
}

export function saveSessionId(surface: string, id: string): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(storageKey(surface), id);
  } catch {
    /* quota / privacy mode — falling back to stateless is fine */
  }
}

export function clearSessionId(surface: string): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.removeItem(storageKey(surface));
  } catch {
    /* ignore */
  }
}

export interface SessionArtifact {
  session_id: string;
  key: string;
  version: number;
  parent_version: number | null;
  value: unknown;
  value_bytes: number;
  produced_by_seq: number | null;
  updated_at: string;
}

export interface ArtifactVersionMeta {
  version: number;
  parent_version: number | null;
  value_bytes: number;
  produced_by_seq: number | null;
  updated_at: string;
}

/** `POST /api/sessions` — create a fresh agent session and return
 *  its id. We always pass `kind: "page-builder"` so retention /
 *  metrics can segment surfaces later. */
export async function createSession(
  signal?: AbortSignal,
): Promise<string> {
  const res = await fetch("/api/sessions", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ kind: "page-builder" }),
    signal,
  });
  if (!res.ok) {
    throw new Error(`POST /api/sessions: HTTP ${res.status}`);
  }
  const body = (await res.json()) as { session_id: string };
  return body.session_id;
}

/** `GET /api/sessions/:id/artifacts/:key` — latest snapshot, or
 *  `null` on 404. Other errors throw so the caller can decide. */
export async function getLatestArtifact(
  sessionId: string,
  key: string,
  signal?: AbortSignal,
): Promise<SessionArtifact | null> {
  const res = await fetch(
    `/api/sessions/${encodeURIComponent(sessionId)}/artifacts/${encodeURIComponent(key)}`,
    { signal },
  );
  if (res.status === 404) return null;
  if (!res.ok) {
    throw new Error(`GET artifact: HTTP ${res.status}`);
  }
  return (await res.json()) as SessionArtifact;
}

/** `GET /api/sessions/:id/artifacts/:key/versions` — metadata only,
 *  newest first. Used by the undo / version-picker UI; bodies are
 *  fetched lazily via `getArtifactVersion`. */
export async function listArtifactVersions(
  sessionId: string,
  key: string,
  signal?: AbortSignal,
): Promise<ArtifactVersionMeta[]> {
  const res = await fetch(
    `/api/sessions/${encodeURIComponent(sessionId)}/artifacts/${encodeURIComponent(key)}/versions`,
    { signal },
  );
  if (!res.ok) {
    throw new Error(`GET artifact versions: HTTP ${res.status}`);
  }
  return (await res.json()) as ArtifactVersionMeta[];
}

/** `GET /api/sessions/:id/artifacts/:key/versions/:version` —
 *  historical body for the undo target. */
export async function getArtifactVersion(
  sessionId: string,
  key: string,
  version: number,
  signal?: AbortSignal,
): Promise<SessionArtifact | null> {
  const res = await fetch(
    `/api/sessions/${encodeURIComponent(sessionId)}/artifacts/${encodeURIComponent(key)}/versions/${version}`,
    { signal },
  );
  if (res.status === 404) return null;
  if (!res.ok) {
    throw new Error(`GET artifact v${version}: HTTP ${res.status}`);
  }
  return (await res.json()) as SessionArtifact;
}

/** Convenience: the `tree` artifact value is a `UiComponentTree`.
 *  Narrowed here so the caller doesn't repeat the cast. */
export function asTree(artifact: SessionArtifact | null): UiComponentTree | null {
  if (!artifact) return null;
  // The backend stores whatever JSON the model produced; we trust
  // the shape because the same backend wrote it. If it's malformed,
  // the SDUI renderer will surface that itself.
  return artifact.value as UiComponentTree;
}
