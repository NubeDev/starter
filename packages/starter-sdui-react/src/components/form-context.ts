/**
 * Form context — shared between `form` and the input components
 * (`field`, `select`, `toggle`). Holds the per-form values map, the
 * setter, and the diagnostics list so each control can render the
 * subset of diagnostics scoped to its `name`.
 */
import { createContext, useContext } from "react";
import type { Diagnostic } from "../types.js";

export interface FormCtx {
  values: Record<string, unknown>;
  setField: (name: string, value: unknown) => void;
  diagnostics: Diagnostic[];
}

export const FormContext = createContext<FormCtx | null>(null);

export function useFormCtx(): FormCtx | null {
  return useContext(FormContext);
}
