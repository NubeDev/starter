import type { Actor, Op } from "@/api/types";

// A short human label for who caused a change. Agents read as their model so an
// "AI did this" row is legible at a glance; system actions are unattributed.
export function actorLabel(actor: Actor): string {
  switch (actor.kind) {
    case "user":
      return actor.subject;
    case "agent":
      return `agent · ${actor.model}`;
    case "system":
      return "system";
  }
}

// The operation as a lowercase verb. `Op` is a string enum plus a `{ custom }`
// object variant for kind-specific verbs; render the custom name when present.
export function opLabel(op: Op): string {
  if (typeof op === "string") return op;
  if (op && typeof op === "object" && "custom" in op) {
    return String((op as { custom: string }).custom);
  }
  return "change";
}
