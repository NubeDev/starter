import { PanelLeft, Settings2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@nube/starter-ui-kit/components/dropdown-menu";

import {
  useLayout,
  type SidebarCollapsible,
  type SidebarVariant,
} from "@/app/LayoutProvider";

const VARIANTS: SidebarVariant[] = ["floating", "inset", "sidebar"];
const COLLAPSE: SidebarCollapsible[] = ["icon", "offcanvas", "none"];

// Lets the user reshape the shell: sidebar variant (floating / inset /
// flush) and how it collapses. Choices persist via the layout provider's
// cookies, mirroring the shadcn-admin config drawer.
export function LayoutSwitcher() {
  const { variant, setVariant, collapsible, setCollapsible } = useLayout();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="icon" aria-label="Layout settings">
          <Settings2 className="size-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-44">
        <DropdownMenuLabel className="flex items-center gap-2">
          <PanelLeft className="size-3.5" /> Sidebar style
        </DropdownMenuLabel>
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
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
