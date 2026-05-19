// Minimal shadcn/ui-inspired components. No external deps — just
// className-based styling with CSS variables matching shadcn's design.

import { forwardRef, type ButtonHTMLAttributes, type InputHTMLAttributes, type HTMLAttributes } from "react";

// ---------- cn helper ----------
function cn(...classes: (string | false | undefined | null)[]) {
  return classes.filter(Boolean).join(" ");
}

// ---------- Button ----------
export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "secondary" | "outline" | "ghost" | "destructive";
  size?: "default" | "sm" | "lg" | "icon";
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = "default", size = "default", ...props }, ref) => (
    <button
      ref={ref}
      className={cn("ui-btn", `ui-btn--${variant}`, `ui-btn--${size}`, className)}
      {...props}
    />
  ),
);
Button.displayName = "Button";

// ---------- Input ----------
export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input ref={ref} className={cn("ui-input", className)} {...props} />
  ),
);
Input.displayName = "Input";

// ---------- Card ----------
export function Card({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("ui-card", className)} {...props} />;
}
export function CardHeader({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("ui-card-header", className)} {...props} />;
}
export function CardTitle({ className, ...props }: HTMLAttributes<HTMLHeadingElement>) {
  return <h3 className={cn("ui-card-title", className)} {...props} />;
}
export function CardDescription({ className, ...props }: HTMLAttributes<HTMLParagraphElement>) {
  return <p className={cn("ui-card-desc", className)} {...props} />;
}
export function CardContent({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("ui-card-content", className)} {...props} />;
}

// ---------- Badge ----------
export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  variant?: "default" | "secondary" | "outline" | "destructive" | "success";
}
export function Badge({ className, variant = "default", ...props }: BadgeProps) {
  return <span className={cn("ui-badge", `ui-badge--${variant}`, className)} {...props} />;
}

// ---------- Separator ----------
export function Separator({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("ui-separator", className)} role="separator" {...props} />;
}

// ---------- Tabs ----------
export function Tabs({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("ui-tabs", className)} {...props} />;
}
export function TabsList({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("ui-tabs-list", className)} role="tablist" {...props} />;
}
export interface TabsTriggerProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  active?: boolean;
}
export function TabsTrigger({ className, active, ...props }: TabsTriggerProps) {
  return (
    <button
      role="tab"
      aria-selected={active}
      className={cn("ui-tabs-trigger", active && "ui-tabs-trigger--active", className)}
      {...props}
    />
  );
}
