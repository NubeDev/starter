# Feedback Loop — Develop & Fix Nexus with AI

> The point of this whole suite: an AI session runs the stack, something
> misbehaves, and the session can **diagnose and fix Nexus** with consistent
> evidence — then prove the fix by re-running.

The loop:

```
   ┌─────────────────────────────────────────────────────────────┐
   │  1. RUN a scenario/feature runbook (00_setup + features/)     │
   │  2. A ✅ check fails  ──────────────────────────────────────┐ │
   │  3. CAPTURE evidence bundle      → CAPTURE.md                │ │
   │  4. TRIAGE symptom → root cause  → TRIAGE.md                 │ │
   │  5. FIX Nexus (smallest change)  → FIX_LOOP.md               │ │
   │  6. RE-RUN the failing check + its scenario                 │ │
   │  7. RECORD before/after in the feature doc's "Known issues" ─┘ │
   └─────────────────────────────────────────────────────────────┘
```

Three docs:

- **[CAPTURE.md](CAPTURE.md)** — produce the standard evidence bundle. Same
  inputs every time so triage is mechanical, not improvised.
- **[TRIAGE.md](TRIAGE.md)** — symptom → likely-cause table + the checks that
  confirm each. The fast path from "broken" to "here's why".
- **[FIX_LOOP.md](FIX_LOOP.md)** — the AI working contract: how to change Nexus
  safely, keep tests green, and verify the fix didn't paper over the symptom.

Evidence lands in `testing/.evidence/<scenario>/<timestamp>/` (git-ignored).
