// Persisted UI state (theme, sidebar open/collapsed, expanded paths).
// Mirrors SIDEBAR.md §1–§3 — single localStorage key "fa-ui",
// ephemeral state stays in memory.

import { create } from "zustand";
import { persist } from "zustand/middleware";

export type ActiveSection = "flows" | "agents" | "settings";

interface UiState {
  sidebarOpen: boolean;
  setSidebarOpen(open: boolean): void;

  /**
   * Expanded sidebar group paths (e.g. "flows", "agents", or
   * "flows/<id>" for nested entries). Persisted in localStorage so
   * the user's expand state survives a refresh.
   */
  expandedGroups: string[];
  setExpandedGroups(paths: string[]): void;
  toggleGroup(path: string): void;

  activeSection: ActiveSection;
  setActiveSection(s: ActiveSection): void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set, get) => ({
      sidebarOpen: true,
      setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),

      expandedGroups: ["flows", "agents"],
      setExpandedGroups: (expandedGroups) => set({ expandedGroups }),
      toggleGroup: (path) => {
        const cur = get().expandedGroups;
        set({
          expandedGroups: cur.includes(path)
            ? cur.filter((p) => p !== path)
            : [...cur, path],
        });
      },

      activeSection: "flows",
      setActiveSection: (activeSection) => set({ activeSection }),
    }),
    {
      name: "fa-ui",
      partialize: (s) => ({
        sidebarOpen: s.sidebarOpen,
        expandedGroups: s.expandedGroups,
        activeSection: s.activeSection,
      }),
    },
  ),
);
