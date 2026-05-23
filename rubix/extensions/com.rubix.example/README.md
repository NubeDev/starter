# `com.rubix.example` — reference extension

This is the layout template for third-party rubix extensions.
A real block:

1. Implements its handlers (`EchoTool` in this example) using the
   planned `rubix-extensions-sdk` crate. Per SCOPE R8 the SDK is
   the **only** rubix surface an extension depends on.
2. Ships its `block.yaml` declaring what it contributes
   (`tools` / `skills` / `flows` / `nodes`).
3. Lets the host load it via the planned `starter-ext-flow`
   adapter (see [STARTER-CHANGES.md](../../docs/design/STARTER-CHANGES.md)).

Until those upstream pieces land, this directory is structure-only
— `process/src/main.rs` does not compile because the SDK doesn't
exist yet. Phase 5 (per SCOPE) is when the full end-to-end works.

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
