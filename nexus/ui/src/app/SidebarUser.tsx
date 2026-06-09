import { ChevronsUpDown, LogOut, UserRound } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@nube/starter-ui-kit/components/dropdown-menu";

import { useAuth } from "@/auth/AuthProvider";
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";

// The signed-in identity + logout, pinned to the sidebar footer. The principal
// carries no email/name (just subject/role/tenant), so the row shows the role
// and tenant — enough to confirm who you are — and the menu's only action is
// Log out. Mirrors the LayoutSwitcher/SettingsMenu dropdown pattern.
export function SidebarUser() {
  const { user, logout } = useAuth();
  if (!user) return null;

  const tenant = user.tenant_id && user.tenant_id !== "*" ? user.tenant_id : null;
  const subtitle = tenant ? `${user.role} · ${tenant}` : user.role;

  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <SidebarMenuButton
              size="lg"
              tooltip="Account"
              className="gap-2 data-[state=open]:bg-sidebar-accent"
            >
              <span className="grid size-8 shrink-0 place-items-center rounded-lg bg-primary/15 text-primary">
                <UserRound className="size-4" />
              </span>
              <span className="grid flex-1 text-left leading-tight">
                <span className="truncate text-sm font-medium">Account</span>
                <span className="truncate text-xs text-muted-foreground">
                  {subtitle}
                </span>
              </span>
              <ChevronsUpDown className="ml-auto size-4 opacity-60" />
            </SidebarMenuButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            side="top"
            align="start"
            className="w-(--radix-popper-anchor-width) min-w-56"
          >
            <DropdownMenuLabel className="grid">
              <span className="text-sm font-medium">Signed in</span>
              <span className="truncate text-xs font-normal text-muted-foreground">
                {subtitle}
              </span>
            </DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              onSelect={() => {
                void logout();
              }}
            >
              <LogOut className="size-4" />
              Log out
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}
