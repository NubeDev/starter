# com.nexus.demo

The WS-14 end-to-end example extension for nexus. Builtin flavour (no process
to supervise); its value is the **contributions**:

- **Query-kinds** (`contributes.warehouse_templates[]`): two self-contained
  kinds — `com.nexus.demo.ping` (no params, returns a greeting + the server
  clock) and `com.nexus.demo.echo` (binds a `$message` param). On boot they are
  linted and materialised into `nexus_extension_query_kinds`, becoming the
  query dispatcher's *third source* (file pack → **extension** → tenant
  overlay).
- **UI panel** (`contributes.ui`): a federation entry exposing `HelloPanel`
  into the `sidebar` slot. `ui/remoteEntry.js` is **built from `ui-src/`**
  (vite library mode; React/JSX externalised and resolved to the host's
  instance through the importmap in `nexus/ui/index.html` — same pattern as
  `rubix/extensions/com.rubix.example`). The panel deliberately closes the
  loop: it runs this extension's own contributed kind
  (`com.nexus.demo.ping`) through `useHostClient()` and renders the greeting
  + server time — if it renders, federation load, singleton negotiation, slot
  mounting, cookie auth, and third-source kind dispatch all work.

  Rebuild after editing `ui-src/`:

  ```sh
  pnpm -C nexus/backend/crates/nexus-api/extensions/com.nexus.demo/ui-src build
  ```

  (The `ui-src` is a pnpm workspace member; the built `ui/remoteEntry.js` is
  committed, like the rubix extension bundles, so the backend pack is complete
  without a frontend toolchain.)

## Try it

```sh
# kind-mode query through the contributed kind (third source):
curl -s -X POST :4780/api/v1/query \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"sql":"","kind":"com.nexus.demo.ping"}'

curl -s -X POST :4780/api/v1/query \
  -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"sql":"","kind":"com.nexus.demo.echo","params":{"message":"works"}}'

# admin surface:
curl -s :4780/api/v1/extensions -H "authorization: Bearer $ADMIN_TOKEN"
curl -s :4780/api/v1/extensions/com.nexus.demo/cleanup -H "authorization: Bearer $ADMIN_TOKEN"

# uninstall + purge runs every cleanup provider, including the nexus
# query-kind provider — the contributed kinds disappear from
# nexus_extension_query_kinds (and from dispatch after the next boot):
curl -s -X DELETE ':4780/api/v1/extensions/com.nexus.demo?purge=true' \
  -H "authorization: Bearer $ADMIN_TOKEN"
```

Note: this bundle lives in the **read-only in-repo pack**
(`NEXUS_EXTENSIONS_DIR`); purge removes its DB rows + caches but never deletes
in-repo source. Tarball-installed bundles (under
`NEXUS_EXTENSIONS_INSTALLS_DIR`) are removed from disk too.
