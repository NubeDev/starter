# Starter changes — Phase 3 gates

Dashboards (Goal 1) + flow-programmer (Goal 3) gates. Two tool
crates land upstream; rubix consumes them.

See [README.md](./README.md) for the index and per-item format.

## `starter-tool-sdui` — page-builder primitives

- **Crate:** `starter-tool-sdui` (new; matches existing
  `starter-tool-github`, `starter-tool-slack` pattern)
- **Blocks rubix phase:** 3 (Goal 1 dashboards)
- **Why upstream:** any starter consumer building dashboards via
  SDUI wants this; it's not rubix-specific.
- **Status:** planned
- **Notes:** if upstream review takes too long, primitives stay in
  `rubix-tools::dashboard::sdui_primitives` with a tracking issue.
  *Never* "we'll do it later" without an issue link.

## `starter-tool-flow-ops` — deploy / validate / lint / list

- **Crate:** `starter-tool-flow-ops` (new)
- **Blocks rubix phase:** 3 (Goal 3 flow-programmer)
- **Why upstream:** every starter consumer with `starter-flow`
  wants flow ops surfaced as tools. This is the cleanest example
  of "rubix's tools are mostly reusable."
- **Status:** planned
- **Notes:** rubix consumes; same fallback rules as above.
