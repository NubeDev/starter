import { useEffect } from "react";

import { useRedo, useUndo } from "@/features/audit/useUndoRedo";

// Bind Cmd/Ctrl+Z (undo) and Cmd/Ctrl+Shift+Z (redo) at the document level.
// Keystrokes inside a text field are ignored so the native in-field undo still
// works — global undo is for committed domain changes, not unsaved typing. A
// pending mutation is not re-fired, so holding the keys cannot stack requests.
export function useUndoRedoShortcuts(): void {
  const undo = useUndo();
  const redo = useRedo();

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      const mod = e.metaKey || e.ctrlKey;
      if (!mod || e.key.toLowerCase() !== "z") return;
      if (isEditableTarget(e.target)) return;
      e.preventDefault();
      if (e.shiftKey) {
        if (!redo.isPending) redo.mutate();
      } else if (!undo.isPending) {
        undo.mutate();
      }
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [undo, redo]);
}

// True when the event originates in an input, textarea, or contenteditable —
// where the browser's own undo should win.
function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return (
    tag === "INPUT" ||
    tag === "TEXTAREA" ||
    target.isContentEditable
  );
}
