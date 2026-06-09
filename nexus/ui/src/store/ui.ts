import { create } from "zustand";

// Ephemeral client UI state — edit mode and canvas selection. Server
// state (dashboards, panels, query results) lives in TanStack Query, not
// here (F0: this store holds no fabricated data). `zustand` is one of the
// federation singletons the host registers, so importing `create` from
// the workspace's single `zustand` keeps host and remotes on one store
// runtime.
interface UiState {
  editMode: boolean;
  selectedWidgetId: string | null;
  setEditMode: (on: boolean) => void;
  toggleEditMode: () => void;
  selectWidget: (id: string | null) => void;
}

export const useUiStore = create<UiState>((set) => ({
  editMode: false,
  selectedWidgetId: null,
  setEditMode: (on) =>
    // Leaving edit mode drops any selection — a selected widget only
    // means something while the canvas is editable.
    set(on ? { editMode: true } : { editMode: false, selectedWidgetId: null }),
  toggleEditMode: () =>
    set((s) =>
      s.editMode
        ? { editMode: false, selectedWidgetId: null }
        : { editMode: true },
    ),
  selectWidget: (id) => set({ selectedWidgetId: id }),
}));
