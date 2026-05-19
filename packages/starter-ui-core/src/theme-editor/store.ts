// Editor state machine. Held in Zustand because: (a) the existing
// `@nube/starter-ui-core` already ships `zustand` as a dep, (b) the
// undo/redo ring needs cheap shallow snapshots, (c) the editor's
// React tree is deep enough that prop-drilling token state is painful.
//
// Pure state — no DOM mutation, no I/O. The editor components observe
// `styles` + `mode` and re-stamp the preview element via
// `applyThemeToElement`; the `useThemeEditor` hook owns the network
// round-trip via the injected `ThemeTransport`.

import { create } from "zustand";

import { defaultThemeStyles } from "./defaults.js";
import type {
  ShellConfig,
  ThemeMode,
  ThemeStyleKey,
  ThemeStyleProps,
  ThemeStyles,
} from "./types.js";

/** Ring-buffer cap. Matches the original tweakcn editor (30 entries
 * = enough to recover from a long preset-tweaking session, small
 * enough that the snapshots are free). */
const HISTORY_LIMIT = 30;

/** Snapshots taken within this many milliseconds of the previous one
 * are collapsed (slider drags would otherwise blow the buffer in one
 * gesture). */
const COLLAPSE_MS = 500;

interface Snapshot {
  styles: ThemeStyles;
  shell: ShellConfig;
  takenAt: number;
}

export interface ThemeEditorState {
  /** Currently-edited token map (both modes). */
  styles: ThemeStyles;
  /** Which mode the preview shows / the editor's "active" half. */
  mode: ThemeMode;
  /** Branding sidecar. */
  shell: ShellConfig;
  /** Pending logo file: `File` = upload on save, `null` = no change,
   * `undefined` = delete on save. */
  pendingLogo: File | null | undefined;
  /** Pending favicon file. Same tri-state shape as `pendingLogo`. */
  pendingFavicon: File | null | undefined;
  /** Set when the in-memory state diverges from the last loaded /
   * saved snapshot. The page tab shows a dirty dot when true. */
  isDirty: boolean;

  // Token editing.
  setToken: (mode: ThemeMode, key: ThemeStyleKey, value: string) => void;
  applyPresetStyles: (styles: ThemeStyles) => void;
  setMode: (mode: ThemeMode) => void;
  reset: () => void;

  // Shell editing.
  setShellField: <K extends keyof ShellConfig>(key: K, value: ShellConfig[K]) => void;
  setPendingLogo: (file: File | null | undefined) => void;
  setPendingFavicon: (file: File | null | undefined) => void;

  // Lifecycle (driven by `useThemeEditor`).
  hydrate: (styles: ThemeStyles, shell: ShellConfig) => void;
  markSaved: () => void;

  // History.
  checkpoint: () => void;
  undo: () => void;
  redo: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;
}

const defaultShell: ShellConfig = { nav_title: "", hide_features: [] };

/** Module-private history rings. Kept outside the Zustand store so
 * undo/redo doesn't trigger render storms on every snapshot. */
let past: Snapshot[] = [];
let future: Snapshot[] = [];

function snapshot(state: ThemeEditorState): Snapshot {
  return {
    styles: cloneStyles(state.styles),
    shell: cloneShell(state.shell),
    takenAt: Date.now(),
  };
}

function cloneStyles(s: ThemeStyles): ThemeStyles {
  return { light: { ...s.light }, dark: { ...s.dark } };
}

function cloneShell(s: ShellConfig): ShellConfig {
  return { nav_title: s.nav_title, hide_features: [...s.hide_features] };
}

export const useThemeEditorStore = create<ThemeEditorState>((set, get) => ({
  styles: cloneStyles(defaultThemeStyles),
  mode: "light",
  shell: cloneShell(defaultShell),
  pendingLogo: null,
  pendingFavicon: null,
  isDirty: false,

  setToken(mode, key, value) {
    set((state) => ({
      styles: {
        ...state.styles,
        [mode]: { ...state.styles[mode], [key]: value } as ThemeStyleProps,
      },
      isDirty: true,
    }));
  },

  applyPresetStyles(styles) {
    get().checkpoint();
    set({ styles: cloneStyles(styles), isDirty: true });
  },

  setMode(mode) {
    set({ mode });
  },

  reset() {
    past = [];
    future = [];
    set({
      styles: cloneStyles(defaultThemeStyles),
      shell: cloneShell(defaultShell),
      pendingLogo: null,
      pendingFavicon: null,
      isDirty: true,
    });
  },

  setShellField(key, value) {
    set((state) => ({
      shell: { ...state.shell, [key]: value },
      isDirty: true,
    }));
  },

  setPendingLogo(file) {
    set({ pendingLogo: file, isDirty: true });
  },

  setPendingFavicon(file) {
    set({ pendingFavicon: file, isDirty: true });
  },

  hydrate(styles, shell) {
    past = [];
    future = [];
    set({
      styles: cloneStyles(styles),
      shell: cloneShell(shell),
      pendingLogo: null,
      pendingFavicon: null,
      isDirty: false,
    });
  },

  markSaved() {
    set({ pendingLogo: null, pendingFavicon: null, isDirty: false });
  },

  checkpoint() {
    const state = get();
    const top = past[past.length - 1];
    const now = Date.now();
    // Collapse rapid edits: if the previous snapshot is recent and
    // identical, don't push a duplicate.
    if (top && now - top.takenAt < COLLAPSE_MS) {
      // Update timestamp so a sustained drag keeps the collapse going.
      top.takenAt = now;
      return;
    }
    past.push(snapshot(state));
    if (past.length > HISTORY_LIMIT) past.shift();
    future = [];
  },

  undo() {
    const prev = past.pop();
    if (!prev) return;
    future.push(snapshot(get()));
    set({
      styles: prev.styles,
      shell: prev.shell,
      isDirty: true,
    });
  },

  redo() {
    const next = future.pop();
    if (!next) return;
    past.push(snapshot(get()));
    set({
      styles: next.styles,
      shell: next.shell,
      isDirty: true,
    });
  },

  canUndo() {
    return past.length > 0;
  },

  canRedo() {
    return future.length > 0;
  },
}));
