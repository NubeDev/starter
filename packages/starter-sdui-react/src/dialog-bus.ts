/**
 * Dialog-stack bus — module-level LIFO stack for dynamic dialogs
 * opened via action responses (`{ type: "dialog", tree: ... }`).
 *
 * Module-level so the bus survives React subtree unmounts; the host
 * (rendered by `SduiPage`) subscribes once and cleans up on unmount.
 */
import type { UiComponent } from "./types.js";

type StackListener = (stack: UiComponent[]) => void;

const listeners = new Set<StackListener>();
let dialogStack: UiComponent[] = [];

export function pushDialog(tree: UiComponent): void {
  dialogStack = [...dialogStack, tree];
  notify();
}

export function popDialog(): void {
  if (dialogStack.length === 0) return;
  dialogStack = dialogStack.slice(0, -1);
  notify();
}

export function subscribeDialogStack(fn: StackListener): () => void {
  listeners.add(fn);
  fn(dialogStack);
  return () => {
    listeners.delete(fn);
  };
}

/** Current depth — `useActionResponse` reads this to auto-dismiss
 *  the top dialog after a successful action without forcing the
 *  caller to subscribe. */
export function dialogStackSize(): number {
  return dialogStack.length;
}

function notify(): void {
  for (const fn of listeners) fn(dialogStack);
}
