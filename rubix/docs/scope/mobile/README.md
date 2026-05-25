# Mobile — scope

**Goal:** ship a React Native app (iOS + Android) that looks and
behaves like [`rubix/frontend`](../../../frontend/) for the subset
of the surface that makes sense on a phone: **dashboards and
server-driven UI**. No flow editor, no warehouse admin, no
extensions admin.

**Status:** plan only. Nothing scaffolded yet. Promote each file
in this folder to `docs/design/mobile/` as the corresponding code
lands (per [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md#0a--doc-tiers-and-what-code-may-reference)).

## Why now

The TS chassis already isolates the DOM at exactly two seams:

1. `@nube/starter-ui-kit` — Radix + Tailwind primitives.
2. `@nube/starter-ui-sdui-react` — per-IR-kind `render-*.tsx`
   files (in `src/renderer/`) that import those primitives. The
   mobile plan requires this package to expose a `./headless`
   subpath (registry + provider + hooks, no renderers) so the
   native renderers can register against the same surface — see
   [REUSE.md](./REUSE.md#reused--sdui-after-a-package-split-blocker).

Everything below those seams (HTTP clients, IR types, auth, i18n,
preferences, theme **state**, SDUI **orchestration**) is already
DOM-free. The mobile app reuses those layers verbatim and replaces
only the two web-only ones with native counterparts.

This is the same shape as [`docs/design/frontend/README.md`](../../design/frontend/README.md)
— same layered chassis, different leaves.

## Contents

- [REUSE.md](./REUSE.md) — what mobile pulls in unchanged from
  `packages/` and `rubix/packages/`. The list is exhaustive; if
  a package isn't there, mobile doesn't depend on it.
- [NEW-PACKAGES.md](./NEW-PACKAGES.md) — the four new workspace
  packages mobile needs (`starter-theme-tokens`,
  `starter-ui-kit-native`, `starter-ui-sdui-native`,
  `starter-ui-dashboard-native`). One file per package would be
  premature — they are one decision.
- [APP-SHELL.md](./APP-SHELL.md) — the `rubix/mobile/` Expo app
  layout: provider stack, navigation, env, storage, auth strategy.
- [LOCAL-DB.md](./LOCAL-DB.md) — the on-device SQLite store for
  saved connections to remote rubix-agent servers (the app is
  multi-instance: one phone, many agents).
- [THIN-SLICE.md](./THIN-SLICE.md) — the first milestone: a
  single dashboard (`dashboard.disk-overview`) rendered end-to-end
  through every layer. Five PRs, mirrors the discipline of the
  backend [THIN-SLICE.md](../THIN-SLICE.md).
- [NON-GOALS.md](./NON-GOALS.md) — what the mobile app explicitly
  does **not** ship, so reviewers can reject scope creep by name.

## Promotion path

Each file here promotes to a sibling under `docs/design/mobile/`
once the implementation it describes is in `master`:

| Scope file | Becomes | Trigger |
|---|---|---|
| `REUSE.md` | `docs/design/mobile/dependencies.md` | First green CI on `rubix/mobile` |
| `NEW-PACKAGES.md` | `docs/design/mobile/packages.md` | First version of all four new packages published from `master` |
| `APP-SHELL.md` | `docs/design/mobile/app-shell.md` | Provider stack stable |
| `LOCAL-DB.md` | `docs/design/mobile/local-db.md` | `src/local-db/` on master with schema + provider wired in |
| `THIN-SLICE.md` | — | Slice exit gates #1–#5 met (see [THIN-SLICE §Exit gate](./THIN-SLICE.md#exit-gate)); file deleted once the slice ships. Each block's design lands in the appropriate `docs/design/mobile/` file. |
| `NON-GOALS.md` | `docs/design/mobile/non-goals.md` | Carries forward as-is |

The ADR that justifies the seam choice and the Expo decision lives
at [`docs/adr/0004-react-native-mobile-app.md`](../../adr/0004-react-native-mobile-app.md).

## Backend prerequisites

The mobile plan is not self-contained — two backend changes must
land before any mobile code starts:

1. **Bearer-token issuance from credentials — LANDED.**
   `POST /api/v1/auth/token` in `starter-auth-users` accepts
   `{ email, password, tenant_id? }` and returns `{ token,
   expires_at, token_type: "Bearer" }`. Bearer **acceptance** was
   already in place via
   [`crates/starter-auth-users/src/principal_layer.rs`](../../../../crates/starter-auth-users/src/principal_layer.rs);
   this route is the missing **issuance** counterpart so mobile,
   native-desktop, and CLI clients can sign in without cookies.
   Location decision (live in `starter-auth-users`, not just
   rubix-agent) and full payload + error contract:
   [`docs/design/auth/token-issuance.md`](../../design/auth/token-issuance.md).
2. **`@nube/starter-ui-sdui-react` `./headless` subpath.** Splits
   the registry / provider / hooks / transport (DOM-free) away from
   the renderers (DOM-bound) so the native renderer package can
   register against the same surface without bundling React-DOM
   transitively. Tracked in
   [REUSE.md](./REUSE.md#reused--sdui-after-a-package-split-blocker)
   and [NEW-PACKAGES §Precondition](./NEW-PACKAGES.md#precondition--sdui-react-package-split).
