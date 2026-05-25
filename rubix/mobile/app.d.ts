// Ambient module declarations so TypeScript accepts SQL imports that
// metro/babel resolve at runtime via `@expo/metro-config`. The SQL strings
// are loaded by the migrations index — keeping them as `.sql` files (not
// `.ts` string literals) keeps them grep-able and snapshot-able alongside
// the agent-side migrations folder layout.

declare module '*.sql' {
  const content: string;
  export default content;
}
