import type { SeriesPoint, Transform, WidgetData } from "@/data/types";
import { applyCalculated } from "@/features/canvas/transforms/calculated";
import { applyFilter } from "@/features/canvas/transforms/filter";
import { applyGroupBy } from "@/features/canvas/transforms/groupBy";
import { applyOrganize } from "@/features/canvas/transforms/organize";
import { applyReduce } from "@/features/canvas/transforms/reduce";
import { applyRename } from "@/features/canvas/transforms/rename";

// Runs the panel's transform pipeline over fetched rows, in declared
// order, each transform's output feeding the next. Pure (F6): same data +
// same pipeline → same rows, no fetch, no mutation of the input. This is
// the one place the discriminated `Transform.kind` is dispatched, so a new
// transform kind is a compile error here until it is handled.
export function applyTransforms(
  data: WidgetData,
  transforms: ReadonlyArray<Transform> | undefined,
): WidgetData {
  if (!transforms || transforms.length === 0) return data;
  let rows: SeriesPoint[] = [...data.points];
  for (const t of transforms) {
    rows = applyOne(rows, t);
  }
  return { points: rows };
}

function applyOne(rows: SeriesPoint[], t: Transform): SeriesPoint[] {
  switch (t.kind) {
    case "rename":
      return applyRename(rows, t.from, t.to);
    case "calculated":
      return applyCalculated(rows, t.field, t.left, t.op, t.right);
    case "filter":
      return applyFilter(rows, t.field, t.op, t.value);
    case "groupBy":
      return applyGroupBy(rows, t.by, t.field, t.agg, t.as);
    case "reduce":
      return applyReduce(rows, t.field, t.calc, t.as);
    case "organize":
      return applyOrganize(rows, t.order);
  }
}
