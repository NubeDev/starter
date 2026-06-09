import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";

import type { FieldDisplay, ThresholdStep, ValueMapping } from "@/data/types";
import type { EditorDraft } from "@/features/canvas/PanelEditor/useEditorDraft";
import { ThresholdsEditor } from "@/features/canvas/PanelEditor/ThresholdsEditor";
import { UnitPicker } from "@/features/canvas/PanelEditor/UnitPicker";
import { ValueMappingsEditor } from "@/features/canvas/PanelEditor/ValueMappingsEditor";

// Field tab: the default display for every series — unit, decimals,
// min/max, no-value text, threshold ramp, value mappings. Writes into
// `config.fieldConfig.defaults`; per-series exceptions live in Overrides.
// Each control patches one slice of the defaults so unrelated fields are
// never clobbered.
export function FieldTab({ draft }: { draft: EditorDraft }) {
  const { widget, patchConfig } = draft;
  const fieldConfig = widget.config.fieldConfig ?? {};
  const defaults: FieldDisplay = fieldConfig.defaults ?? {};

  function setDefaults(patch: Partial<FieldDisplay>) {
    patchConfig({
      fieldConfig: { ...fieldConfig, defaults: { ...defaults, ...patch } },
    });
  }

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1.5">
          <Label htmlFor="ed-unit">Unit</Label>
          <UnitPicker id="ed-unit" value={defaults.unit} onChange={(unit) => setDefaults({ unit })} />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="ed-decimals">Decimals</Label>
          <Input
            id="ed-decimals"
            type="number"
            min={0}
            max={10}
            value={defaults.decimals ?? ""}
            onChange={(e) =>
              setDefaults({ decimals: e.target.value === "" ? undefined : Number(e.target.value) })
            }
            placeholder="auto"
          />
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1.5">
          <Label htmlFor="ed-min">Min</Label>
          <Input
            id="ed-min"
            type="number"
            value={defaults.min ?? ""}
            onChange={(e) => setDefaults({ min: e.target.value === "" ? undefined : Number(e.target.value) })}
            placeholder="auto"
          />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="ed-max">Max</Label>
          <Input
            id="ed-max"
            type="number"
            value={defaults.max ?? ""}
            onChange={(e) => setDefaults({ max: e.target.value === "" ? undefined : Number(e.target.value) })}
            placeholder="auto"
          />
        </div>
      </div>

      <div className="space-y-1.5">
        <Label htmlFor="ed-novalue">No-value display</Label>
        <Input
          id="ed-novalue"
          value={defaults.noValue ?? ""}
          onChange={(e) => setDefaults({ noValue: e.target.value || undefined })}
          placeholder="—"
        />
      </div>

      <ThresholdsEditor
        steps={defaults.thresholds ?? []}
        onChange={(thresholds: ThresholdStep[]) =>
          setDefaults({ thresholds: thresholds.length > 0 ? thresholds : undefined })
        }
      />

      <ValueMappingsEditor
        mappings={defaults.mappings ?? []}
        onChange={(mappings: ValueMapping[]) =>
          setDefaults({ mappings: mappings.length > 0 ? mappings : undefined })
        }
      />
    </div>
  );
}
