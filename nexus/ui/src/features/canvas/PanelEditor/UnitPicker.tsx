import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import { UNIT_GROUPS } from "@/features/widgets/_shared/units";

// Grouped unit dropdown (SI, temperature, data rate, currency, …) backed
// by the unit registry. Stores the unit *id*; the renderers turn it into a
// display symbol. `"none"` clears the unit. A presentation-only control:
// it holds no state, just maps value ⇄ onChange.
export function UnitPicker({
  value,
  onChange,
  id,
}: {
  value: string | undefined;
  onChange: (unit: string | undefined) => void;
  id?: string;
}) {
  return (
    <Select
      value={value ?? "none"}
      onValueChange={(v) => onChange(v === "none" ? undefined : v)}
    >
      <SelectTrigger id={id}>
        <SelectValue placeholder="Unit" />
      </SelectTrigger>
      <SelectContent>
        {UNIT_GROUPS.map((group) => (
          <SelectGroup key={group.label}>
            <SelectLabel>{group.label}</SelectLabel>
            {group.units.map((u) => (
              <SelectItem key={u.id} value={u.id}>
                {u.label}
              </SelectItem>
            ))}
          </SelectGroup>
        ))}
      </SelectContent>
    </Select>
  );
}
