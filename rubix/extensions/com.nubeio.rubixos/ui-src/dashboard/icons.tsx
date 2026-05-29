// Icon set + role-accent palette for the dashboard.
//
// Re-exports `lucide-react` glyphs under stable local names so
// the consuming files don't have to know which Lucide icon we
// chose for "region" or "alert" — and so a swap (e.g. MapPin →
// LandPlot) is one line in one place.

import * as React from "react";
import {
  Zap,
  Droplet,
  Map as LMap,
  MapPin,
  LayoutGrid,
  Grid3x3,
  Clock,
  Activity,
  List,
  TrendingUp,
  AlertTriangle,
  Printer,
  Download,
  Check,
  Filter,
  CalendarRange,
  Building2,
  Bookmark,
  Save,
  Gauge,
  Layers,
  Hash,
  CircleDot,
  Wrench,
  Search,
  Eye,
  EyeOff,
  RotateCcw,
  Power,
  ChevronRight,
  Sparkles,
} from "lucide-react";

// Generic "icon component" type — anything with `size` + `className`.
export type IconLike = React.ComponentType<{ size?: number; className?: string; strokeWidth?: number }>;

// Re-export under the names the rest of the dashboard already uses.
export const IconBolt: IconLike = Zap;
export const IconDroplet: IconLike = Droplet;
export const IconMap: IconLike = LMap;
export const IconMapPin: IconLike = MapPin;
export const IconGrid: IconLike = LayoutGrid;
export const IconGridDense: IconLike = Grid3x3;
export const IconRegion: IconLike = Building2;
export const IconClock: IconLike = Clock;
export const IconWave: IconLike = Activity;
export const IconList: IconLike = List;
export const IconTrend: IconLike = TrendingUp;
export const IconAlert: IconLike = AlertTriangle;
export const IconPrint: IconLike = Printer;
export const IconDownload: IconLike = Download;
export const IconCheck: IconLike = Check;
export const IconFilter: IconLike = Filter;
export const IconRange: IconLike = CalendarRange;
export const IconSites: IconLike = Building2;
export const IconBookmark: IconLike = Bookmark;
export const IconSave: IconLike = Save;
export const IconGauge: IconLike = Gauge;
export const IconLayers: IconLike = Layers;
export const IconHash: IconLike = Hash;
export const IconDot: IconLike = CircleDot;
export const IconWrench: IconLike = Wrench;
export const IconSearch: IconLike = Search;
export const IconEye: IconLike = Eye;
export const IconEyeOff: IconLike = EyeOff;
export const IconReset: IconLike = RotateCcw;
export const IconPower: IconLike = Power;
export const IconChevronRight: IconLike = ChevronRight;
export const IconSparkles: IconLike = Sparkles;

/* ============================ role accents =========================== */
//
// Each meter kind gets its own visual identity beyond the shared
// teal accent. Elec = amber (high-energy, "hot"); water = sky
// (cool, fluid). Used by KpiCard, RadialKpi, the report hero,
// and section headers so the page's dominant colour reflects
// what the user is looking at.

export interface RoleAccent {
  Icon: IconLike;
  text: string;
  ring: string;
  cssColor: string;
  bgTint: string;
  borderTint: string;
  label: string;
}

export const ROLE_ACCENT: Record<"elec" | "water", RoleAccent> = {
  elec: {
    Icon: Zap,
    text: "text-amber-400",
    ring: "ring-amber-400/40",
    cssColor: "#f59e0b",
    bgTint: "bg-amber-400/[0.06]",
    borderTint: "border-amber-400/30",
    label: "Electrical",
  },
  water: {
    Icon: Droplet,
    text: "text-sky-400",
    ring: "ring-sky-400/40",
    cssColor: "#38bdf8",
    bgTint: "bg-sky-400/[0.06]",
    borderTint: "border-sky-400/30",
    label: "Water",
  },
};
