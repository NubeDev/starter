# `com.rubix.example` — reference extension

This is the layout template for third-party rubix extensions.
A real block:

1. Implements its handlers (`EchoTool` in this example) using the
   upstream `starter-ext-sdk` crate. Per SCOPE R8 the SDK is the
   **only** rubix surface an extension depends on.
2. Ships its `block.yaml` declaring what it contributes
   (`tools` / `skills` / `flows` / `nodes`).
3. Lets the host load it via the upstream `starter-ext-flow`
   adapter — `rubix-agent`'s `boot::extensions_flow` composer
   walks the sealed `ExtensionRegistry` and folds every
   `contributes.nodes[]` entry into the live `NodeKindRegistry`
   (slice A binds the placeholder behaviour that returns
   `NodeError::Domain { code: "no_behaviour_bound" }` on invoke;
   slice B's `ProcessNodeProxy` swap is owned upstream per
   `starter-extensions/DOCS/extensions/scope/FLOW-NODES.md`).

## Layout

```
com.rubix.example/
├── block.yaml                       contributions declaration
├── kinds/                           (empty — add kind YAMLs here)
├── process/src/                     the extension binary
├── skills/
│   └── example-skill/SKILL.md       extension-shipped skill
│                                    (quarantined by default per
│                                    starter agent SCOPE R4)
└── flows/
    └── example-assistant.yaml       extension-shipped flow
```
