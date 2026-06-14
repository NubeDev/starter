// Component catalog: the lists that drive the builder forms.

import { getJson } from "./client";

export type FieldKind = "text" | "number" | "duration" | "bool" | "code" | "list";

export interface Field {
  name: string;
  kind: FieldKind;
  required: boolean;
  placeholder: string | null;
  help: string | null;
}

export interface ComponentKind {
  type: string;
  label: string;
  summary: string;
  fields: Field[];
}

export interface PluginEntry {
  type: string;
  category: string;
  source: "builtin" | "custom";
}

export const fetchInputs = () => getJson<ComponentKind[]>("/api/inputs");
export const fetchOutputs = () => getJson<ComponentKind[]>("/api/outputs");
export const fetchProcessors = () => getJson<ComponentKind[]>("/api/processors");
export const fetchBuffers = () => getJson<ComponentKind[]>("/api/buffers");
export const fetchPlugins = () => getJson<PluginEntry[]>("/api/plugins");
