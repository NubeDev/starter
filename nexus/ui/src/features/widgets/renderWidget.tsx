import type { Widget, WidgetData } from "@/data/types";
import { Area } from "@/features/widgets/Area";
import { DeviceTable } from "@/features/widgets/DeviceTable";
import { Gauge } from "@/features/widgets/Gauge";
import { Line } from "@/features/widgets/Line";
import { Stat } from "@/features/widgets/Stat";
import { Status } from "@/features/widgets/Status";

// Dispatches a widget to its renderer by type. The exhaustive switch is
// the one place panel types are enumerated; adding a type is a compile
// error here until it's handled. Pure — every renderer takes the same
// typed props and fetches nothing (F6).
export function RenderWidget({
  widget,
  data,
}: {
  widget: Widget;
  data: WidgetData;
}) {
  switch (widget.type) {
    case "line":
      return <Line widget={widget} data={data} />;
    case "area":
      return <Area widget={widget} data={data} />;
    case "gauge":
      return <Gauge widget={widget} data={data} />;
    case "stat":
      return <Stat widget={widget} data={data} />;
    case "status":
      return <Status widget={widget} data={data} />;
    case "table":
      return <DeviceTable widget={widget} data={data} />;
  }
}
