# LAYERING — transport never contains business logic

This is the rule most often broken under deadline pressure. AI
assistants are especially prone to it because they edit where they
first land. When asked to "add a check before saving", the laziest
correct place to put it is the handler — and that is the wrong
place every time.

> **TL;DR.** REST / gRPC / CLI / MCP handlers do four things:
> **extract → call domain → shape DTO → return.** Anything else lives
> in `*-domain` or a shared module.

---

## 1. The arrow

```
transport (REST / gRPC / CLI / MCP / SSE)
    ↓ calls
domain (pure business logic, no I/O frameworks)
    ↓ calls
data (storage, external APIs, side effects)
```

Never the other way. No SQL in handlers. No HTTP in domain. No
clap types in domain. No axum types in domain.

A domain function takes plain owned types and returns plain owned
types. It compiles without `axum`, `tonic`, `clap`, or `rmcp` in
the dependency tree of the crate it lives in.

---

## 2. What "transport" means in this repo

Every file at any of these paths is **transport-layer**:

| Where | Crate convention |
|---|---|
| HTTP route handlers | `*-transport-rest/src/**` or files containing `Router::new()` |
| gRPC tool surface | `*-transport-grpc/src/**` or `tonic` server impls |
| CLI subcommands | `*-cli/src/**` or `clap` `Args`/`Subcommand` types |
| MCP tool surface | `*-transport-mcp/src/**` or files behind `rmcp` |
| SSE / WebSocket | The handler that pushes the stream over the wire |

The line is: **the file imports a transport framework (axum, tonic,
clap, rmcp, tokio-tungstenite).** If you've imported one of those,
you're in transport.

---

## 3. The four steps a handler does — and only those

```rust
// LAYER:transport — REST handler. See docs/design/layering/.
pub async fn create_user(
    State(ctx): State<AppCtx>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<CreateUserResponse>, ApiError> {
    // 1. EXTRACT — body, query, headers, auth principal.
    let principal = ctx.auth.principal()?;

    // 2. CALL DOMAIN — one function, returns owned domain types.
    let user = domain::user::create(&ctx.db, principal, req.into_input()).await?;

    // 3. SHAPE DTO — domain type → wire type.
    let resp = CreateUserResponse::from(user);

    // 4. RETURN.
    Ok(Json(resp))
}
```

**Twenty-line ceiling.** A handler over 20 lines is almost certainly
doing work that belongs in domain. The exception is verbose extractors
(multiple typed-headers, large body validation that's structural, not
semantic). If a handler crosses 20 lines, justify it in the PR.

---

## 4. What may appear in transport

Yes:

- Framework adapters: `Path`, `Query`, `Json`, `State`, `Headers`,
  `clap::Args`, `tonic::Request`, MCP `tool!` macro invocations.
- Type conversion: `From`/`Into` between wire DTOs and domain types.
- Auth-principal extraction (the wire-level part — token parsing,
  cookie lookup). Permission *evaluation* is domain.
- Logging the request envelope (method, path, latency). Logging the
  business outcome is domain's job.
- Error-to-status mapping (`DomainError::NotFound → 404`).

No:

- SQL queries, even one-liners.
- HTTP / RPC calls to other services.
- Multi-step predicates ("if X and Y, then Z").
- Containment / placement rules ("can A be parent of B?").
- Computation, formatting, currency math, time zone math.
- Event emission to a broker.
- Default values for business fields. Defaults are a domain concept.
- Cross-resource walks ("list all children, then…").

---

## 5. The gRPC-swap smoke test

Before merging a transport file, ask:

> **If I swap this REST handler for a gRPC handler tomorrow, how
> much of this file changes?**

The correct answer is "the wire-DTO shaping and the extractor types".
If the answer includes any business predicate, any SQL, any cross-
resource walk — the logic is in the wrong layer. Move it down.

Companion test for CLI:

> **If I delete this CLI command and a fleet RPC needs the same
> behaviour, what do I lift up?** Whatever you lift up was already
> in the wrong place. It belonged in `*-domain` from day one.

---

## 6. Required header on every transport file

Every transport-layer file starts with this exact line as its first
doc-comment, immediately after the file's `//!` summary:

```rust
//! Create-user REST handler.
//!
//! LAYER: transport. Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.
```

The `LAYER: transport.` marker is the load-bearing line. It:

- Is the first thing an AI reads when opening the file.
- Is grep-able for CI (`grep -L 'LAYER: transport' transport-*/**/*.rs`).
- Names the rule in present tense without ceremony.

For the other transport flavours, swap the descriptor verb:

```rust
//! LAYER: transport (CLI).    Extract args → call domain → format → exit.
//! LAYER: transport (gRPC).   Extract request → call domain → shape response → return.
//! LAYER: transport (MCP).    Extract tool args → call domain → shape MCP result → return.
//! LAYER: transport (SSE).    Subscribe → call domain stream → shape events → push.
```

---

## 7. A worked refactor — the placement-check mistake

A real case, the kind that recurs:

**What happened.** Added `/api/v1/kinds?placeable_under=<path>` to
filter a palette. The handler in `transport-rest/src/kinds.rs`
contained:

```rust
// WRONG — business predicate in transport
let parent = db.fetch_kind(parent_path).await?;
let parent_manifest = manifests.get(&parent.kind)?;
candidates.retain(|c| {
    parent_manifest.allowed_children.contains(&c.id)
        || c.facets.contains("is_anywhere")  // copy of graph rule
});
```

**Why wrong.** That `retain` is the *same containment rule* the
domain enforces on `GraphStore::create_child`. Two sources of
truth. They drifted: domain added an `isAnywhere` carve-out, the
transport copy didn't, and the palette quietly hid valid kinds.

**The fix.** Extract a pure function:

```rust
// domain/placement.rs
pub fn placement_allowed(
    parent_manifest: &KindManifest,
    candidate: &Kind,
) -> bool {
    parent_manifest.allowed_children.contains(&candidate.id)
        || candidate.facets.contains("is_anywhere")
}
```

Both `GraphStore::create_child` and the REST handler call it. One
source of truth. The handler is now five lines:

```rust
// LAYER:transport
let parent_manifest = ctx.kinds.manifest_for(parent_path).await?;
candidates.retain(|c| placement_allowed(&parent_manifest, c));
Ok(Json(candidates))
```

The pattern generalises: **whenever a handler contains a predicate,
extract it.** Even a one-line predicate. Especially a one-line
predicate, because that's exactly the size the next session will
duplicate in another handler.

---

## 8. How to resist the "lazy add" failure mode

The failure mode the user named: AI is asked to add a check, opens
the handler (it's the file mentioned in the prompt), adds the check
there. Three counters that work:

1. **Banner comment.** Every transport file's first doc-comment ends
   with "no business predicates here." When the AI loads the file
   into context, the rule loads with it.
2. **Domain-first prompting.** When you ask the AI to add a check,
   phrase it as "add the check to the domain layer for X". This
   primes the search.
3. **Review test.** After every change touching a transport file,
   diff the file and ask: "is there a predicate, a loop, a match,
   or a cross-resource walk that wasn't there before?" If yes,
   relocate before merging.

The banner alone catches ~70% of the slips. The other 30% are the
session where the AI doesn't open the file at all and writes a new
handler from scratch — for those, NEW-SESSION.md and HOW-TO-CODE.md
must repeat the rule at session start. They do.

---

## 9. One-line summary

**Transport extracts, calls domain, shapes, returns. Anything else
lives in domain. Every transport file declares `LAYER: transport.`
in its header so the next reader — human or AI — sees the rule
before the code.**
