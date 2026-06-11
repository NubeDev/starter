# Reusable, Identity-Scoped Pages (consumer + EMS product scenarios)

**Status:** design proposal — downstream product context. Companion to
[setup-automation-builder.md](setup-automation-builder.md) (split out of its §12A
on peer-review advice). **Not** core automation-builder scope.
**Owner:** ap@nube-io.com
**Date:** 2026-06-11

> **Read this scope marker first.** This doc depends on the **Nexus** product
> layer (WS-13 nav/context, query-kinds, `$caller_*` host tokens) and the core
> [setup-automation-builder](setup-automation-builder.md) Template/Run engine.
> It adds **one** core change (`$caller_team_ids`, tracked as **P3a** in the
> parent doc) plus per-extension product code — none of which the core
> Template/Run builder needs to ship. Treat its build-plan items as a **separate
> product workstream** layered on top.

---

## 1. The problem

Two product scenarios both need **one page reused across many users — never a
page-per-user — where each viewer sees only their own data** (or their
team's/site's). This is a *different reuse axis* from the one
[`WS-13`](../nexus/docs/scope/nextgen/WS-13_NAV_AND_CONTEXT.md) already shipped,
and most of it already exists. The key is to use the right mechanism for each
axis.

### Two reuse axes — don't conflate them

| Axis | "Scope comes from…" | Mechanism | Authored by | Scales to |
|------|--------------------|-----------|-------------|-----------|
| **Place-based** (WS-13, landed) | an admin-authored nav node (`{building: b1}`) | `context` VariableKind (`nav`/`url`/`tag`/`values`) + per-node grant | an admin, one node per place | tens–hundreds of places |
| **Identity-based** (these cases) | **who is logged in** (their user id / teams / site) | un-spoofable `$caller_*` host tokens in the query-kind SQL | nobody — it's automatic | **millions of users**, zero per-user authoring |

The mistake to avoid: **do not create a nav node or an access grant per
consumer.** A million consumers do not get a million nav nodes. They get **one**
tenant-scoped "My Energy" page whose query-kind filters by
`$caller_user_id` — the data isolation *is* the query, not a per-user grant.

## 2. What already exists (verified)

- **Reusable page, many mounts, per-node access** — WS-13 nav tree +
  `context` VariableKind + `nexus.nav_node` grants. Landed on `nexus-gaps`.
