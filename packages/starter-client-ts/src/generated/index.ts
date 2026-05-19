// Codegen output lives here. **Do not hand-edit** — `pnpm codegen`
// regenerates this directory from the server's OpenAPI document.
// SCOPE.md R7 forbids hand-edited TS wire types.

export type Problem = {
  type: string;
  title: string;
  detail?: string;
};

// TODO(codegen): the rest of the wire types are generated on first
// codegen run. This stub keeps the package compilable beforehand.
