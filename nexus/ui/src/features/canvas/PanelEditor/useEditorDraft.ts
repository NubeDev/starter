import { useCallback, useState } from "react";

import type { Widget, WidgetConfig } from "@/data/types";

// Local edit state for the panel editor: a working copy of the widget the
// user mutates tab-by-tab, plus narrow setters so each tab edits its slice
// without re-implementing immutable updates. The draft lives entirely in
// the editor (the canvas keeps showing the saved panel) and is only
// persisted on Save. Seeded once from the opened widget.
export interface EditorDraft {
  widget: Widget;
  /** Replace the whole widget (type switch carries title/query along). */
  setWidget: (next: Widget) => void;
  /** Patch top-level widget fields (title, type). */
  patch: (fields: Partial<Pick<Widget, "title" | "type" | "subtitle">>) => void;
  /** Patch the panel config (query, fields, fieldConfig, transforms). */
  patchConfig: (fields: Partial<WidgetConfig>) => void;
}

export function useEditorDraft(initial: Widget): EditorDraft {
  const [widget, setWidget] = useState<Widget>(initial);

  const patch = useCallback(
    (fields: Partial<Pick<Widget, "title" | "type" | "subtitle">>) =>
      setWidget((w) => ({ ...w, ...fields })),
    [],
  );

  const patchConfig = useCallback(
    (fields: Partial<WidgetConfig>) =>
      setWidget((w) => ({ ...w, config: { ...w.config, ...fields } })),
    [],
  );

  return { widget, setWidget, patch, patchConfig };
}
