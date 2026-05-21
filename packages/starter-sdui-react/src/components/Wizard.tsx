/**
 * `wizard` — multi-step form. Each step has its own child tree;
 * the final step's submit fires the `submit` action.
 *
 * Per R9, step state is local UI state only — the *data* the
 * wizard collects flows through the same form coordinator the
 * inner controls use; the wizard itself owns only the step index.
 *
 * `drawer` — off-canvas slide-over panel. Bound to a `$page` key
 * (default `drawer_<id>`); the close gesture writes `false` back.
 */
import { useState } from "react";
import { Button, Sheet, SheetContent, SheetHeader, SheetTitle } from "@nube/starter-ui-kit";
import type { ComponentSpec } from "../registry/types.js";
import type { UiComponent } from "../types.js";
import { useSdui } from "../context.js";
import { RendererList } from "../Renderer.js";

export interface WizardStep {
  id?: string;
  title?: string;
  children: UiComponent[];
}
export interface WizardNode extends UiComponent {
  type: "wizard";
  steps?: WizardStep[];
  submit?: { handler: string; args?: Record<string, unknown> };
}

export const wizardSpec: ComponentSpec<WizardNode> = {
  kind: "wizard" as never,
  Component: ({ node }) => {
    const { dispatchAction } = useSdui();
    const steps = node.steps ?? [];
    const [idx, setIdx] = useState(0);
    if (steps.length === 0) return null;
    const safeIdx = Math.min(idx, steps.length - 1);
    const step = steps[safeIdx]!;
    const isLast = safeIdx === steps.length - 1;
    return (
      <div className={`flex flex-col gap-3 ${node.style?.className ?? ""}`}>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          {steps.map((s, i) => (
            <span key={s.id ?? i} className={i === safeIdx ? "font-medium text-foreground" : ""}>
              {i + 1}. {s.title ?? `step ${i + 1}`}
            </span>
          ))}
        </div>
        <div>
          <RendererList nodes={step.children ?? []} />
        </div>
        <div className="flex items-center justify-end gap-2">
          <Button
            variant="outline"
            disabled={safeIdx === 0}
            onClick={() => setIdx((i) => Math.max(0, i - 1))}
          >
            back
          </Button>
          {isLast ? (
            <Button
              onClick={() => {
                if (node.submit) {
                  void dispatchAction(node.submit.handler, node.submit.args);
                }
              }}
            >
              submit
            </Button>
          ) : (
            <Button onClick={() => setIdx((i) => Math.min(steps.length - 1, i + 1))}>
              next
            </Button>
          )}
        </div>
      </div>
    );
  },
};

export interface DrawerNode extends UiComponent {
  type: "drawer";
  title?: string;
  open?: boolean;
  page_state_key?: string;
  children?: UiComponent[];
}

export const drawerSpec: ComponentSpec<DrawerNode> = {
  kind: "drawer" as never,
  Component: ({ node }) => {
    const { pageState, setPageState } = useSdui();
    const key = node.page_state_key ?? `drawer_${node.id ?? "default"}`;
    const open = Boolean(pageState[key] ?? node.open ?? false);
    const setOpen = (v: boolean) => setPageState({ [key]: v });
    return (
      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent>
          {node.title ? (
            <SheetHeader>
              <SheetTitle>{node.title}</SheetTitle>
            </SheetHeader>
          ) : null}
          <div className="mt-3">
            <RendererList nodes={node.children ?? []} />
          </div>
        </SheetContent>
      </Sheet>
    );
  },
};
