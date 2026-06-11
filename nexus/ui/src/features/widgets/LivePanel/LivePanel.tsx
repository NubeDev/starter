import type { Widget } from "@/data/types";
import { WidgetCard } from "@/features/widgets/WidgetCard";
import { useLiveStream } from "@/features/widgets/useLiveStream";

// A panel fed by a live SSE stream rather than a one-shot query. Thin: it
// just pairs the live subscription with the card, the streaming twin of
// the query path in `PanelHost`.
export function LivePanel({
  widget,
  editing,
  selected,
  onRemove,
  onSelect,
  onDuplicate,
}: {
  widget: Widget;
  editing?: boolean;
  selected?: boolean;
  onRemove?: () => void;
  onSelect?: () => void;
  onDuplicate?: () => void;
}) {
  const state = useLiveStream(widget);
  return (
    <WidgetCard
      widget={widget}
      state={state}
      editing={editing}
      selected={selected}
      onRemove={onRemove}
      onSelect={onSelect}
      onDuplicate={onDuplicate}
    />
  );
}
