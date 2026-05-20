// Dev-only Vite plugin — watch every `extensions/**/i18n/*.json`
// file and trigger an HMR full reload when one changes
// (`examples/notes/user-pref.md` Stage 7).
//
// Without this, every catalog edit is a server restart and extension
// authors will hard-code strings just to iterate faster. The plugin
// is a no-op in production builds — the dev catalog watcher only
// belongs in `vite dev`.
//
// Implementation notes:
//
//   * We resolve the extensions root relative to the plugin file
//     (`../extensions`) — the same layout the notes server uses to
//     serve the static catalogs. `configResolved` records the project
//     root so the watcher can compute pretty log paths.
//   * Vite's `server.watcher` (chokidar) already walks the project
//     tree but ignores `node_modules`. We `.add()` the extensions
//     glob explicitly so a workspace outside the project root still
//     fires events.
//   * On change/add/unlink, we send `{ type: "full-reload" }` on the
//     Vite HMR websocket. Per-extension catalog updates *could* be
//     surgical (only the changed extension's strings need to refetch),
//     but the simpler full-reload story is the right default for a
//     dev convenience plugin — the production code path that handles
//     "language flipped" already does the merge correctly, and we
//     reuse it instead of duplicating the wiring.

import path from "node:path";
import { fileURLToPath } from "node:url";
import type { Plugin } from "vite";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export interface ExtensionCatalogWatcherOptions {
  /** Override the extensions directory. Defaults to
   *  `examples/notes/extensions` relative to this file. Useful in
   *  tests; production wiring takes the default. */
  extensionsDir?: string;
}

/**
 * Dev plugin: watch every `extensions/*​/i18n/*.json` and trigger an
 * HMR full reload on change/add/unlink. Disabled outside `vite dev`.
 */
export function extensionCatalogWatcher(
  opts: ExtensionCatalogWatcherOptions = {},
): Plugin {
  const extensionsDir = opts.extensionsDir
    ? path.resolve(opts.extensionsDir)
    : path.resolve(__dirname, "../extensions");
  const catalogGlob = path.join(extensionsDir, "**/i18n/*.json");

  return {
    name: "starter:extension-catalog-watcher",
    apply: "serve", // dev only
    configureServer(server) {
      // chokidar accepts globs in `.add()`; the extension catalogs
      // already sit under the workspace root for `examples/notes`,
      // but adding the glob explicitly is defensive against future
      // layouts where catalogs live outside the project root.
      server.watcher.add(catalogGlob);
      const reload = (file: string) => {
        if (!file.includes(`${path.sep}i18n${path.sep}`)) return;
        if (!file.endsWith(".json")) return;
        // Pretty-print relative path.
        const rel = path.relative(extensionsDir, file);
        // eslint-disable-next-line no-console
        console.log(`[starter:i18n] catalog changed: ${rel} → reloading`);
        server.ws.send({ type: "full-reload" });
      };
      server.watcher.on("change", reload);
      server.watcher.on("add", reload);
      server.watcher.on("unlink", reload);
    },
  };
}
