// `form` — wraps children + a submit `Action`. Field state lives in
// `page_state` so the resolve loop reflows on input.
import type { FormEvent } from "react";
import { Button, cn } from "@nube/starter-ui-kit";
import { RenderChildren } from "../headless/render.js";
import { registerRenderer } from "../headless/registry.js";
import { useSduiAction } from "../headless/hooks/use-action.js";
import { usePageState } from "../headless/page-state.js";

export function RenderForm({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const submit = node.submit as
    | { handler: string; label?: string; args?: Record<string, unknown> }
    | undefined;
  const [state] = usePageState();
  const action = useSduiAction();
  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (!submit) return;
    action.mutate({
      handler: submit.handler,
      args: { ...(submit.args ?? {}), page_state: state },
    });
  };
  return (
    <form
      className={cn("sdui-form flex flex-col gap-3", node.style?.className)}
      onSubmit={onSubmit}
    >
      <RenderChildren nodes={node.children} />
      {submit ? (
        <div>
          <Button type="submit" disabled={action.isPending}>
            {submit.label ?? "Submit"}
          </Button>
        </div>
      ) : null}
    </form>
  );
}

registerRenderer("form", RenderForm);
