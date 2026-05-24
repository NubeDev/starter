// Generic Sheet + tab-nav shell for an in-app settings drawer.
// Stateless w.r.t. the user's preferences — the host owns those and
// supplies a `tabs` array. The drawer manages only which tab is
// active.
//
// The reset behaviour is also delegated: the host passes `onReset`
// and the localized `resetLabel` / `resetAriaLabel`. If neither is
// provided, the footer is not rendered.

import { useState, type ComponentType, type ReactNode } from "react";
import { Settings } from "lucide-react";
import { cn } from "../../lib/utils.js";
import { Button } from "../../components/ui/button.js";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "../../components/ui/sheet.js";

export interface ConfigDrawerTab {
  /** Stable id. Used as the React key and as the `role="tab"` /
   * `role="tabpanel"` ARIA id suffix. */
  id: string;
  /** Localized label rendered in the tab strip. */
  label: string;
  /** Tab icon. Defaults to a small lucide icon if omitted, but in
   * practice every host supplies one for affordance. */
  icon?: ComponentType<{ className?: string }>;
  /** The content rendered when this tab is active. */
  content: ReactNode;
}

export interface ConfigDrawerProps {
  tabs: ConfigDrawerTab[];
  /** Initial tab id; defaults to `tabs[0].id`. */
  defaultTabId?: string;
  /** Localized strings. The host should derive these from its
   * `useTranslate`-style hook. */
  i18n: {
    /** Sheet title. */
    title: string;
    /** Sheet description, displayed below the title. */
    description?: string;
    /** aria-label for the trigger button. */
    triggerAriaLabel: string;
    /** aria-label for the tab list. */
    tabsAriaLabel: string;
    /** Reset button label and aria-label. If `resetLabel` is omitted,
     * the footer is hidden. */
    resetLabel?: string;
    resetAriaLabel?: string;
  };
  /** Reset callback. Called when the user clicks the footer button.
   * If omitted (or `i18n.resetLabel` is omitted), the footer is
   * hidden. */
  onReset?: () => void;
  /** Custom trigger. If supplied, replaces the default Settings icon
   * button. Useful when the consumer wants to embed the drawer
   * trigger inside an existing toolbar. */
  trigger?: ReactNode;
  /** Class for the trigger button (only used when `trigger` is not
   * supplied). */
  triggerClassName?: string;
}

export function ConfigDrawer({
  tabs,
  defaultTabId,
  i18n,
  onReset,
  trigger,
  triggerClassName,
}: ConfigDrawerProps) {
  const initial = defaultTabId ?? tabs[0]?.id ?? "";
  const [activeId, setActiveId] = useState(initial);
  const activeTab = tabs.find((t) => t.id === activeId) ?? tabs[0];
  const showFooter = Boolean(onReset && i18n.resetLabel);

  const defaultTrigger = (
    <SheetTrigger
      aria-label={i18n.triggerAriaLabel}
      className={cn(
        "flex h-9 w-9 cursor-pointer items-center justify-center rounded-full text-[color:var(--color-muted)] transition-colors hover:bg-[color:var(--color-surface-2)]/50 hover:text-[color:var(--color-text)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]",
        triggerClassName,
      )}
    >
      <Settings className="h-4 w-4" aria-hidden="true" />
    </SheetTrigger>
  );

  return (
    <Sheet>
      {trigger ?? defaultTrigger}
      <SheetContent className="flex flex-col">
        <SheetHeader className="pb-0 text-start">
          <SheetTitle>{i18n.title}</SheetTitle>
          {i18n.description && <SheetDescription>{i18n.description}</SheetDescription>}
        </SheetHeader>

        <nav
          className="flex shrink-0 gap-1 border-b border-[color:var(--color-border)] px-4"
          role="tablist"
          aria-label={i18n.tabsAriaLabel}
        >
          {tabs.map(({ id, label, icon: Icon }) => {
            const active = activeId === id;
            return (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={active}
                aria-controls={`config-panel-${id}`}
                id={`config-tab-${id}`}
                onClick={() => setActiveId(id)}
                className={cn(
                  "group relative flex items-center gap-1.5 px-3 py-2.5 text-xs font-medium transition-colors",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]",
                  active
                    ? "text-[color:var(--color-text)]"
                    : "text-[color:var(--color-subtle)] hover:text-[color:var(--color-muted)]",
                )}
              >
                {Icon && <Icon className="size-3.5" aria-hidden="true" />}
                {label}
                {active && (
                  <span
                    className="absolute inset-x-2 -bottom-px h-0.5 rounded-full bg-[color:var(--color-leaf)]"
                    aria-hidden="true"
                  />
                )}
              </button>
            );
          })}
        </nav>

        <div
          className="flex-1 overflow-y-auto px-4 py-4"
          role="tabpanel"
          id={`config-panel-${activeId}`}
          aria-labelledby={`config-tab-${activeId}`}
        >
          {activeTab?.content}
        </div>

        {showFooter && (
          <SheetFooter className="gap-2">
            <Button variant="outline" onClick={onReset} aria-label={i18n.resetAriaLabel}>
              {i18n.resetLabel}
            </Button>
          </SheetFooter>
        )}
      </SheetContent>
    </Sheet>
  );
}
