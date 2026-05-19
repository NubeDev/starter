// Pluggable persistence seam. The editor is transport-agnostic: it
// loads via `transport.load()`, saves via `transport.save()`, and
// uploads/deletes logos and favicons via the asset hooks. Three impls
// ship here:
//
// - `httpThemeTransport({ client })` — talks to a starter-server
//   instance over the OpenAPI surface via `@nube/starter-client-ts`'s
//   `ThemeApi`. This is the default.
// - `localStorageThemeTransport({ key })` — single-tenant fallback for
//   demos and consumers without a backend yet. Stores the full
//   `ThemeDocument` JSON under one key.
// - `inMemoryThemeTransport()` — for tests; resets on reload.
//
// A consumer who wants a non-HTTP backend (gRPC, IPC, fleet
// orchestration) implements `ThemeTransport` directly.

import type { StarterClient } from "@nube/starter-client-ts";

import type { ShellConfig, ThemeDocument, ThemeStyles } from "./types.js";

/** The full contract the editor needs from its backend. */
export interface ThemeTransport {
  /** Fetch the current document. Implementations MUST return a usable
   * document even for "not yet configured" tenants — typically by
   * filling `theme_styles` with empty maps and `shell` with sensible
   * defaults. */
  load(): Promise<ThemeDocument>;

  /** Persist token map + shell config in one atomic step. The asset
   * URLs in the returned document reflect what the server now stores
   * (the editor uses them to update its preview after upload). */
  save(input: { theme_styles: ThemeStyles; shell: ShellConfig }): Promise<ThemeDocument>;

  /** Replace the stored logo. `null` clears it. */
  setLogo(file: File | null): Promise<void>;

  /** Replace the stored favicon. `null` clears it. */
  setFavicon(file: File | null): Promise<void>;
}

/** HTTP transport over the starter-server REST surface.
 *
 * The wire types from `@nube/starter-client-ts` are generated from
 * OpenAPI and mark every field as optional (utoipa default). The
 * editor's local `ThemeDocument` shape is stricter — both mode maps
 * are required (the editor always renders both halves), and `shell`
 * fields are required (the editor controls them). We normalise here
 * so the rest of the editor never sees an undefined `light` /
 * `dark` / `nav_title`. */
export function httpThemeTransport(opts: { client: StarterClient }): ThemeTransport {
  const { client } = opts;
  const normalise = (raw: Awaited<ReturnType<StarterClient["themeGet"]>>): ThemeDocument => ({
    theme_styles: {
      light: raw.theme_styles.light ?? {},
      dark: raw.theme_styles.dark ?? {},
    },
    shell: {
      nav_title: raw.shell.nav_title ?? "",
      hide_features: raw.shell.hide_features ?? [],
    },
    logo_url: raw.logo_url ?? null,
    favicon_url: raw.favicon_url ?? null,
  });
  return {
    async load() {
      return normalise(await client.themeGet());
    },
    async save(input) {
      return normalise(await client.themeSave(input));
    },
    async setLogo(file) {
      if (file == null) {
        await client.themeDeleteLogo();
      } else {
        await client.themeUploadLogo(file);
      }
    },
    async setFavicon(file) {
      if (file == null) {
        await client.themeDeleteFavicon();
      } else {
        await client.themeUploadFavicon(file);
      }
    },
  };
}

/** `localStorage`-backed transport. Asset hooks are no-ops because
 * `localStorage` can't store binary; the editor's preview still shows
 * pending uploads, they just don't persist across reloads. */
export function localStorageThemeTransport(opts: { key?: string } = {}): ThemeTransport {
  const key = opts.key ?? "starter:theme-document";
  return {
    async load() {
      if (typeof window === "undefined") return emptyDocument();
      const raw = window.localStorage.getItem(key);
      if (!raw) return emptyDocument();
      try {
        return JSON.parse(raw) as ThemeDocument;
      } catch {
        return emptyDocument();
      }
    },
    async save(input) {
      const doc: ThemeDocument = { ...input, logo_url: null, favicon_url: null };
      window.localStorage.setItem(key, JSON.stringify(doc));
      return doc;
    },
    async setLogo() {
      // No-op — see comment on the factory.
    },
    async setFavicon() {
      // No-op — see comment on the factory.
    },
  };
}

/** Volatile transport for tests. */
export function inMemoryThemeTransport(initial?: Partial<ThemeDocument>): ThemeTransport {
  let doc: ThemeDocument = { ...emptyDocument(), ...initial };
  return {
    async load() {
      return doc;
    },
    async save(input) {
      doc = { ...doc, ...input };
      return doc;
    },
    async setLogo(file) {
      doc = { ...doc, logo_url: file ? `mem://logo/${file.name}` : null };
    },
    async setFavicon(file) {
      doc = { ...doc, favicon_url: file ? `mem://favicon/${file.name}` : null };
    },
  };
}

function emptyDocument(): ThemeDocument {
  return {
    theme_styles: { light: {}, dark: {} },
    shell: { nav_title: "", hide_features: [] },
    logo_url: null,
    favicon_url: null,
  };
}
