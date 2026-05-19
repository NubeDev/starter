// `/ui/theme/*` client methods. Provides the persistence surface the
// theme editor in `@nube/starter-ui-core` expects. The endpoints are
// part of the starter REST contract documented in
// `crates/starter-server` (route module ships with the backend, see
// the upcoming `theme` feature flag).
//
// Wire types are hand-rolled here (not codegenned) because the
// server-side handler hasn't landed yet — they are kept narrow and
// well-named so the codegen swap is a no-op when it does.
//
// Multipart-style asset uploads use raw `fetch` with the file's
// declared MIME type as `Content-Type`; the server stores the body
// bytes verbatim and returns `204 No Content` on success.

import { StarterClient } from "../client/client.js";
import { StarterError } from "../error/starter-error.js";

/** Per-mode token map. Open-ended to track the editor's
 * `ThemeStyleKey` set without coupling the two packages. */
export type ThemeStyleProps = Record<string, string>;

export interface ThemeStyles {
  light: ThemeStyleProps;
  dark: ThemeStyleProps;
}

export interface ThemeShellConfig {
  nav_title: string;
  hide_features: string[];
}

export interface ThemeDocument {
  theme_styles: ThemeStyles;
  shell: ThemeShellConfig;
  logo_url?: string | null;
  favicon_url?: string | null;
}

export interface ThemeSaveRequest {
  theme_styles: ThemeStyles;
  shell: ThemeShellConfig;
}

declare module "../client/client.js" {
  interface StarterClient {
    /** GET `/api/v1/ui/theme`. Returns the stored document (or a
     * blank document for unconfigured tenants). */
    themeGet(): Promise<ThemeDocument>;

    /** PUT `/api/v1/ui/theme`. Replaces both halves of the document
     * atomically and returns the canonical post-write state. */
    themeSave(request: ThemeSaveRequest): Promise<ThemeDocument>;

    /** POST `/api/v1/ui/theme/logo` with the file bytes as the body
     * and the file's MIME type as `Content-Type`. */
    themeUploadLogo(file: File): Promise<void>;

    /** DELETE `/api/v1/ui/theme/logo`. */
    themeDeleteLogo(): Promise<void>;

    /** POST `/api/v1/ui/theme/favicon`. Same shape as `themeUploadLogo`. */
    themeUploadFavicon(file: File): Promise<void>;

    /** DELETE `/api/v1/ui/theme/favicon`. */
    themeDeleteFavicon(): Promise<void>;
  }
}

const BASE = "/api/v1/ui/theme";

StarterClient.prototype.themeGet = async function themeGet(this: StarterClient): Promise<ThemeDocument> {
  const res = await this.fetch(`${this.baseUrl}${BASE}`, {
    credentials: "include",
    headers: this.headers,
  });
  if (!res.ok) throw await StarterError.fromResponse(res);
  return (await res.json()) as ThemeDocument;
};

StarterClient.prototype.themeSave = async function themeSave(
  this: StarterClient,
  request: ThemeSaveRequest,
): Promise<ThemeDocument> {
  const res = await this.fetch(`${this.baseUrl}${BASE}`, {
    method: "PUT",
    credentials: "include",
    headers: { ...this.headers, "content-type": "application/json" },
    body: JSON.stringify(request),
  });
  if (!res.ok) throw await StarterError.fromResponse(res);
  // The server may return either the updated document or 204 — handle
  // both so we don't blow up on `res.json()` against an empty body.
  if (res.status === 204) return (await this.themeGet());
  return (await res.json()) as ThemeDocument;
};

StarterClient.prototype.themeUploadLogo = async function themeUploadLogo(
  this: StarterClient,
  file: File,
): Promise<void> {
  await uploadAsset(this, "logo", file);
};

StarterClient.prototype.themeDeleteLogo = async function themeDeleteLogo(this: StarterClient): Promise<void> {
  await deleteAsset(this, "logo");
};

StarterClient.prototype.themeUploadFavicon = async function themeUploadFavicon(
  this: StarterClient,
  file: File,
): Promise<void> {
  await uploadAsset(this, "favicon", file);
};

StarterClient.prototype.themeDeleteFavicon = async function themeDeleteFavicon(this: StarterClient): Promise<void> {
  await deleteAsset(this, "favicon");
};

async function uploadAsset(client: StarterClient, kind: "logo" | "favicon", file: File): Promise<void> {
  const res = await client.fetch(`${client.baseUrl}${BASE}/${kind}`, {
    method: "POST",
    credentials: "include",
    headers: { ...client.headers, "content-type": file.type || "application/octet-stream" },
    body: file,
  });
  if (!res.ok) throw await StarterError.fromResponse(res);
}

async function deleteAsset(client: StarterClient, kind: "logo" | "favicon"): Promise<void> {
  const res = await client.fetch(`${client.baseUrl}${BASE}/${kind}`, {
    method: "DELETE",
    credentials: "include",
    headers: client.headers,
  });
  if (!res.ok) throw await StarterError.fromResponse(res);
}
