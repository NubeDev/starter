// Horizontal-scroll gallery of preset cards. Each card shows a
// four-swatch preview and applies its `ThemeStyles` on click.
//
// The component is presentational — it reads the preset list and the
// active-mode flag from props, and pushes the picked styles into the
// editor store via the supplied callback. That separation lets a
// consumer slot in a custom preset source (org-curated themes, AI
// suggestions, …) without rewriting the UI.

import { useThemeEditorStore } from "@nube/starter-ui-core/theme-editor";
import type { ThemeMode, ThemePreset, ThemeStyleProps, ThemeStyles } from "@nube/starter-ui-core/theme-editor";

import { cn } from "../lib/utils";

export interface ThemeGalleryProps {
  presets: readonly ThemePreset[];
  /** Optional active-preset id to highlight; matched by `preset.id`. */
  activePresetId?: string | null;
  /** Called when the user picks a card. */
  onPick?: (preset: ThemePreset) => void;
}

export function ThemeGallery({ presets, activePresetId, onPick }: ThemeGalleryProps) {
  const applyPresetStyles = useThemeEditorStore((s) => s.applyPresetStyles);
  const mode = useThemeEditorStore((s) => s.mode);

  return (
    <div className="flex gap-3 overflow-x-auto pb-2" data-testid="theme-gallery">
      {presets.map((preset) => {
        const isActive = preset.id === activePresetId;
        return (
          <button
            key={preset.id}
            type="button"
            onClick={() => {
              applyPresetStyles(preset.styles);
              onPick?.(preset);
            }}
            className={cn(
              "flex shrink-0 flex-col gap-2 rounded-lg border bg-card p-3 text-left",
              "transition-colors hover:border-ring focus-visible:border-ring focus-visible:outline-none",
              isActive && "border-ring ring-2 ring-ring/30",
            )}
            aria-pressed={isActive}
            aria-label={preset.description ?? preset.label}
          >
            <SwatchRow styles={preset.styles} mode={mode} />
            <span className="text-sm font-medium text-card-foreground">{preset.label}</span>
          </button>
        );
      })}
    </div>
  );
}

function SwatchRow({ styles, mode }: { styles: ThemeStyles; mode: ThemeMode }) {
  const map: ThemeStyleProps = styles[mode];
  const swatches: Array<{ key: keyof ThemeStyleProps; title: string }> = [
    { key: "background", title: "Background" },
    { key: "primary", title: "Primary" },
    { key: "accent", title: "Accent" },
    { key: "sidebar", title: "Sidebar" },
  ];
  return (
    <div className="flex gap-1">
      {swatches.map(({ key, title }) => (
        <div
          key={key}
          title={title}
          className="size-6 rounded border border-border"
          style={{ backgroundColor: map[key] }}
        />
      ))}
    </div>
  );
}
