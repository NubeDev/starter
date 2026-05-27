# Mobile — non-goals

Things the mobile app explicitly does **not** ship. Listed by
name so reviewers can reject scope creep with a one-line link.

## Surfaces

| Excluded surface | Where it lives on web | Reason |
|---|---|---|
| Flow editor | [`rubix/frontend/src/routes/flows/`](../../../frontend/src/routes/flows/) | Pan/zoom node graph; not a phone-shaped interaction. |
| Warehouse admin | [`rubix/frontend/src/components/admin/warehouse/`](../../../frontend/src/components/admin/warehouse/) | Data-heavy admin; operators use the web app for this. |
| Extensions admin | [`rubix/frontend/src/routes/extensions.tsx`](../../../frontend/src/routes/extensions.tsx) | Same as above. |
| Tenant / team / authz admin | [`@nube/starter-ui-authz`](../../../../packages/starter-ui-authz/) | Same as above. |
| Print / export to PDF | [`@nube/starter-ui-export`](../../../../packages/starter-ui-export/) | DOM-only stack (`html2canvas` + `jspdf`). If mobile needs export, that's a separate share-sheet feature with its own ADR. |

## Technical

- **No web bundle.** `rubix/mobile` ships native binaries via Expo
  EAS. We don't add Expo Web — that would muddy the seam with the
  existing Vite SPA.
- **No code-push / OTA before launch.** Expo Updates may come
  later, but the slice ships through the store review pipeline so
  we measure the real-world boot path.
- **No biometric unlock in v1.** A bearer token in
  `expo-secure-store` is the baseline; biometrics is a follow-up
  ADR after the slice ships.
- **No offline cache in v1.** React-Query cache is in-memory.
  Persistence is a follow-up once we understand which pages
  operators actually use on flaky connections.
- **No push notifications in v1.** SSE on a foregrounded screen
  is the slice; APNs / FCM is a separate workstream.
- **No bearer-token refresh in v1.** On 401 the app evicts the
  token, preserves `last_opened_page_ref`, and routes back to
  per-connection login (see [APP-SHELL.md](./APP-SHELL.md#strategy)).
  Refresh tokens are a separate auth-crate workstream.
- **No self-signed TLS / cert pinning in v1.** The mobile app
  trusts the system root store only. **Deployment assumption,
  stated explicitly:** operators with home-LAN rubix-agent
  instances (e.g. `https://rubix.local:8088` with a self-signed
  cert) are expected to front them with Tailscale, Cloudflare
  Tunnel, Let's Encrypt + a real DNS name, or another path that
  presents a system-trusted cert. **Mobile will not work against
  a bare self-signed cert in v1, and that is a known UX gap for
  the home-lab audience.** Per-connection pinning is a follow-up
  whose schema hook (`tls_pinned_fingerprint`) is already
  reserved in [local-db.md](../../design/mobile/local-db.md#deferred-not-in-v1).
  *This is an open call: if home-lab UX matters more than v1
  ship date, TLS pinning gets promoted to Block 0.* See
  question to the author in the README.
- **No server discovery (mDNS / Bonjour / QR-pairing) in v1.**
  Add a server by typing URL + label. Forward compatibility:
  `connections/new.tsx` URL parsing strips an optional `?pair=`
  query string and treats the rest as a plain `base_url`, so a
  future QR-pairing flow can encode its handshake in the same
  surface without breaking today's manual add.
- **No deep links into a specific connection's dashboard in v1.**
  Expo Router supports it; the per-connection resolution UX is a
  follow-up.
- **No second SDUI renderer set.** `@nube/starter-ui-sdui-react` is
  the single renderer. Mobile maintains one renderer surface, period.
- **No bespoke per-page React Native screens beyond the dashboard
  route.** Everything is SDUI-driven; if a screen needs a hand
  layout, that's a sign the IR needs a new kind, not a new screen.
- **No formal accessibility audit gate on the slice.** A formal
  audit (dynamic type, screen reader walkthroughs, contrast
  checks against WCAG AA) is a follow-up after the slice. The
  **per-primitive `accessibilityRole` + `accessibilityLabel`
  contract is NOT a non-goal** — it is a kit acceptance criterion
  enforced at primitive-PR review time. See
  [NEW-PACKAGES §starter-ui-kit-native](./NEW-PACKAGES.md#starter-ui-kit-native).
## Process

- **No "phase 2" markers in code.** Per
  [NEW-SESSION.md §2](../../../NEW-SESSION.md#2--the-non-negotiables),
  comments describe the code as it is now. Excluded surfaces are
  recorded **here**, not as `// TODO: ship in mobile v2` strings
  scattered through the source tree.
- **No mobile-only forks of shared packages.** If mobile needs a
  prop the web doesn't have, it lands on the web component first
  (see [NEW-PACKAGES.md](./NEW-PACKAGES.md#starter-ui-dashboard-native)).
- **No bypassing the import-lint rule.** A `// eslint-disable`
  on the forbidden-imports rule is a PR blocker.