- **Server-enforced, client-un-spoofable identity in queries** — the host
  tokens `$caller_tenant_id` and `$caller_user_id`
  ([`query/bind/context.rs`](../nexus/backend/crates/nexus-store/src/query/bind/context.rs),
  [`vars.rs:33`](../nexus/backend/crates/nexus-store/src/query/bind/vars.rs#L33)).
  Bound from the verified `Principal`; **rejected if a caller tries to supply
  them**. Extension query-kinds that read real tables are *already required* to
  scope by `$caller_tenant_id`
  ([EXTENSIONS.md §3](../nexus/testing/docs/EXTENSIONS.md)).
- **Tenant data isolation** — Postgres RLS + FORCE RLS on every tenant table.
- **Ownership + team authz conditions** — `starter-authz` (`owner`,
  `contains principal.teams`); `Principal` already carries `teams`.
- **The extension surface for a reusable page** — a `main`-slot federation
  component that calls `POST /api/v1/query` with the extension's query-kind
  ([EXTENSIONS.md §6–§7](../nexus/testing/docs/EXTENSIONS.md)).

## 3. The gaps (small, well-scoped — the "update the core if needed")

1. **`$caller_team_ids` host token (and optionally `$caller_tenant_scope`)** —
   *the one real core change.* Mirror the existing `$caller_user_id` /
   `$caller_tenant_id` in
   [`query/bind/{context,vars,scan}.rs`](../nexus/backend/crates/nexus-store/src/query/bind/),
   bound from `Principal.teams`, equally un-spoofable. This unlocks "a team lead
   sees the **team's** meters" and the electrician's site-as-team scope. Without
   it, only own-row (`$caller_user_id`) and whole-tenant (`$caller_tenant_id`)
   filters exist — no middle "my team/site" tier. *Tracked as **P3a** in the
   parent build plan.*
2. **A `principal` source on the `context` VariableKind** — *optional nicety.*
   Lets a no-SQL `context` variable expose user/team to a panel without writing
   a query. The `$__user` built-in already exists UI-side; the real enforcement
   is the host token, so this is convenience, not security. Add only if authors
   want it. *(Parent Q7.)*
3. **Entitlement provisioning on purchase/invite** — *new product code, not an
   engine gap.* On "buy a meter" / "redeem an EMS code": create the meter row
   **owned by the buyer** (and tagged to a site/team), and — for B2B — bind the
   invited user to a team(=site) with a narrowed role. No per-user page or grant.
4. **"Site" as a first-class concept** — *a decision, not new infra.*
   **Recommendation: site == team.** A site is a `TeamRecord`; the electrician is
   a member of that team; `$caller_team_ids` scopes their page and the meters
   they add are tagged with the site. This reuses teams, the team token, and
   WS-13 nav verbatim — no new tenancy tier. (Alternative: site == sub-tenant,
   using the existing tenant hierarchy + `tenant_scope`; heavier, only if sites
   need full data isolation from each other rather than just filtered views.)
   *(Parent Q6 — decide before P5.)*

So: **one core addition** (the team token), **one decision** (site == team),
**one piece of product code** (entitlement on purchase). Everything else is
already there.

---

## 4. Example A — Consumer power-meter product (self-service, identity-scoped)

**Extension `com.acme.power`** (builtin or process): contributes a
`high-usage-alert` automation template, a `my-energy` query-kind, an
`energy-report` insight, and a `main`-slot "My Energy" page. Sold as a consumer
product; thousands of unrelated buyers share one tenant.

1. **Buy + onboard.** Customer buys a meter; signs up (`POST /auth/signup`) or
   attaches to an existing account. **Entitlement (new product code):** create
   the `meter` row with `owner_user_id = principal.subject`. *No nav node, no
   per-user grant* — ownership is a column, isolation is the query.
2. **Access "just to this."** The "My Energy" page is **one** nav node granted
   at **tenant scope** to all customers (WS-13). What each customer *sees* is
   bounded by the query-kind, not the grant:
   ```sql
   -- kinds/my-energy.sql  (contributed query-kind; tables: [meter_readings])
   SELECT bucket, kwh
   FROM   meter_readings
   WHERE  owner_user_id = $caller_user_id      -- server-bound, un-spoofable
     AND  $__timeFilter(ts)
   ```
   A customer cannot deep-link `?owner=someone-else` — `$caller_user_id` comes
   from their verified session, never the request body.
3. **High-usage alert.** The extension's `high-usage-alert` template is a flow:
   `read-usage → threshold-gate → notify`. Instantiated per meter at purchase
   (owner-scoped); fires through the existing alert/notify path.
4. **"See other users' data" (team/household/fleet view).** A customer who is a
   **team lead** (e.g. a household owner, or a building manager over many
   meters) opens the **same page**; their role's query-kind variant widens the
   filter to the team token:
   ```sql
   WHERE owner_user_id = $caller_user_id
      OR site_team = ANY($caller_team_ids)     -- ← needs the new team token (gap #1)
   ```
   Same page, same nav node — the *viewer's identity* changes what rows return.
   Regular customers (no team) get only their own; leads get the team's. This is
   the "reuse the page, access based on their data" requirement, satisfied
   without a second page or a per-user anything.

**Net for Example A:** ships today for the single-user view (`$caller_user_id`
already exists); the household/fleet view needs only the team token (gap #1).

---

## 5. Example B — BMS/EMS company → customer site → onsite electrician (B2B)

**Extension `com.acme.ems`**: contributes the `add-power-meter` setup automation
(the barcode flow — parent doc §6/§12) and a `site-checkout` reusable
verification page. The EMS company operates the platform; each customer is a
tenant (or a team — see below); each customer site has onsite electricians with
a **single allowed action**.

1. **Issue scoped access (new product code).** The EMS company sends the
   electrician a **redeem code** bound to `{ tenant: customer-acme, site: ahu-roof,
   role: installer }`. Redeeming it (`POST /auth/signup` + code) creates the user,
   adds them to the **team `ahu-roof` (= the site)**, and assigns the `installer`
   role. Their `Principal` now carries `teams: [ahu-roof]`.
2. **One allowed action.** Authz (`starter-authz`) grants `installer` exactly
   `setup.templates/run` on `com.acme.ems.add-power-meter` — and nothing else.
   The nav tree (WS-13) shows the electrician **only** that one action node and
   the site-checkout page; no dashboards, no admin, no other tenants
   (cross-tenant predicate + per-node grant).
3. **Scan → add meter.** The electrician runs the setup automation exactly as
   parent §12: scan barcode → `202 { run_id }` → SSE progress → resume-on-failure
   (per parent §8b). The `device.create` node **tags the new meter with the
   electrician's site**, reading the **server-seeded trusted slots** (parent §9
   "Trusted identity"), not form input: `site_team = caller_team_ids[0]`
   (`ahu-roof`), `owner_tenant = caller_tenant_id`. Because those slots are
   written from the verified `Principal` at run start and templates cannot bind
   over them, the electrician can't spoof another site. Provisioned hardware is
   site-scoped at creation time.
4. **"Test/check it works" — the reusable site page.** The electrician opens
   **`site-checkout`**, one page reused for every site, scoped by the team token:
   ```sql
   -- kinds/site-checkout.sql  (tables: [meter_readings, meters])
   SELECT m.serial, m.installed_at, r.last_seen, r.kwh_last_hour
   FROM   meters m
   LEFT   JOIN meter_latest r ON r.meter_id = m.id
   WHERE  m.site_team = ANY($caller_team_ids)   -- ← the new team token (gap #1)
     AND  m.owner_tenant = $caller_tenant_id    -- existing, defence-in-depth
   ```
   The electrician sees every meter they installed **at this site** — live
   readings confirm each is reporting — and nothing from other sites or
   customers. The EMS company's own staff, members of all site teams (or
   tenant-admins using `$caller_tenant_id`), see the whole customer estate
   through the *same page*.

**Net for Example B:** the automation half ships on the parent §13 plan; the
verify page needs the team token (gap #1) and the site==team decision (gap #4).
The "single allowed action, tightly scoped" property is pure authz + WS-13 nav,
already there.

---

## 6. Where this lands in the build plan

- The **team host token** (gap #1) is a small, self-contained core change —
  it is **P3a** in the parent build plan, alongside the authz work; it is the
  only item that touches shipped query internals, so spec + test it carefully
  (un-spoofable, rejected in caller-supplied position, RLS still the backstop).
- **Entitlement provisioning** (gap #3) is per-extension product code — it lives
  in `com.acme.power` / `com.acme.ems`, built in **P5** (the extension seam).
- The **site==team decision** (gap #4, parent Q6) should be ratified before P5
  since the setup automation's tagging and the verify page both depend on it.
- The **reusable pages themselves** are `main`-slot federation components +
  contributed query-kinds — standard extension work (P6 for the builder; the
  pages here can ship as hand-authored kinds earlier).
