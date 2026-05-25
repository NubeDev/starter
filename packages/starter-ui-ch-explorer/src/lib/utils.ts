// Forked from sql-studio (MIT) — https://github.com/frectonz/sql-studio
// Original copyright (c) frectonz. See NOTICES.md.

import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
