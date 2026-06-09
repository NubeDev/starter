import type { WidgetData } from "@/data/types";
import type { QueryResponse } from "@/api/types";

// Reshape a `POST /query` response into the `WidgetData` a panel renders.
// The server already returns one JSON object per row keyed by column
// name, which *is* `SeriesPoint`, so this is a total, allocation-light
// mapping — no per-cell coercion, no invented values (F0). Column types
// (`columns[].type`) are available for callers that need them but a panel
// reads only the columns its field mapping names.
export function toWidgetData(response: QueryResponse): WidgetData {
  return { points: response.rows as WidgetData["points"] };
}
