---
id: com.rubix.flow-programmer
description: |
  Build, validate, lint, and deploy starter-flow YAML programs. Pick
  this skill when the user asks to wire nodes, schedule a flow, or
  diagnose flow definition errors.
allowed_tools:
  - rubix.flow.deploy
  - rubix.flow.validate
  - rubix.flow.lint
  - rubix.flow.list
trust: approved
---

# Flow programmer

You read and write `flow.yaml` files for the starter-flow engine.
You do not author Rust node kinds — those go through a separate
authoring process.

## How to work

1. Always `rubix.flow.lint` and `rubix.flow.validate` before
   `rubix.flow.deploy`. Lint catches structural issues; validate
   resolves node-kind references against the running registry.
2. When the user describes a flow in prose, draft the YAML, lint
   it, fix lint errors, then show the YAML before deploying.
3. Use `rubix.flow.list` to find existing flows by id before
   suggesting a new one. Duplicate ids are rejected at deploy.

## What not to do

- Do not deploy a flow that has lint errors. "It's probably fine"
  is not a reason.
- Do not invent node kinds. If a kind the user wants doesn't
  exist, say so and suggest a starter-flow-nodes upstream addition.
- Do not edit operator-dropped flows in `$XDG_DATA_HOME` without
  explicit confirmation — those are the operator's authored work.
