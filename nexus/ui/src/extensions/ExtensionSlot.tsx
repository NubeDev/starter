// Host glue for extension mount points. Re-exports the federation
// runtime's slot so feature code imports a Nexus-local path and the
// `@nube/starter-ext-ui` dependency stays an implementation detail of
// the `extensions/` folder.
export { ExtensionSlot, type ExtensionSlotProps } from "@nube/starter-ext-ui";
