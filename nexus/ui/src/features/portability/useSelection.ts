import { useCallback, useState } from "react";

import type { DashboardExport } from "@/api/types";
import {
  selectAll,
  selectNone,
  type PortableSelection,
} from "@/features/portability/model";

// Selection state + toggles shared by the export and import pages. Seeds to
// "everything selected" once a model is available (the user deselects what they
// don't want), and exposes per-item toggles plus all/none bulk actions.
export function useSelection(model: DashboardExport | undefined) {
  const [selection, setSelection] = useState<PortableSelection>(() =>
    model ? selectAll(model) : selectNone(),
  );
  // Whether we've seeded from a model yet — so a late-arriving model (the export
  // fetch resolves after mount) initialises the selection exactly once.
  const [seeded, setSeeded] = useState(model !== undefined);
  if (model && !seeded) {
    setSelection(selectAll(model));
    setSeeded(true);
  }

  const togglePanel = useCallback((index: number) => {
    setSelection((prev) => {
      const panelIndices = new Set(prev.panelIndices);
      if (panelIndices.has(index)) panelIndices.delete(index);
      else panelIndices.add(index);
      return { ...prev, panelIndices };
    });
  }, []);

  const toggleVariable = useCallback((name: string) => {
    setSelection((prev) => {
      const variableNames = new Set(prev.variableNames);
      if (variableNames.has(name)) variableNames.delete(name);
      else variableNames.add(name);
      return { ...prev, variableNames };
    });
  }, []);

  const all = useCallback(() => {
    if (model) setSelection(selectAll(model));
  }, [model]);

  const none = useCallback(() => setSelection(selectNone()), []);

  return { selection, togglePanel, toggleVariable, all, none };
}
