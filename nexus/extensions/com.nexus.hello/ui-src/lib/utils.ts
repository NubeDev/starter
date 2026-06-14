// `cn` — the standard shadcn/ui class-merge helper: clsx for conditional
// classes, tailwind-merge to dedupe conflicting Tailwind utilities (so a later
// `p-4` wins over an earlier `p-2` rather than both landing in the class list).
// Every shadcn primitive in this extension uses it.
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
