import {
  Check,
  Clock,
  Monitor,
  Moon,
  PanelLeft,
  Palette as PaletteIcon,
  Settings2,
  Sun,
} from "lucide-react";
import type { DateFormat, TimeFormat } from "@nube/starter-ui-core/preferences";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@nube/starter-ui-kit/components/dropdown-menu";

import {
  useLayout,
  type SidebarCollapsible,
  type SidebarVariant,
} from "@/app/LayoutProvider";
import { palettes, useThemeStore } from "@/theme";
import type { ThemePreference } from "@/theme";
import { REGION_LABELS, type Region, useDateTime } from "@/datetime";
import { useDateTimeControls } from "@/datetime/useDateTimeControls";

const MODES: { value: ThemePreference; label: string; icon: typeof Sun }[] = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "System", icon: Monitor },
];

// Industry-standard date/time controls: explicit, independent knobs over
// an "Automatic" baseline that follows the device locale.
const DATE_FORMATS: { value: DateFormat; label: string }[] = [
  { value: "auto", label: "Automatic" },
  { value: "YYYY-MM-DD", label: "2026-03-09 (ISO)" },
  { value: "MM/DD/YYYY", label: "03/09/2026 (US)" },
  { value: "DD/MM/YYYY", label: "09/03/2026 (EU)" },
];
const TIME_FORMATS: { value: TimeFormat; label: string }[] = [
  { value: "auto", label: "Automatic" },
  { value: "24h", label: "24-hour" },
  { value: "12h", label: "12-hour" },
];

const REGIONS = Object.keys(REGION_LABELS) as Region[];
const VARIANTS: SidebarVariant[] = ["floating", "inset", "sidebar"];
const COLLAPSE: SidebarCollapsible[] = ["icon", "offcanvas", "none"];

// Single settings menu for the header: consolidates everything that used
// to be four separate icon buttons (theme mode, colour palette, display
// region, sidebar layout) into one gear dropdown. Appearance sits inline
// at the top; region and layout are submenus to keep the surface compact.
// Each section reads/sets its own store — no shared state here.
export function SettingsMenu() {
  const mode = useThemeStore((s) => s.mode);
  const preference = useThemeStore((s) => s.preference);
  const setPreference = useThemeStore((s) => s.setPreference);
  const palette = useThemeStore((s) => s.palette);
  const setPalette = useThemeStore((s) => s.setPalette);

  // Persists to the backend when a session is present (applies across
  // devices), else to the per-device local store. Same surface either way.
  const { dateFormat, timeFormat, set, reset, applyRegion } =
    useDateTimeControls();
  // Live preview of the current date/time choice.
  const { dateTime } = useDateTime();

  const { variant, setVariant, collapsible, setCollapsible } = useLayout();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="icon"
          className="size-8"
          aria-label="Settings"
        >
          <Settings2 className="size-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        {/* Appearance — light / dark / system */}
        <DropdownMenuLabel>Appearance</DropdownMenuLabel>
        <DropdownMenuRadioGroup
          value={preference}
          onValueChange={(v) => setPreference(v as ThemePreference)}
        >
          {MODES.map(({ value, label, icon: Icon }) => (
            <DropdownMenuRadioItem key={value} value={value} className="gap-2">
              <Icon className="size-4" />
              {label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>

        <DropdownMenuSeparator />

        {/* Palette — the switchable colour systems */}
        <DropdownMenuLabel className="flex items-center gap-2">
          <PaletteIcon className="size-3.5" /> Palette
        </DropdownMenuLabel>
        {palettes.map((p) => (
          <DropdownMenuItem
            key={p.id}
            onSelect={() => setPalette(p.id)}
            className="gap-2"
          >
            <span
              aria-hidden
              className="size-4 shrink-0 rounded-full border border-border/60"
              style={{ background: mode === "dark" ? p.dark.primary : p.light.primary }}
            />
            <span className="flex-1">{p.name}</span>
            {p.id === palette && (
              <Check className="size-4 shrink-0 opacity-70" />
            )}
          </DropdownMenuItem>
        ))}

        <DropdownMenuSeparator />

        {/* Date & time — how timestamps render across the app. Explicit
            format + clock controls, each defaulting to Automatic (the
            device locale); the region row is a quick-set shortcut. */}
        <DropdownMenuSub>
          <DropdownMenuSubTrigger className="gap-2">
            <Clock className="size-4" />
            <span className="flex-1">Date &amp; time</span>
            <span className="text-xs text-muted-foreground">
              {dateTime(Date.now())}
            </span>
          </DropdownMenuSubTrigger>
          <DropdownMenuSubContent className="w-56">
            <DropdownMenuLabel>Date format</DropdownMenuLabel>
            <DropdownMenuRadioGroup
              value={dateFormat}
              onValueChange={(v) => set({ dateFormat: v as DateFormat })}
            >
              {DATE_FORMATS.map((o) => (
                <DropdownMenuRadioItem key={o.value} value={o.value}>
                  {o.label}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>

            <DropdownMenuSeparator />
            <DropdownMenuLabel>Time format</DropdownMenuLabel>
            <DropdownMenuRadioGroup
              value={timeFormat}
              onValueChange={(v) => set({ timeFormat: v as TimeFormat })}
            >
              {TIME_FORMATS.map((o) => (
                <DropdownMenuRadioItem key={o.value} value={o.value}>
                  {o.label}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>

            <DropdownMenuSeparator />
            <DropdownMenuLabel>Quick set</DropdownMenuLabel>
            {REGIONS.map((r) => (
              <DropdownMenuItem key={r} onSelect={() => applyRegion(r)}>
                {REGION_LABELS[r]}
              </DropdownMenuItem>
            ))}
            <DropdownMenuSeparator />
            <DropdownMenuItem onSelect={reset}>
              Reset to automatic
            </DropdownMenuItem>
          </DropdownMenuSubContent>
        </DropdownMenuSub>

        {/* Layout — sidebar variant + collapse behaviour */}
        <DropdownMenuSub>
          <DropdownMenuSubTrigger className="gap-2">
            <PanelLeft className="size-4" />
            <span className="flex-1">Layout</span>
            <span className="text-xs capitalize text-muted-foreground">
              {variant}
            </span>
          </DropdownMenuSubTrigger>
          <DropdownMenuSubContent>
            <DropdownMenuLabel>Sidebar style</DropdownMenuLabel>
            <DropdownMenuRadioGroup
              value={variant}
              onValueChange={(v) => setVariant(v as SidebarVariant)}
            >
              {VARIANTS.map((v) => (
                <DropdownMenuRadioItem key={v} value={v} className="capitalize">
                  {v}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
            <DropdownMenuSeparator />
            <DropdownMenuLabel>Collapse</DropdownMenuLabel>
            <DropdownMenuRadioGroup
              value={collapsible}
              onValueChange={(c) => setCollapsible(c as SidebarCollapsible)}
            >
              {COLLAPSE.map((c) => (
                <DropdownMenuRadioItem key={c} value={c} className="capitalize">
                  {c}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </DropdownMenuSubContent>
        </DropdownMenuSub>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
