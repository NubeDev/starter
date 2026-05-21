/**
 * `SduiDialogHost` — the dialog-bus subscriber. Mounted once by
 * `SduiPage` / `SduiRenderPage`; renders the top dialog tree
 * (LIFO) inside a shadcn `Dialog` and closes the topmost dialog
 * when the user dismisses.
 *
 * The dialog tree is itself a `UiComponent` — the bus carries IR,
 * not React — so each dialog dispatches through the same renderer,
 * with the same action protocol, as the page.
 */
import { useEffect, useState } from "react";
import { Dialog, DialogContent } from "@nube/starter-ui-kit";
import { popDialog, subscribeDialogStack } from "./dialog-bus.js";
import { Renderer } from "./Renderer.js";
import type { UiComponent } from "./types.js";

export function SduiDialogHost() {
  const [stack, setStack] = useState<UiComponent[]>([]);

  useEffect(() => subscribeDialogStack(setStack), []);

  const top = stack[stack.length - 1];
  if (!top) return null;

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) popDialog();
      }}
    >
      <DialogContent>
        <Renderer node={top} />
      </DialogContent>
    </Dialog>
  );
}
