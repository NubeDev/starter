// Minimal JSON-Schema (draft-07) shape helpers used by the puck
// config generator. We only model the keywords schemars emits for
// the starter-ui-ir crate, not the full draft-07 spec.

export interface JsonSchema {
  $schema?: string;
  title?: string;
  description?: string;
  type?: string | string[];
  enum?: unknown[];
  default?: unknown;
  format?: string;
  properties?: Record<string, JsonSchema>;
  required?: string[];
  additionalProperties?: boolean | JsonSchema;
  items?: JsonSchema | JsonSchema[];
  oneOf?: JsonSchema[];
  anyOf?: JsonSchema[];
  allOf?: JsonSchema[];
  $ref?: string;
  definitions?: Record<string, JsonSchema>;
  minimum?: number;
  maximum?: number;
}

/** `#/definitions/Foo` → `"Foo"`. Throws on unknown ref shape. */
export function refName(ref: string): string {
  const prefix = "#/definitions/";
  if (!ref.startsWith(prefix)) {
    throw new Error(`[schema-walker] unsupported $ref: ${ref}`);
  }
  return ref.slice(prefix.length);
}

/** Resolve a `$ref` against the schema's `definitions`. */
export function resolveRef(root: JsonSchema, ref: string): JsonSchema {
  const name = refName(ref);
  const def = root.definitions?.[name];
  if (!def) {
    throw new Error(`[schema-walker] missing definition: ${name}`);
  }
  return def;
}

/**
 * Flatten the schemars "oneOf-inside-allOf" / "anyOf-with-null"
 * wrappers schemars emits, returning the underlying typed schema.
 *
 * Per scope §B1: "Schemars output is not a clean discriminated
 * union." Specifically the patterns we collapse here are:
 *
 *   - `{ allOf: [{ $ref }] }`            → resolve the $ref.
 *   - `{ anyOf: [X, { type: "null" }] }` → return X (the non-null arm).
 *   - `{ anyOf: [{ $ref }, { type: "null" }] }` → resolve the $ref.
 *
 * Everything else passes through unchanged. We DO NOT collapse a
 * generic top-level `oneOf` — that's the Component discriminated
 * union the caller wants to walk arm-by-arm.
 */
export function flatten(root: JsonSchema, schema: JsonSchema): JsonSchema {
  let cur = schema;
  // Bounded loop — pathological schemas with deep wrapping would
  // otherwise spin; ten levels is more than schemars ever emits.
  for (let i = 0; i < 10; i += 1) {
    if (cur.allOf && cur.allOf.length === 1 && cur.allOf[0]) {
      cur = cur.allOf[0];
      continue;
    }
    if (cur.anyOf && cur.anyOf.length === 2) {
      const [a, b] = cur.anyOf;
      const nullArm = (s: JsonSchema | undefined) => s && s.type === "null";
      if (a && nullArm(b)) {
        cur = a;
        continue;
      }
      if (b && nullArm(a)) {
        cur = b;
        continue;
      }
    }
    if (cur.$ref) {
      cur = resolveRef(root, cur.$ref);
      continue;
    }
    break;
  }
  return cur;
}

/** Snake-case `type` discriminator value from a Component oneOf arm. */
export function variantTypeOf(arm: JsonSchema): string | undefined {
  const typeProp = arm.properties?.["type"];
  if (!typeProp || !typeProp.enum || typeProp.enum.length === 0) return undefined;
  const v = typeProp.enum[0];
  return typeof v === "string" ? v : undefined;
}
