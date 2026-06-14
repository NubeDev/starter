import {
  Activity,
  Cpu,
  Droplets,
  Factory,
  Gauge,
  LayoutDashboard,
  Radio,
  SignalHigh,
  Snowflake,
  Thermometer,
  Wind,
  Zap,
  type LucideIcon,
} from "lucide-react";

const MAP: Record<string, LucideIcon> = {
  LayoutDashboard,
  Zap,
  Snowflake,
  Wind,
  Activity,
  Gauge,
  Cpu,
  Radio,
  Thermometer,
  Droplets,
  SignalHigh,
  Factory,
};

export function DashIcon({
  name,
  className,
}: {
  name: string;
  className?: string;
}) {
  const Icon = MAP[name] ?? Activity;
  return <Icon className={className} />;
}

export const ICON_NAMES = Object.keys(MAP);
