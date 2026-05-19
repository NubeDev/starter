// Barrel of endpoint modules. Each module augments `StarterClient`
// with the methods for one endpoint family — mirrors the Rust
// client's per-endpoint files.

export * from "./health.js";
export * from "./auth.js";
export * from "./openapi.js";
