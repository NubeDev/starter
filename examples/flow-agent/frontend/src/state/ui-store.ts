// Persisted UI state (theme, sidebar open/collapsed, expanded paths).
// Mirrors SIDEBAR.md §1–§3 — single localStorage key "fa-ui",
// ephemeral state stays in memory.

import { create } from "zustand";
import { persist } from "zustand/middleware";

interface UiState {
  sidebarOpen: boolean;
  setSidebarOpen(open: boolean): void;

  expandedFlowGroups: string[];
  setExpandedFlowGroups(paths: string[]): void;

  activeSection: "flows" | "agents" | "settings";
  setActiveSection(s: UiState["activeSection"]): void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      sidebarOpen: true,
      setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),

      expandedFlowGroups: [],
      setExpandedFlowGroups: (expandedFlowGroups) =>
        set({ expandedFlowGroups }),

      activeSection: "flows",
      setActiveSection: (activeSection) => set({ activeSection }),
    }),
    {
      name: "fa-ui",
      partialize: (s) => ({
        sidebarOpen: s.sidebarOpen,
        expandedFlowGroups: s.expandedFlowGroups,
        activeSection: s.activeSection,
      }),
    },
  ),
);
