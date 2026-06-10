import { useState } from "react";
import { ChevronDown } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Checkbox } from "@nube/starter-ui-kit/components/checkbox";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@nube/starter-ui-kit/components/popover";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import type { ResolvedVariable } from "@/data/types";

// One control in the variable bar, dispatched on the variable's kind/flags:
//   - textbox  → a free-text input (commits on Enter/blur)
//   - multi    → a checkbox dropdown with an "All" toggle
//   - single   → a Select dropdown
// Each emits the full selection array so the parent treats them uniformly.
// The control is pure presentation: it never fetches and never mutates the
// store directly — selection changes flow up via `onChange`.
export function VariableControl({
  variable,
  onChange,
}: {
  variable: ResolvedVariable;
  onChange: (values: string[]) => void;
}) {
  const label = variable.label || variable.name;

  if (variable.kind === "textbox") {
    return <TextboxControl variable={variable} label={label} onChange={onChange} />;
  }
  if (variable.multi) {
    return <MultiControl variable={variable} label={label} onChange={onChange} />;
  }
  return <SingleControl variable={variable} label={label} onChange={onChange} />;
}

function FieldLabel({ htmlFor, children }: { htmlFor: string; children: string }) {
  return (
    <Label htmlFor={htmlFor} className="text-xs text-muted-foreground">
      {children}
    </Label>
  );
}

function SingleControl({
  variable,
  label,
  onChange,
}: {
  variable: ResolvedVariable;
  label: string;
  onChange: (values: string[]) => void;
}) {
  const id = `var-${variable.name}`;
  const value = variable.current[0] ?? "";
  return (
    <div className="flex flex-col gap-1">
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Select value={value} onValueChange={(v) => onChange([v])}>
        <SelectTrigger id={id} className="h-8 min-w-32">
          <SelectValue placeholder="Select…" />
        </SelectTrigger>
        <SelectContent>
          {variable.options.map((o) => (
            <SelectItem key={o.value} value={o.value}>
              {o.text}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

function MultiControl({
  variable,
  label,
  onChange,
}: {
  variable: ResolvedVariable;
  label: string;
  onChange: (values: string[]) => void;
}) {
  const id = `var-${variable.name}`;
  const selected = new Set(variable.current);
  const allValues = variable.options.map((o) => o.value);
  const allSelected = allValues.length > 0 && allValues.every((v) => selected.has(v));

  const toggle = (value: string, on: boolean) => {
    const next = new Set(selected);
    if (on) next.add(value);
    else next.delete(value);
    onChange(allValues.filter((v) => next.has(v)));
  };

  const summary =
    selected.size === 0
      ? "None"
      : allSelected
        ? "All"
        : variable.current
            .map((v) => variable.options.find((o) => o.value === v)?.text ?? v)
            .join(", ");

  return (
    <div className="flex flex-col gap-1">
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Popover>
        <PopoverTrigger asChild>
          <Button
            id={id}
            variant="outline"
            className="h-8 min-w-32 justify-between font-normal"
          >
            <span className="truncate">{summary}</span>
            <ChevronDown className="ml-2 size-4 shrink-0 opacity-60" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-56 p-2" align="start">
          {variable.includeAll ? (
            <label className="flex items-center gap-2 rounded px-2 py-1.5 text-sm hover:bg-accent">
              <Checkbox
                checked={allSelected}
                onCheckedChange={(c) => onChange(c === true ? allValues : [])}
              />
              All
            </label>
          ) : null}
          <div className="max-h-64 overflow-auto">
            {variable.options.map((o) => (
              <label
                key={o.value}
                className="flex items-center gap-2 rounded px-2 py-1.5 text-sm hover:bg-accent"
              >
                <Checkbox
                  checked={selected.has(o.value)}
                  onCheckedChange={(c) => toggle(o.value, c === true)}
                />
                {o.text}
              </label>
            ))}
          </div>
        </PopoverContent>
      </Popover>
    </div>
  );
}

function TextboxControl({
  variable,
  label,
  onChange,
}: {
  variable: ResolvedVariable;
  label: string;
  onChange: (values: string[]) => void;
}) {
  const id = `var-${variable.name}`;
  const [draft, setDraft] = useState(variable.current[0] ?? "");

  const commit = () => {
    const value = draft.trim();
    if (value !== (variable.current[0] ?? "")) onChange(value ? [value] : []);
  };

  return (
    <div className="flex flex-col gap-1">
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Input
        id={id}
        className="h-8 min-w-32"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
        }}
        placeholder="Type a value…"
      />
    </div>
  );
}
