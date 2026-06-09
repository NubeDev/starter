import { Monitor, Moon, Sun } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@nube/starter-ui-kit/components/dropdown-menu";

import { useThemeStore } from "@/theme/store";
import type { ThemePreference } from "@/theme/theme";

const OPTIONS: { value: ThemePreference; label: string; icon: typeof Sun }[] = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "System", icon: Monitor },
];

// Dark/light switcher for the header. The trigger shows the *resolved*
// mode (sun/moon), the menu lets you pin Light, Dark, or follow System.
// All state lives in `useThemeStore`; this component only reads and sets.
export function ThemeToggle() {
  const preference = useThemeStore((s) => s.preference);
  const mode = useThemeStore((s) => s.mode);
  const setPreference = useThemeStore((s) => s.setPreference);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="icon"
          className="size-8"
          aria-label="Toggle theme"
        >
          {mode === "dark" ? (
            <Moon className="size-4" />
          ) : (
            <Sun className="size-4" />
          )}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-36">
        <DropdownMenuRadioGroup
          value={preference}
          onValueChange={(v) => setPreference(v as ThemePreference)}
        >
          {OPTIONS.map(({ value, label, icon: Icon }) => (
            <DropdownMenuRadioItem key={value} value={value} className="gap-2">
              <Icon className="size-4" />
              {label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
