// `useHostPrefs()` — read the host's resolved preferences from the
// `@nube/starter-ui-core/preferences` singleton.
//
// Why a dedicated hook (rather than asking extension authors to
// reach into `handle.singletons`):
//
// - One stable surface. The singleton id and the Context shape are
//   internal contracts; the hook is what the SCOPE froze on merge
//   (DOCS/user/scope/SCOPE.md R9; examples/notes/user-pref.md §
//   Stage 3).
// - Loud-fail wiring. If the extension was mounted outside the
//   host's prefs subtree (no singleton provided, or prefs still
//   loading), the hook throws with a documented message so the
//   mistake surfaces at render time, not as `undefined` percolating
//   through formatters.
// - Free composition with `useHostFormatters` — that hook pipes the
//   prefs object straight into the bound formatters so the
//   extension never threads `prefs` through call sites.

import * as React from "react";

import { useHostBindings } from "./host-bindings.js";
import { SINGLETON_UI_CORE_PREFERENCES } from "./singleton-keys.js";
import type { HostPreferencesContextValue, ResolvedPreferences } from "./prefs-types.js";

/**
 * Return the host's resolved preferences. Throws when:
 *
 * - The panel is not mounted by the host's federation runtime (no
 *   `<HostBindingsProvider>` in the tree).
 * - The extension's `block.yaml` did not declare
 *   `@nube/starter-ui-core/preferences` as a required singleton, so
 *   the host did not pass it to the `init` factory.
 * - The host's `<PreferencesProvider>` has not resolved yet (its
 *   `fallback` should still be rendering; the panel was somehow
 *   mounted past the loading gate).
 *
 * The thrown messages name the failure mode so an operator reading
 * the host's error boundary sees what to fix.
 */
export function useHostPrefs(): ResolvedPreferences {
  const { singletons } = useHostBindings();
  const PrefsContext = singletons[SINGLETON_UI_CORE_PREFERENCES] as
    | React.Context<HostPreferencesContextValue | undefined>
    | undefined;
  if (!PrefsContext) {
    throw new Error(
      "useHostPrefs(): host did not provide the " +
        "@nube/starter-ui-core/preferences singleton. " +
        "Declare it under `singletons` in your remoteEntry factory " +
        "and add it to `block.yaml`'s `requires`.",
    );
  }
  const value = React.useContext(PrefsContext);
  if (!value) {
    throw new Error(
      "useHostPrefs(): host has not mounted <PreferencesProvider>. " +
        "The notes host's app.tsx must wrap extensions in PreferencesProvider " +
        "for the singleton context to resolve.",
    );
  }
  if (!value.preferences) {
    throw new Error(
      "useHostPrefs(): preferences not resolved yet. The host's " +
        "PreferencesProvider should hold back rendering until its " +
        "loading-resolved subtree mounts — see Stage 1 loading contract.",
    );
  }
  return value.preferences;
}
