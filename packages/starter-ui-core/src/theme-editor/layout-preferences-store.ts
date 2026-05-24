// Zustand store for `LayoutPreferences`. Kept in its own module so the
// existing `useThemeEditorStore` (which carries undo/redo + dirty
// tracking for the 38-token model) stays narrow.
//
// Persistence is handled by `zustand/middleware/persist` against
// `localStorage` by default; tests can pass a `MockStorage` via the
// `storage` option of `createLayoutPreferencesStore`.

import { create, type Mutate, type StoreApi, type UseBoundStore } from "zustand";
import { createJSONStorage, persist, type PersistStorage } from "zustand/middleware";

import {
  defaultLayoutPreferences,
  type Density,
  type FontSize,
  type LayoutPreferences,
  type ModePreference,
  type Motion,
  type PaletteId,
} from "./layout-preferences.js";

export interface LayoutPreferencesState extends LayoutPreferences {
  setMode: (m: ModePreference) => void;
  setDensity: (d: Density) => void;
  setFontSize: (f: FontSize) => void;
  setMotion: (m: Motion) => void;
  setPalette: (p: PaletteId | null) => void;
  /** Replace the whole preferences object atomically. Useful for
   * "reset" and for hydrating from a server payload. */
  hydrate: (next: LayoutPreferences) => void;
}

export interface CreateLayoutPreferencesStoreOptions {
  /** localStorage key. Defaults to `"starter:layout-preferences"`. */
  storageKey?: string;
  /** Custom storage adapter (tests pass an in-memory one). */
  storage?: PersistStorage<LayoutPreferencesState> | null;
  /** Override the seed values. */
  initial?: Partial<LayoutPreferences>;
}

/** A `LayoutPreferences` store with `persist` middleware attached.
 * Exposed as a return type so consumers (and tests) can reach
 * `.persist.clearStorage()` etc. without losing type-safety. */
export type LayoutPreferencesStore = UseBoundStore<
  Mutate<StoreApi<LayoutPreferencesState>, [["zustand/persist", LayoutPreferencesState]]>
>;

/** Build a configured `useLayoutPreferences` hook. Use the default
 * `useLayoutPreferences` export for the common case; call this factory
 * if a consumer needs a non-default storage key or a fresh instance
 * (multi-tenant testbeds). */
export function createLayoutPreferencesStore(
  options: CreateLayoutPreferencesStoreOptions = {},
): LayoutPreferencesStore {
  const {
    storageKey = "starter:layout-preferences",
    storage,
    initial,
  } = options;

  const seed: LayoutPreferences = { ...defaultLayoutPreferences, ...initial };

  return create<LayoutPreferencesState>()(
    persist(
      (set) => ({
        ...seed,
        setMode: (mode) => set({ mode }),
        setDensity: (density) => set({ density }),
        setFontSize: (fontSize) => set({ fontSize }),
        setMotion: (motion) => set({ motion }),
        setPalette: (palette) => set({ palette }),
        hydrate: (next) => set({ ...next }),
      }),
      {
        name: storageKey,
        storage:
          storage === undefined
            ? (createJSONStorage(() => localStorage) as PersistStorage<LayoutPreferencesState>)
            : (storage ?? undefined),
      },
    ),
  );
}

/** Shared singleton — the common case. Consumers that want their own
 * isolated store can call `createLayoutPreferencesStore` directly. */
export const useLayoutPreferences = createLayoutPreferencesStore();
