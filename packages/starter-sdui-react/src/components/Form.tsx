/**
 * `form` — collects child `field` / `select` / `toggle` values into
 * an `args` object keyed by each child's `name` and dispatches a
 * single action on submit.
 *
 * Diagnostics flow (divergence **D1**): the server returns
 * `{ type: "diagnostics", items: [{ severity, code, message, field? }] }`
 * on validation failure. Field-scoped items render inline next to
 * their control; orphan items render in a banner above the form.
 *
 * The form holds its own values in a `useReducer`-shaped state so
 * children don't have to plumb refs; the per-control state path is
 * `pageState[`${formId}.${fieldName}`]` if a control wants
 * cross-form binding (`bind: "page_state"`).
 */
import { useState } from "react";
import { Button } from "@nube/starter-ui-kit";
import type { ComponentSpec } from "../registry/types.js";
import { RendererList } from "../Renderer.js";
import { FormContext } from "./form-context.js";
import { useSdui } from "../context.js";
import { useActionResponse } from "../useActionResponse.js";
import type { UiComponent, Diagnostic, UiActionResponse } from "../types.js";

export interface FormNode extends UiComponent {
  type: "form";
  children: UiComponent[];
  /** Action handler invoked with `{ ...values }` on submit. */
  submit?: string;
  submit_label?: string;
  cancel_label?: string;
  on_cancel?: string;
}

export const formSpec: ComponentSpec<FormNode> = {
  kind: "form",
  Component: ({ node }) => {
    const { dispatchAction } = useSdui();
    const interpret = useActionResponse();
    const [values, setValues] = useState<Record<string, unknown>>({});
    const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([]);
    const [pending, setPending] = useState(false);

    const setField = (name: string, value: unknown) => {
      setValues((prev) => ({ ...prev, [name]: value }));
      // Clear field-scoped diagnostics when the field changes.
      setDiagnostics((prev) => prev.filter((d) => d.field !== name));
    };

    const onSubmit = async (e: React.FormEvent) => {
      e.preventDefault();
      if (!node.submit) return;
      setPending(true);
      setDiagnostics([]);
      try {
        const res: UiActionResponse = await dispatchAction(node.submit, values);
        if (res.type === "diagnostics") {
          setDiagnostics(res.items);
          return;
        }
        interpret(res);
      } finally {
        setPending(false);
      }
    };

    const orphans = diagnostics.filter((d) => !d.field);

    return (
      <FormContext.Provider value={{ values, setField, diagnostics }}>
        <form onSubmit={onSubmit} className="flex flex-col gap-4">
          {orphans.length > 0 ? (
            <div className="flex flex-col gap-1 rounded-md border border-destructive/40 bg-destructive/5 p-3 text-sm">
              {orphans.map((d, i) => (
                <div
                  key={i}
                  className={
                    d.severity === "error"
                      ? "text-destructive"
                      : d.severity === "warning"
                      ? "text-amber-600"
                      : "text-muted-foreground"
                  }
                >
                  {d.message}
                </div>
              ))}
            </div>
          ) : null}
          <RendererList nodes={node.children ?? []} parentId={node.id} parentType="form" />
          <div className="mt-2 flex justify-end gap-2">
            {node.on_cancel ? (
              <Button
                type="button"
                variant="outline"
                disabled={pending}
                onClick={() => void dispatchAction(node.on_cancel!).then(interpret)}
              >
                {node.cancel_label ?? "Cancel"}
              </Button>
            ) : null}
            {node.submit ? (
              <Button type="submit" disabled={pending}>
                {node.submit_label ?? "Submit"}
              </Button>
            ) : null}
          </div>
        </form>
      </FormContext.Provider>
    );
  },
};
