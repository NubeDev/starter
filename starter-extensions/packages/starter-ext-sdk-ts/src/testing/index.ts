// `@nube/starter-ext-sdk-ts/testing` — helpers extension authors
// import in their unit tests. Kept on a subpath so the production
// bundle never pulls them.

export {
  MockHostProvider,
  type MockHostProviderProps,
} from "./mock-host-provider.js";
