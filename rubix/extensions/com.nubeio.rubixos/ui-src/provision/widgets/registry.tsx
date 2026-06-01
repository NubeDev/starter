// `registry.tsx` — maps a widget enum string to its renderer + renderWidget().
import * as React from "react";
import { GaugeWidget } from "./gauge";
import { StatWidget } from "./stat";
import { BatteryWidget } from "./battery";
import { CounterWidget } from "./counter";
import { LedWidget } from "./led";
import { ToggleWidget } from "./toggle";
import { LineWidget } from "./line";

export interface WidgetProps {
  title: string;
  value?: number | string | boolean;
  unit?: string;
  widget: string;
}

type WidgetComponent = (props: WidgetProps) => React.ReactElement;

const REGISTRY: Record<string, WidgetComponent> = {
  gauge: GaugeWidget,
  stat: StatWidget,
  battery: BatteryWidget,
  counter: CounterWidget,
  led: LedWidget,
  toggle: ToggleWidget,
  line: LineWidget,
};

/** Render the renderer matching `widget`, falling back to a stat readout. */
export function renderWidget(widget: string, props: WidgetProps): React.ReactElement {
  const Cmp = REGISTRY[widget] ?? StatWidget;
  return <Cmp {...props} widget={widget} />;
}

export function hasWidget(widget: string): boolean {
  return widget in REGISTRY;
}
