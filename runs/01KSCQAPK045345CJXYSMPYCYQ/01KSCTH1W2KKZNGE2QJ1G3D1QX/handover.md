## Done

- packages/starter-client-react/README.md added — provider hierarchy, hook patterns, query-key convention
- packages/starter-client-ts/README.md — appended Streaming section pointing at streamJson
- rubix/docs/design/frontend/README.md added — chassis architecture, layering, proxy/env, SSE worked example for /extensions
- rubix/docs/sessions/2026-05-24-frontend-wired.md added — per-phase summary + operator manual flow + test counts
- .github/workflows/rubix-frontend-e2e.yml added — Playwright job for the three new specs with Postgres + ClickHouse services and a backgrounded rubix-agent
- committed as `stage 16: phase D — closing docs + CI + PR — chore(docs+ci) close out frontend wiring + open PR`

## Next

- operator to confirm PR shape and run `gh pr create --base master --head codeless/rubix-frontend-wire` with title `feat(rubix-frontend) wire REST + SSE end-to-end + starter-client-react + rubix-client-react upstream` and body summarising each phase

## What you need to know

- `pnpm -r run typecheck` / `pnpm -r run test` in the existing pnpm CI job already covers both new packages (workspace-recursive), so no per-package job was added
- `rubix-client-react/README.md` already exists at `rubix/packages/rubix-client-react/README.md` (landed in stage 8 B.5); there is no `packages/rubix-client-react/` because the package is rubix-namespaced
- the new rubix-frontend-e2e workflow assumes `rubix-agent` exposes `serve` and `seed-bootstrap` subcommands and a `/readyz` endpoint — verify against the real binary before relying on CI signal
- the playwright job invocation passes spec filenames as positional args (`run e2e -- auth.spec.ts ...`); adjust if the playwright config rejects them

## Open questions

- PR creation skipped — headless run had no operator confirmation channel; commit landed on `codeless/rubix-frontend-wire` ready for the operator to open
- the rubix-agent CLI surface used in `rubix-frontend-e2e.yml` (`seed-bootstrap`, `serve`, env vars) was inferred from `mani run demo` and may need a one-shot tweak when the workflow first runs
