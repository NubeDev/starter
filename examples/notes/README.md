# notes — proof that starter is a library, not a framework

A demo consumer that extends **every** starter surface — REST, gRPC,
MCP, CLI, UI — without changing a single line in any `starter-*`
crate or `@nube/starter-*` package.

If you can read this file and trace the demo's code, you can answer
"can I extend `<surface>`?" by example for every surface starter
ships, plus one (`gRPC`) that starter doesn't ship at all.

## What it proves

| Surface  | How it's extended                                                                | Where to look |
|----------|----------------------------------------------------------------------------------|---------------|
| REST     | Domain router merged into `ServerBuilder` alongside `/health`, `/metrics`, etc.  | [src/rest.rs](src/rest.rs) |
| MCP      | Custom `Tool` impl registered into `ToolRegistry` next to starter tools          | [src/mcp.rs](src/mcp.rs) |
| CLI      | Two `Command` impls registered next to starter's `health` / `openapi`            | [src/cli.rs](src/cli.rs) |
| gRPC     | Whole-cloth tonic service — **starter has no gRPC, the consumer brought it in**  | [src/grpc.rs](src/grpc.rs), [proto/notes.proto](proto/notes.proto) |
| UI       | `NotesClient` composes `StarterClient`; `<AuthProvider>` + `tokenStrategy` reused | [frontend/src/](frontend/src/) |
| Auth     | Same `Authenticator` (token-claim) gates HTTP, MCP, AND gRPC                     | [src/server.rs](src/server.rs) (HTTP/MCP), [src/grpc.rs](src/grpc.rs) (gRPC) |
| Storage  | `NoteStore` is consumer-owned; migrations applied via starter's namespaced runner | [src/domain.rs](src/domain.rs), [migrations/notes/](migrations/notes/) |

End-to-end test: [tests/e2e.rs](tests/e2e.rs) — one process spins up
the real router, claims an owner token, then exercises REST, MCP, and
gRPC against the same backend with the same bearer.

## Try it

```bash
# 1. Apply migrations.
cargo run -p starter-notes --bin notes -- migrate

# 2. Issue the first owner token.
cargo run -p starter-notes --bin notes -- claim --yes
# → prints the bearer; export it for the next steps:
export NOTES_TOKEN=…

# 3. Start the server (HTTP :8080 + gRPC :50051).
cargo run -p starter-notes --bin notes -- serve

# 4. From another shell — REST via curl:
curl -s -X POST localhost:8080/notes \
  -H "Authorization: Bearer $NOTES_TOKEN" \
  -H "content-type: application/json" \
  -d '{"body":"first note"}'
curl -s localhost:8080/notes -H "Authorization: Bearer $NOTES_TOKEN"

# 5. Same data, via the consumer CLI:
cargo run -p starter-notes --bin notes -- add "second note"
cargo run -p starter-notes --bin notes -- list

# 6. MCP tool from any MCP client (or curl-as-jsonrpc):
curl -s -X POST localhost:8080/mcp \
  -H "Authorization: Bearer $NOTES_TOKEN" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"note_search","arguments":{"query":"first"}}}'

# 7. gRPC with grpcurl:
grpcurl -plaintext \
  -import-path proto -proto notes.proto \
  -H "authorization: Bearer $NOTES_TOKEN" \
  localhost:50051 notes.v1.NoteService/List

# 8. Frontend:
pnpm --filter starter-notes-frontend dev
# → http://localhost:5173, paste the same bearer, get a working UI.
```

## What's NOT here

- **No starter-side changes.** `git diff master -- crates/ packages/`
  has no diff for the surfaces this demo extends. The only repo-level
  add was three workspace deps (`tonic`, `prost`, `tonic-build`) the
  demo uses; no `starter-*` crate consumes them.
- **No leaked abstractions.** The notes domain (`Note`, `NoteStore`,
  `NoteError`) is plain Rust — no `starter-*` types in the function
  signatures of the domain layer. Every starter touch is at the edge
  (router composition, tool registration, auth wiring).
- **No special hooks.** Every extension uses the same public API a
  third-party consumer would use after `cargo add starter-server`.

## Things I'd add for a real product

- `serve` should bind on `:0` and write the actual ports somewhere
  (file, log) so smoke tests don't race.
- gRPC reflection (`tonic-reflection`) so `grpcurl -list` works.
- A real `NoteService::Watch` streaming RPC for the "demos with SSE"
  story.
- Frontend: tanstack-query for cache + invalidation; the demo uses raw
  `useState` to keep it under 200 lines.

None of these need starter changes either.
