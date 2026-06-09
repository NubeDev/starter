import { Check, Palette as PaletteIcon } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@nube/starter-ui-kit/components/dropdown-menu";

import { useThemeStore } from "@/theme/store";
import { palettes } from "@/theme/theme";

// Colour-system switcher for the header. Lets the user pick one of the
// bundled palettes (emerald / ocean / violet); each works in both light
// and dark mode. State lives in `useThemeStore`, which repaints the
// `--*` tokens on <html> and re-tints the charts — this only reads/sets.
//
// The swatch previews the palette's *dark-mode* primary so the menu reads
// the same regardless of the current mode.
export function PaletteSwitcher() {
  const palette = useThemeStore((s) => s.palette);
  const setPalette = useThemeStore((s) => s.setPalette);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="icon"
          className="size-8"
          aria-label="Colour palette"
        >
          <PaletteIcon className="size-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-48">
        <DropdownMenuLabel>Colour palette</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {palettes.map((p) => {
          const active = p.id === palette;
          return (
            <DropdownMenuItem
              key={p.id}
              onSelect={() => setPalette(p.id)}
              className="gap-2"
            >
              <span
                aria-hidden
                className="size-4 shrink-0 rounded-full border border-border/60"
                style={{ background: p.dark.primary }}
              />
              <span className="flex-1">{p.name}</span>
              {active && <Check className="size-4 shrink-0 opacity-70" />}
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
