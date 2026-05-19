// Re-exports for the auth surface.

export { AuthProvider, useAuth } from "./provider.js";
export type { AuthContextValue, AuthProviderProps, AuthStatus } from "./provider.js";
export { sessionStrategy, tokenStrategy, externalStrategy } from "./strategy.js";
export type { AuthStrategy, LoginInput } from "./strategy.js";
