import { Check } from "lucide-react";

import type { VariableExport } from "@/api/types";
import type { PortableSelection } from "@/features/portability/model";

// A checkbox list of a dashboard's variables for inclusion in an export/import.
// Each row shows the `$name`, its human label, and its kind so the user can tell
// a query-variable from a constant at a glance. Deselected rows dim but stay
// listed (parity with the panel placement preview).
export function VariableSelectList({
  variables,
  selection,
  onToggle,
}: {
  variables: ReadonlyArray<VariableExport>;
  selection: PortableSelection;
  onToggle: (name: string) => void;
}) {
  if (variables.length === 0) {
    return (
      <p className="rounded-lg border border-dashed border-border p-3 text-sm text-muted-foreground">
        This dashboard has no variables.
      </p>
    );
  }

  return (
    <ul className="flex flex-col gap-1.5">
      {variables.map((variable) => {
        const selected = selection.variableNames.has(variable.name);
        return (
          <li key={variable.name}>
            <button
              type="button"
              onClick={() => onToggle(variable.name)}
              className={[
                "flex w-full items-center gap-3 rounded-lg border p-2.5 text-left transition",
                selected
                  ? "border-primary/40 bg-card"
                  : "border-border bg-card/40 opacity-60 hover:opacity-90",
              ].join(" ")}
            >
              <span
                className={[
                  "flex size-4 shrink-0 items-center justify-center rounded border",
                  selected
                    ? "border-primary bg-primary text-primary-foreground"
                    : "border-border bg-background",
                ].join(" ")}
                aria-hidden
              >
                {selected ? <Check className="size-3" /> : null}
              </span>
              <span className="min-w-0 flex-1">
                <span className="flex items-baseline gap-2">
                  <code className="font-mono text-sm text-foreground">
                    ${variable.name}
                  </code>
                  {variable.label ? (
                    <span className="truncate text-xs text-muted-foreground">
                      {variable.label}
                    </span>
                  ) : null}
                </span>
              </span>
              <span className="shrink-0 rounded-full bg-muted px-2 py-0.5 text-[10px] uppercase tracking-wide text-muted-foreground">
                {variable.kind}
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
