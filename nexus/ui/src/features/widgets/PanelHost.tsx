import type { Widget } from "@/data/types";
import { WidgetCard } from "@/features/widgets/WidgetCard";
import { useWidgetQuery } from "@/features/widgets/useWidgetQuery";

// Binds a panel to its data and renders the card. This is the only place
// a panel's query is run; the widget components stay pure (F6). A panel
// declaring a live stream will instead route through a live host (added
// when streaming panels land) — the card contract is identical, so the
// widget never knows which feed it's on.
export function PanelHost({ widget }: { widget: Widget }) {
  const state = useWidgetQuery(widget);
  return <WidgetCard widget={widget} state={state} />;
}
