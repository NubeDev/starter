import { Input } from "@nube/starter-ui-kit/components/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import type { Transform } from "@/data/types";

// Per-kind config fields for one transform step. Exhaustive over
// `Transform.kind` (a compile error here until a new kind is handled), so
// the Transforms tab stays a thin list and each operation owns its own
// inputs. Presentational: the parent holds the pipeline and persists it.
const CALC_OPS = ["+", "-", "*", "/"] as const;
const FILTER_OPS = ["=", "!=", ">", ">=", "<", "<="] as const;
const AGGS = ["sum", "avg", "min", "max", "count"] as const;
const CALCS = ["last", "first", "sum", "avg", "min", "max", "count"] as const;

export function TransformRow({
  index,
  transform,
  onChange,
}: {
  index: number;
  transform: Transform;
  onChange: (next: Transform) => void;
}) {
  switch (transform.kind) {
    case "rename":
      return (
        <div className="grid grid-cols-2 gap-2">
          <Input
            aria-label={`Transform ${index + 1} from`}
            value={transform.from}
            onChange={(e) => onChange({ ...transform, from: e.target.value })}
            placeholder="from"
          />
          <Input
            aria-label={`Transform ${index + 1} to`}
            value={transform.to}
            onChange={(e) => onChange({ ...transform, to: e.target.value })}
            placeholder="to"
          />
        </div>
      );
    case "calculated":
      return (
        <div className="space-y-2">
          <Input
            aria-label={`Transform ${index + 1} new field`}
            value={transform.field}
            onChange={(e) => onChange({ ...transform, field: e.target.value })}
            placeholder="new field name"
          />
          <div className="flex items-center gap-2">
            <Input
              aria-label={`Transform ${index + 1} left`}
              value={transform.left}
              onChange={(e) => onChange({ ...transform, left: e.target.value })}
              placeholder="left"
              className="flex-1"
            />
            <Select
              value={transform.op}
              onValueChange={(v) => onChange({ ...transform, op: v as typeof transform.op })}
            >
              <SelectTrigger className="w-16 shrink-0" aria-label={`Transform ${index + 1} operator`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {CALC_OPS.map((o) => (
                  <SelectItem key={o} value={o}>
                    {o}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Input
              aria-label={`Transform ${index + 1} right`}
              value={transform.right}
              onChange={(e) => onChange({ ...transform, right: e.target.value })}
              placeholder="right"
              className="flex-1"
            />
          </div>
        </div>
      );
    case "filter":
      return (
        <div className="flex items-center gap-2">
          <Input
            aria-label={`Transform ${index + 1} field`}
            value={transform.field}
            onChange={(e) => onChange({ ...transform, field: e.target.value })}
            placeholder="field"
            className="flex-1"
          />
          <Select
            value={transform.op}
            onValueChange={(v) => onChange({ ...transform, op: v as typeof transform.op })}
          >
            <SelectTrigger className="w-20 shrink-0" aria-label={`Transform ${index + 1} operator`}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {FILTER_OPS.map((o) => (
                <SelectItem key={o} value={o}>
                  {o}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            aria-label={`Transform ${index + 1} value`}
            value={transform.value}
            onChange={(e) => onChange({ ...transform, value: e.target.value })}
            placeholder="value"
            className="flex-1"
          />
        </div>
      );
    case "groupBy":
      return (
        <div className="space-y-2">
          <div className="grid grid-cols-2 gap-2">
            <Input
              aria-label={`Transform ${index + 1} group by`}
              value={transform.by}
              onChange={(e) => onChange({ ...transform, by: e.target.value })}
              placeholder="group by field"
            />
            <Input
              aria-label={`Transform ${index + 1} field`}
              value={transform.field}
              onChange={(e) => onChange({ ...transform, field: e.target.value })}
              placeholder="aggregate field"
            />
          </div>
          <div className="flex items-center gap-2">
            <Select
              value={transform.agg}
              onValueChange={(v) => onChange({ ...transform, agg: v as typeof transform.agg })}
            >
              <SelectTrigger className="w-28 shrink-0" aria-label={`Transform ${index + 1} aggregation`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {AGGS.map((a) => (
                  <SelectItem key={a} value={a}>
                    {a}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Input
              aria-label={`Transform ${index + 1} output field`}
              value={transform.as}
              onChange={(e) => onChange({ ...transform, as: e.target.value })}
              placeholder="output field"
              className="flex-1"
            />
          </div>
        </div>
      );
    case "reduce":
      return (
        <div className="flex items-center gap-2">
          <Input
            aria-label={`Transform ${index + 1} field`}
            value={transform.field}
            onChange={(e) => onChange({ ...transform, field: e.target.value })}
            placeholder="field"
            className="flex-1"
          />
          <Select
            value={transform.calc}
            onValueChange={(v) => onChange({ ...transform, calc: v as typeof transform.calc })}
          >
            <SelectTrigger className="w-28 shrink-0" aria-label={`Transform ${index + 1} calculation`}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {CALCS.map((c) => (
                <SelectItem key={c} value={c}>
                  {c}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            aria-label={`Transform ${index + 1} output field`}
            value={transform.as}
            onChange={(e) => onChange({ ...transform, as: e.target.value })}
            placeholder="output field"
            className="flex-1"
          />
        </div>
      );
    case "organize":
      return (
        <Input
          aria-label={`Transform ${index + 1} field order`}
          value={transform.order.join(", ")}
          onChange={(e) =>
            onChange({
              ...transform,
              order: e.target.value
                .split(",")
                .map((s) => s.trim())
                .filter(Boolean),
            })
          }
          placeholder="comma-separated field order"
        />
      );
  }
}
