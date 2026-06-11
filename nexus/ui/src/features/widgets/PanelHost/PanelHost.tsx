import type { Widget } from "@/data/types";
import { WidgetCard } from "@/features/widgets/WidgetCard";
import { LivePanel } from "@/features/widgets/LivePanel";
import { useWidgetQuery } from "@/features/widgets/useWidgetQuery";

// Binds a panel to its data and renders the card. A panel that declares a
// live stream routes through `LivePanel` (SSE); otherwise it runs a
// one-shot query. Either way the card contract is identical, so the widget
// components never know which feed they're on and stay pure (F6). This is
// the only place a panel's data is fetched. Edit affordances
// (`editing`/`onRemove`) pass straight through to the card.
export function PanelHost({
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
  const common = { editing, selected, onRemove, onSelect, onDuplicate };
  if (widget.config.live?.streamId) {
    return <LivePanel widget={widget} {...common} />;
  }
  return <QueryPanel widget={widget} {...common} />;
}

// Split out so the live/query hooks each live behind their own component
// (a hook can't be called conditionally).
function QueryPanel({
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
  const state = useWidgetQuery(widget);
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
