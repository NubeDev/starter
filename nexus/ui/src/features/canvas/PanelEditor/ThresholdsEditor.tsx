import { Plus, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";

import type { ThresholdStep } from "@/data/types";
import { hexToHsl, hslToHex } from "@/features/canvas/PanelEditor/hslHex";

// Editor for a multi-step threshold ramp. The base step (`value: null`)
// is always first and has no editable lower bound; further steps carry a
// numeric lower bound and a colour (an hsl triple matching the project's
// `SeriesField.color` convention). Steps stay sorted ascending so the
// renderer (`rampColor`) reads them predictably. Presentational: state is
// owned by the Field tab.
const DEFAULT_COLORS = ["152 76% 44%", "38 92% 50%", "0 84% 60%"];

export function ThresholdsEditor({
  steps,
  onChange,
}: {
  steps: ReadonlyArray<ThresholdStep>;
  onChange: (steps: ThresholdStep[]) => void;
}) {
  // Ensure a base step exists so colouring has a floor; this is display
  // scaffolding, not persisted unless the user keeps a non-empty ramp.
  const list: ThresholdStep[] =
    steps.length === 0 ? [{ value: null, color: DEFAULT_COLORS[0] }] : [...steps];

  function update(i: number, patch: Partial<ThresholdStep>) {
    const next = list.map((s, j) => (j === i ? { ...s, ...patch } : s));
    onChange(sortSteps(next));
  }

  function add() {
    const max = Math.max(0, ...list.map((s) => s.value ?? 0));
    const color = DEFAULT_COLORS[list.length % DEFAULT_COLORS.length];
    onChange(sortSteps([...list, { value: max + 10, color }]));
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <Label>Thresholds</Label>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 gap-1 px-2 text-xs"
          onClick={add}
        >
          <Plus className="size-3.5" /> Add step
        </Button>
      </div>
      {list.map((step, i) => (
        <div key={i} className="flex items-center gap-2">
          <input
            type="color"
            aria-label={`Step ${i + 1} colour`}
            value={hslToHex(step.color)}
            onChange={(e) => update(i, { color: hexToHsl(e.target.value) })}
            className="h-8 w-8 shrink-0 cursor-pointer rounded border border-border bg-transparent"
          />
          {step.value == null ? (
            <span className="flex-1 text-xs text-muted-foreground">Base (everything below)</span>
          ) : (
            <Input
              type="number"
              aria-label={`Step ${i + 1} value`}
              value={step.value}
              onChange={(e) => update(i, { value: Number(e.target.value) })}
              className="flex-1"
            />
          )}
          {step.value != null ? (
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label={`Remove step ${i + 1}`}
              className="size-8 text-muted-foreground hover:text-destructive"
              onClick={() => onChange(list.filter((_, j) => j !== i))}
            >
              <Trash2 className="size-4" />
            </Button>
          ) : (
            <span className="w-8" />
          )}
        </div>
      ))}
    </div>
  );
}

function sortSteps(steps: ThresholdStep[]): ThresholdStep[] {
  return [...steps].sort((a, b) => (a.value ?? -Infinity) - (b.value ?? -Infinity));
}
