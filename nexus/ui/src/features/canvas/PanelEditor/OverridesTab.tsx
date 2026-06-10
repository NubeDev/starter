import { Plus, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
import { Switch } from "@nube/starter-ui-kit/components/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import type { FieldMatcher, FieldOverride } from "@/data/types";
import type { EditorDraft } from "@/features/canvas/PanelEditor/useEditorDraft";
import { hexToHsl, hslToHex } from "@/features/canvas/PanelEditor/hslHex";
import { UnitPicker } from "@/features/canvas/PanelEditor/UnitPicker";

// Overrides tab: targeted exceptions to the field defaults. Each override
// matches a series by name or regex and lays display props on top — a
// display name, unit, colour, or hidden flag. The first matching override
// wins (resolution order = list order). Writes `fieldConfig.overrides`.
export function OverridesTab({ draft }: { draft: EditorDraft }) {
  const { widget, patchConfig } = draft;
  const fieldConfig = widget.config.fieldConfig ?? {};
  const overrides = fieldConfig.overrides ?? [];

  function set(next: FieldOverride[]) {
    patchConfig({
      fieldConfig: { ...fieldConfig, overrides: next.length > 0 ? next : undefined },
    });
  }
  function update(i: number, patch: Partial<FieldOverride>) {
    set(overrides.map((o, j) => (j === i ? { ...o, ...patch } : o)));
  }
  function updateMatcher(i: number, patch: Partial<FieldMatcher>) {
    update(i, { matcher: { ...overrides[i].matcher, ...patch } });
  }
  function updateDisplay(i: number, patch: Partial<FieldOverride["display"]>) {
    update(i, { display: { ...overrides[i].display, ...patch } });
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <p className="text-xs text-muted-foreground">
          Match series by name or regex, then override their display.
        </p>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 gap-1 px-2 text-xs"
          onClick={() => set([...overrides, { matcher: { type: "byName", value: "" }, display: {} }])}
        >
          <Plus className="size-3.5" /> Add override
        </Button>
      </div>

      {overrides.map((o, i) => (
        <div key={i} className="space-y-3 rounded-lg border border-border/60 p-3">
          <div className="flex items-center gap-2">
            <Select
              value={o.matcher.type}
              onValueChange={(v) => updateMatcher(i, { type: v as FieldMatcher["type"] })}
            >
              <SelectTrigger className="w-32 shrink-0" aria-label={`Override ${i + 1} matcher`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="byName">By name</SelectItem>
                <SelectItem value="byRegex">By regex</SelectItem>
              </SelectContent>
            </Select>
            <Input
              aria-label={`Override ${i + 1} pattern`}
              value={o.matcher.value}
              onChange={(e) => updateMatcher(i, { value: e.target.value })}
              placeholder={o.matcher.type === "byRegex" ? "/temp/" : "column name"}
              className="flex-1"
            />
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label={`Remove override ${i + 1}`}
              className="size-9 text-muted-foreground hover:text-destructive"
              onClick={() => set(overrides.filter((_, j) => j !== i))}
            >
              <Trash2 className="size-4" />
            </Button>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <Label className="text-xs text-muted-foreground">Display name</Label>
              <Input
                value={o.display.displayName ?? ""}
                onChange={(e) => updateDisplay(i, { displayName: e.target.value || undefined })}
              />
            </div>
            <div className="space-y-1">
              <Label className="text-xs text-muted-foreground">Unit</Label>
              <UnitPicker value={o.display.unit} onChange={(unit) => updateDisplay(i, { unit })} />
            </div>
          </div>

          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              <input
                type="color"
                aria-label={`Override ${i + 1} colour`}
                value={hslToHex(o.display.color)}
                onChange={(e) => updateDisplay(i, { color: hexToHsl(e.target.value) })}
                className="h-8 w-8 cursor-pointer rounded border border-border bg-transparent"
              />
              <span className="text-xs text-muted-foreground">Colour</span>
            </div>
            <label className="flex items-center gap-2 text-xs text-muted-foreground">
              Hidden
              <Switch
                checked={o.display.hidden ?? false}
                onCheckedChange={(hidden) => updateDisplay(i, { hidden: hidden || undefined })}
                aria-label={`Override ${i + 1} hidden`}
              />
            </label>
          </div>
        </div>
      ))}
    </div>
  );
}
