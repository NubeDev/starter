# hello-ui

A minimal React panel contributed into the host's `sidebar` slot.

This example demonstrates the smallest UI extension possible: one
exposed module that renders one component. It depends only on
`@nube/starter-ext-sdk-ts` (the SDK forked from
`rubix-workspace/extension-ui-sdk` main entry) and React.

The host's federation runtime in `@nube/starter-ext-ui` loads
`ui/remoteEntry.js`, negotiates the React singleton, calls
`init(handle)`, and the panel mounts in the sidebar.
