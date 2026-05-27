// Primitives the ConfigDrawer's sections share — section heading with
// an optional "reset" affordance, two radio-tile shapes, and a
// "coming soon" wrapper for fields the host hasn't wired up yet.
//
// These are intentionally framework-agnostic: they take strings and
// callbacks as props so the host can i18n / state-manage them however
// it likes. They depend only on Radix RadioGroup and the host's
// shadcn `Button` primitive (which we own in this package).

import { type SVGProps, type ReactNode, type ReactElement, type CSSProperties } from "react";
import { RadioGroup as RadioGroupPrimitive } from "radix-ui";
import { CircleCheck, RotateCcw } from "lucide-react";
import { cn } from "../../lib/utils.js";
import { Button } from "../../components/ui/button.js";

const { Item } = RadioGroupPrimitive;

export interface SectionTitleProps {
  title: string;
  description?: string;
  showReset?: boolean;
  onReset?: () => void;
  /** Accessible label for the reset button. The host should pass an
   * i18n-localized string here. */
  resetAriaLabel?: string;
  className?: string;
}

export function SectionTitle({
  title,
  description,
  showReset = false,
  onReset,
  resetAriaLabel,
  className,
}: SectionTitleProps) {
  return (
    <div className={cn("mb-2", className)}>
      <div className="flex items-center gap-2 text-sm font-semibold text-[color:var(--color-muted-foreground)]">
        {title}
        {showReset && onReset && (
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="size-4 rounded-full"
            onClick={onReset}
            aria-label={resetAriaLabel}
          >
            <RotateCcw className="size-3" />
          </Button>
        )}
      </div>
      {description && (
        <p className="mt-0.5 text-xs text-[color:var(--color-subtle)]">{description}</p>
      )}
    </div>
  );
}

export interface RadioIconTileProps {
  item: {
    value: string;
    label: string;
    icon: (props: SVGProps<SVGSVGElement>) => ReactElement;
  };
  /** When true, the icon is rendered without the `fill-leaf / stroke-leaf`
   * override — used by the theme (light/dark/system) tiles whose icons
   * already carry their own palette. */
  isTheme?: boolean;
  /** Accessible label override. Defaults to `"Select <label>"` — but
   * the host should pass an i18n-localized string when possible. */
  ariaLabel?: string;
}

export function RadioIconTile({ item, isTheme = false, ariaLabel }: RadioIconTileProps) {
  return (
    <Item
      value={item.value}
      className={cn("group outline-none", "transition duration-200 ease-in")}
      aria-label={ariaLabel ?? `Select ${item.label.toLowerCase()}`}
      aria-describedby={`${item.value}-description`}
    >
      <div
        className={cn(
          "relative rounded-[6px] ring-[1px] ring-[color:var(--color-border)]",
          "group-data-[state=checked]:shadow-2xl group-data-[state=checked]:ring-[color:var(--color-leaf)]",
          "group-focus-visible:ring-2",
        )}
        role="img"
        aria-hidden="false"
        aria-label={`${item.label} option preview`}
      >
        <CircleCheck
          className={cn(
            "size-6 fill-[color:var(--color-leaf)] stroke-white",
            "group-data-[state=unchecked]:hidden",
            "absolute top-0 right-0 translate-x-1/2 -translate-y-1/2",
          )}
          aria-hidden="true"
        />
        <item.icon
          className={cn(
            !isTheme &&
              "fill-[color:var(--color-leaf)] stroke-[color:var(--color-leaf)] group-data-[state=unchecked]:fill-[color:var(--color-muted-foreground)] group-data-[state=unchecked]:stroke-[color:var(--color-muted-foreground)]",
          )}
          aria-hidden="true"
        />
      </div>
      <div className="mt-1 text-xs" id={`${item.value}-description`} aria-live="polite">
        {item.label}
      </div>
    </Item>
  );
}

export interface RadioTileProps {
  value: string;
  label: string;
  ariaLabel?: string;
  className?: string;
  style?: CSSProperties;
  children?: ReactNode;
}

export function RadioTile({
  value,
  label,
  ariaLabel,
  className,
  style,
  children,
}: RadioTileProps) {
  return (
    <Item
      value={value}
      className="group outline-none"
      aria-label={ariaLabel ?? `Select ${label.toLowerCase()}`}
    >
      <div
        className={cn(
          "relative rounded-[6px] ring-[1px] ring-[color:var(--color-border)] px-3 py-2 text-center",
          "group-data-[state=checked]:shadow-2xl group-data-[state=checked]:ring-[color:var(--color-leaf)]",
          className,
        )}
        style={style}
      >
        <CircleCheck
          className={cn(
            "size-5 fill-[color:var(--color-leaf)] stroke-white",
            "group-data-[state=unchecked]:hidden",
            "absolute top-0 right-0 translate-x-1/2 -translate-y-1/2",
          )}
        />
        {children}
      </div>
      <div className="mt-1 text-xs">{label}</div>
    </Item>
  );
}

export interface ComingSoonFieldProps {
  label: string;
  description?: string;
  /** Text shown in the "coming soon" pill. Defaults to `"Coming soon"`
   * — host can pass an i18n-localized string. */
  badgeLabel?: string;
  control: ReactNode;
}

export function ComingSoonField({
  label,
  description,
  badgeLabel = "Coming soon",
  control,
}: ComingSoonFieldProps) {
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between gap-2">
        <label className="text-xs font-medium text-[color:var(--color-text)]">{label}</label>
        <span className="rounded-full bg-[color:var(--color-surface-2)] px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-[color:var(--color-subtle)]">
          {badgeLabel}
        </span>
      </div>
      {control}
      {description && (
        <p className="text-[11px] text-[color:var(--color-subtle)]">{description}</p>
      )}
    </div>
  );
}
