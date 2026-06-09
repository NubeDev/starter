import { Plus, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import type { ValueMapping } from "@/data/types";

// Editor for value mappings (value / range / regex → display text). The
// first matching mapping wins at render; mappings are listed in match
// order here. Presentational — owned by the Field tab.
export function ValueMappingsEditor({
  mappings,
  onChange,
}: {
  mappings: ReadonlyArray<ValueMapping>;
  onChange: (mappings: ValueMapping[]) => void;
}) {
  function update(i: number, patch: Partial<ValueMapping>) {
    onChange(mappings.map((m, j) => (j === i ? { ...m, ...patch } : m)));
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <Label>Value mappings</Label>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 gap-1 px-2 text-xs"
          onClick={() => onChange([...mappings, { type: "value", match: "", text: "" }])}
        >
          <Plus className="size-3.5" /> Add mapping
        </Button>
      </div>
      {mappings.map((m, i) => (
        <div key={i} className="flex items-center gap-2">
          <Select value={m.type} onValueChange={(v) => update(i, { type: v as ValueMapping["type"] })}>
            <SelectTrigger className="w-24 shrink-0" aria-label={`Mapping ${i + 1} type`}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="value">Value</SelectItem>
              <SelectItem value="range">Range</SelectItem>
              <SelectItem value="regex">Regex</SelectItem>
            </SelectContent>
          </Select>
          {m.type === "range" ? (
            <>
              <Input
                type="number"
                aria-label={`Mapping ${i + 1} from`}
                value={m.from ?? ""}
                onChange={(e) => update(i, { from: e.target.value === "" ? undefined : Number(e.target.value) })}
                placeholder="from"
                className="w-20"
              />
              <Input
                type="number"
                aria-label={`Mapping ${i + 1} to`}
                value={m.to ?? ""}
                onChange={(e) => update(i, { to: e.target.value === "" ? undefined : Number(e.target.value) })}
                placeholder="to"
                className="w-20"
              />
            </>
          ) : (
            <Input
              aria-label={`Mapping ${i + 1} match`}
              value={m.match ?? ""}
              onChange={(e) => update(i, { match: e.target.value })}
              placeholder={m.type === "regex" ? "pattern" : "value"}
              className="flex-1"
            />
          )}
          <Input
            aria-label={`Mapping ${i + 1} text`}
            value={m.text ?? ""}
            onChange={(e) => update(i, { text: e.target.value })}
            placeholder="display"
            className="flex-1"
          />
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={`Remove mapping ${i + 1}`}
            className="size-8 text-muted-foreground hover:text-destructive"
            onClick={() => onChange(mappings.filter((_, j) => j !== i))}
          >
            <Trash2 className="size-4" />
          </Button>
        </div>
      ))}
    </div>
  );
}
