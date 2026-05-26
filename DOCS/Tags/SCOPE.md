# Tags — Scope

> ⚠ Read [ADR-003 — ClickHouse warehouse, Postgres OLTP](../storage/ADR-003-clickhouse-warehouse.md)
> first. This document defines the tag language; the warehouse is its
> first heavy consumer, but the tag types live in their own crate so
> [Insights](../Insights/SCOPE.md), [authz](../auth/), pages, and the
> AI agent share one vocabulary.

## One-line summary

`starter-tags` is the workspace's **shared tag language**: a flat
`TagSet`, a small `TagQuery` grammar, a `TagDefinition` registry, and
**two canonical compilation targets** — Postgres (`jsonb @> '…'`
against a `jsonb_path_ops` GIN index, for dimension tables) and
ClickHouse (`tags['k'] = 'v'` against a `bloom_filter` skip index on
`Map(String, String)`, for history tables). It is deliberately tiny
and deliberately stable — every crate that filters, groups, or
routes by tags depends on it, and a churn here ripples through the
warehouse, Insights, pages, and the AI agent.

The design follows Project Haystack's lesson learned the hard way:
**tags are a flat dictionary, not a typed schema**, but **refs deserve
real foreign keys** and **known tags benefit from a soft dictionary**
to prevent dialect drift.

## Hard rules

### T1 — One crate, one vocabulary

`starter-tags` is the only place `TagSet`, `TagQuery`, and
`TagDefinition` are defined. Every other crate that speaks tags
depends on it; no crate ships its own copy or parallel type. The crate
has **zero domain knowledge** — no `Building`, no `Energy`, no
`PointKind`. Domain semantics live in the `TagDefinition` rows that
deployments load at boot.

### T2 — Tags are a flat `Map<String, TagValue>`; tag values are `Bool | Str`

A `TagSet` is `BTreeMap<String, TagValue>`. `TagValue` is one of
`Bool` or `Str(String)`. **No nesting, no arrays, no floats.** If you
want "multiple tenants on this entity," you have multiple entities or
multiple boolean tags (`tenant:acme=true`, `tenant:globex=true`) — not
a JSON array.

Why flat: containment queries need flat shapes to use the GIN
`jsonb_path_ops` opclass on Postgres efficiently, AND to map cleanly
onto ClickHouse `Map(String, String)` with a `bloom_filter` skip
index. Arrays and nested objects defeat both indexes in the cases we
care about.

**Why no `Num`.** Floating-point equality on a tag bag is a footgun.
Two arithmetic paths that should produce "the same" measurement
(`0.1 + 0.2` vs `0.3`, `sum_of_readings` vs `total - other_total`)
produce different `f64` bit patterns and therefore different
stringified tag values. A `TagQuery` literal like `value:42.3` then
matches one path and not the other, identically on all three
compilation targets — so the [D6 semantic-parity test](#d6--semantic-parity-across-all-three-targets-is-a-hard-invariant)
passes while the dashboard silently produces a partial result.

Numeric *measurements* belong in the typed numeric column on the
sample/event row (`samples.value_num Nullable(Float64)` — see
[Warehouse SCOPE L2](../Warehouse/SCOPE.md#l2--curated-history-clickhouse)),
where comparisons are typed and `mart.define` aggregates them with
real numeric functions. Numeric *discriminants* (port numbers,
firmware-major versions, building IDs) belong in `TagValue::Str`
with a `TagDefinition` entry carrying `kind='str'` and an
`enum_values` set; equality on a string is exact and the bloom-filter
skip index works as designed.

When serialising to ClickHouse, `TagValue::Bool` becomes `"true"` /
`"false"`; `TagValue::Str` is stored as-is. The canonical conversion
is a single public function in `set.rs`:

```rust
pub fn tag_value_to_ch_string(v: &TagValue) -> String
```

This function is the **only** place `TagValue` → `String` conversion
for ClickHouse storage is defined. Both `compile_ch.rs` (when binding
query literals) and `starter-store-warehouse` (when writing rows)
call this function — never inline their own conversion.

**Bool / Str reserved-string rule.** ClickHouse stores both
`Bool(true)` and a hypothetical `Str("true")` as the byte string
`"true"`, so a `tags['k'] = 'true'` query would match both. The
in-process matcher T8c distinguishes them by enum variant. To keep
D6 semantic parity holding, `TagSet` construction **rejects**
`Str("true")`, `Str("false")`, `Str("True")`, `Str("FALSE")` and
any other ASCII-case variant whose lowercase normalisation matches
`"true"` or `"false"`. A writer who meant the boolean uses
`Bool(true)`; a writer who genuinely needs the literal string
`"true"` is asked to namespace it (`"true:literal"` or similar) —
in practice nobody needs this, and the type error at write time is
preferable to the silent semantic mismatch at read time.

This is enforced in `TagSet::insert`, `TagSet::extend`, and the
`Deserialize` impl for `TagValue`. Tests in `tests/parser.rs` and
`tests/semantic_parity.rs` cover the rejected cases.

**Inputs that look like numbers.** If a tag write arrives over the
wire as `{"port": 8080}`, the tag layer parses the JSON number, then
either (a) the key has a `TagDefinition` with `kind='str'` and the
number is coerced to its canonical decimal string (`"8080"`) with a
log line at INFO, or (b) the key has no definition and the value is
coerced to `Str` the same way. A JSON number that is not an integer
(`{"reading": 42.3}`) is rejected at `TagSet` construction time with
a typed error pointing the writer at `value_num` on the sample row.
This refusal is at the *typed input* boundary, not the chaotic-payload
boundary — `raw_events` still accepts the entire payload verbatim per
[W7](../Warehouse/SCOPE.md#w7--ingestion-never-refuses); the typed
`TagSet` rejection only applies to writes that target a typed shape
(`samples`, `events`, `documents`, `entities`).

**NaN, Infinity, and oversized integers.** A `TagValue::Str` whose
contents would parse back as `NaN`, `inf`, `-inf`, or a number larger
than `i64::MAX` is allowed (it is just a string), but the
`TagDefinition` numeric-discriminant pattern in T5 rejects these at
validation time. The point is to prevent a future "rehydrate Str to
number" code path from inheriting a silent footgun.

### T3 — Bare tags are sugar for `tag:true`

`sensor` in a query or a tag set means `sensor: true`. Haystack
convention; matches operator intuition; saves bytes in the database.
The serialiser writes the boolean form (`"sensor": true`) so JSONB
containment works uniformly.

### T4 — Refs are not tags

A `TagSet` may carry strings that *look* like refs (`equipRef:
"equip_…"`). They are not enforced as foreign keys at the tag layer.
**Real refs live in an `entity_refs(from_id, rel, to_id)` table with
PK + FK constraints**, owned by the [warehouse](../Warehouse/SCOPE.md).
A query that filters by ref uses both the tag bag (for fast
containment) *and* a join through `entity_refs` (for integrity-aware
traversal). The warehouse's query compiler decides which to use; tag
authors do not.

This is the single most important departure from naïve Haystack
implementations and the source of most "tags rotted over years"
failures we have seen elsewhere.

### T5 — `TagDefinition` is advisory, not a schema migration

Known tags register in a `tag_definitions` table:

```sql
CREATE TABLE tag_definitions (
  key         TEXT PRIMARY KEY,
  kind        TEXT NOT NULL,                  -- 'bool' | 'str' | 'ref' | 'num_discriminant'
  description TEXT,
  enum_values JSONB,                          -- optional canonical value set
  ref_kind    TEXT,                           -- when kind='ref', the target entity kind
  source      TEXT NOT NULL,                  -- 'builtin' | 'pack:<id>' | 'user' | 'agent'
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

The `kind` enum no longer includes a generic `'num'` — there is no
`TagValue::Num` for it to describe (T2). `'num_discriminant'`
declares "this key's `TagValue::Str` is canonically an integer
discriminant" (port numbers, building IDs, firmware-major
versions). The UI renders typed inputs; the writer validates that
the inbound JSON number coerces to an exact integer string; the
tag itself is still stored and queried as `Str`. Floating-point
measurements have no `TagDefinition` kind because they don't live
in the tag bag at all — see [T2](#t2--tags-are-a-flat-mapstring-tagvalue-tag-values-are-bool--str).

Writes to a tagged entity **consult definitions but do not require
them**. An unknown tag passes through with a `kind=str` default and a
single log line at INFO. A known tag with a wrong type warns at WARN
and coerces if possible. **Never refuses a write.** The system must
ingest chaotic data; the dictionary is how the AI and the user
gradually civilise it.

The agent reads `tag_definitions` to avoid inventing `celsius` when
`degC` already exists. The UI reads it to render typed inputs and
populate autocomplete.

### T6 — One reserved namespace, documented

The following keys are reserved by the workspace and not available for
user-defined semantics:

| Key          | Meaning                                                    |
|--------------|------------------------------------------------------------|
| `kind`       | Entity kind (`point`, `equip`, `site`, `flow`, `page`, …)  |
| `unit`       | Canonical unit string (`degC`, `kWh`, `m/s`, …)            |
| `source`     | Where the row originated (`mqtt`, `bacnet`, `flow:<id>`, …)|
| `entityRef`  | Generic ref. Use specific names (`equipRef`, `siteRef`) when the relation is known. |

**Sample quality is *not* a tag.** It is a typed column on the
[`samples`](../Warehouse/SCOPE.md) row (`quality SMALLINT`). Two
sources of truth for the same concept is a footgun; we keep it on the
column for indexed filtering and accept that quality is the one
"semantic" field that does not flow through the tag query language.
`mart.read` exposes it as a first-class filter, not a `TagQuery`
clause.

Domain packs MAY define their own reserved keys under a prefix
(`energy.*`, `hvac.*`). Bare top-level keys are first-come-first-served
*except* for the table above.

**Prefix registry.** Two packs both claiming `energy.*` is a failure
mode discovered at install time. To prevent it, prefix ownership
lives in its own table — **not** as overloaded rows in
`tag_definitions` (which is per-key advisory metadata; a prefix is
a per-glob ownership claim, structurally different):

```sql
CREATE TABLE tag_prefix_registry (
  prefix      TEXT PRIMARY KEY,        -- e.g. 'energy.', 'hvac.' — trailing dot required
  owner       TEXT NOT NULL,           -- 'pack:<id>' | 'builtin'
  claimed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

The pack installer inserts one row per declared prefix in the same
transaction as the pack's other catalog rows; a second pack
claiming the same prefix fails the `PRIMARY KEY` constraint and
the install rolls back. The dimensions migration that creates the
table seeds workspace-owned prefixes (none on day one — packs ship
their own prefix rows). The registry is consulted by:

- The pack installer, before activating the pack.
- The agent's `tag_entity` MCP tool, which warns when writing a
  prefixed key that no pack owns.
- The UI tag editor, which renders the owning pack next to a
  prefixed key in autocomplete.

A pack uninstall MAY leave its prefix row in place (orphan); the
operator chooses between reclaiming the prefix and locking out a
future pack reinstall. The doc does not pick the default — that is
an operator policy decision, not a tag-language decision.

`tag_definitions` continues to carry per-key metadata only
(`kind`, `description`, `enum_values`, `ref_kind`). The prefix
table is the only authority for who owns `energy.*`; per-key rows
under that prefix (e.g. `energy.kWh`) live in `tag_definitions`
with `source = 'pack:energy'` and inherit the prefix claim
transitively.

### T7 — Query grammar is small and total

`TagQuery` parses one of:

```
expr   := or
or     := and ( 'or' and )*
and    := not ( 'and' not )*
not    := 'not' not | atom
atom   := key | key ':' literal | '(' expr ')'
literal:= STRING | INTEGER | 'true' | 'false'
key    := IDENT ( '.' IDENT )*           // dotted keys allowed
STRING := double-quoted
```

That's it. No regex matches, no globs in the literal position, no
SQL operators leaking into the grammar, no float literals. Numeric
literals in queries are integer-only and compile to exact string
equality against the stored `Str` form (`port:8080` matches
`Str("8080")` and nothing else). Anything more expressive — range
comparisons, float thresholds, regex matches — goes through
`rule.sql` in [Insights](../Insights/SCOPE.md) where the operator
has accepted full SQL semantics, or through `mart.read` on a mart
whose aggregations promoted the measurement into a typed column.

Examples:

```
sensor and temp
point and equipRef:"equip_abc" and unit:"degC"
energy and (building:"hq" or building:"warehouse")
not stale
```

**Value-set idiom.** "Tag has any of these values" is expressed by
disjunction, not by an array literal. `building:"hq" or
building:"warehouse"` is the intended form. T2 forbids array values
inside a `TagSet`; this idiom is how you get the equivalent of `IN`
without breaking GIN containment.

### T8 — Two SQL compilation targets plus an in-process matcher

The crate exposes three total ways to evaluate a `TagQuery`: two
SQL targets (Postgres, ClickHouse) and one in-process matcher. Each
is a pure function; none touches a database directly.

#### T8a — Postgres (dimensions, authz, GIN-indexed)

`compile_to_pg(q: &TagQuery, opts: PgCompileOptions) -> SqlFragment`
produces a parameterised WHERE-clause fragment that uses **only**
`jsonb @> '…'`, `NOT`, `AND`, `OR`, and the configured tags column
name. **No `->>` extractions, no `jsonb_path_query`, no array
operators.** This is what makes the GIN `jsonb_path_ops` index
actually work.

The compiler MAY emit multiple `@>` predicates joined by `AND` (the
PG optimiser handles this well). It MUST NOT emit a predicate that
depends on a tag value being nested. Used for: `entities` filtering,
`tag_definitions` lookups, authz rules on Postgres-resident rows.

#### T8b — ClickHouse (history, `Map(String, String)` + bloom_filter)

`compile_to_ch(q: &TagQuery, opts: ChCompileOptions) -> SqlFragment`
produces a parameterised WHERE-clause fragment that uses **only**
equality on `tags['k']`, `NOT`, `AND`, `OR`. **No `mapContains` with
nesting, no `JSONExtract`, no `LIKE` on tag values.** This is what
makes the `bloom_filter` skip index actually prune granules.

`TagValue` is serialised to the stringified form described in T2
before binding. The compiler MAY emit multiple `tags['k'] = ?`
predicates joined by `AND`. It MUST NOT emit substring matches or
regex; those go through Insights' `rule.sql`. Used for: `mart.read`
filters that the mart hasn't promoted into a column, `samples`
ad-hoc reads, history-level authz.

The two compilers share the parser, the AST, and the optimiser pass
(constant folding, `not not` elimination). Only the leaf rendering
differs.

#### T8c — In-process (`Fn(&TagSet) -> bool`)

`compile_to_match(q: &TagQuery) -> impl Fn(&TagSet) -> bool` runs the
same query in-process for flow nodes that filter without hitting any
database. The match semantics are defined to be **identical** to T8a
and T8b for all queries the grammar can produce; roundtrip tests
fix this property.

## Crate layout

```
crates/starter-tags/
  Cargo.toml
  src/
    lib.rs          // public prelude
    set.rs          // TagSet, TagValue
    query.rs        // TagQuery AST + parser
    compile_pg.rs   // TagQuery -> Postgres predicate (T8a)
    compile_ch.rs   // TagQuery -> ClickHouse predicate (T8b)
    compile_match.rs// TagQuery -> in-process predicate (T8c)
    definition.rs   // TagDefinition, TagDictionary (load/save through a trait)
    reserved.rs     // the T6 table as code
    error.rs
  tests/
    parser.rs
    pg.rs
    ch.rs
    match.rs
    roundtrip.rs    // parse → render → parse fixed point
    semantic_parity.rs  // T8a ≡ T8b ≡ T8c against the same fixtures
```

Dependencies (minimal):

```toml
serde            = { workspace = true, features = ["derive"] }
serde_json       = { workspace = true }
thiserror        = { workspace = true }
nom              = "7"          # only place in the workspace we add a parser dep
```

No `sqlx`, no `clickhouse`, no `tokio`. The crate is sync,
allocation-light, and trivial to embed. `TagDictionary` storage is
behind a trait; `starter-store-postgres["dimensions"]` provides the
impl. Neither compiler links a DB driver — they produce SQL fragments
that the warehouse store crates bind.

## Public API sketch

```rust
// set.rs
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TagSet(pub BTreeMap<String, TagValue>);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TagValue { Bool(bool), Str(String) }

impl TagSet {
    pub fn insert_bare(&mut self, key: impl Into<String>);   // sugar for true
    pub fn merge(&mut self, other: &TagSet);                  // last write wins
    pub fn matches(&self, q: &TagQuery) -> bool;
}

// query.rs
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TagQuery {
    Has(String),
    Eq(String, TagValue),
    And(Vec<TagQuery>),
    Or(Vec<TagQuery>),
    Not(Box<TagQuery>),
}

impl FromStr for TagQuery { type Err = TagParseError; /* … */ }
impl Display  for TagQuery { /* canonical rendering */ }

// compile_pg.rs
pub struct PgCompileOptions<'a> { pub column: &'a str, pub first_bind: usize }
pub struct SqlFragment { pub sql: String, pub binds: Vec<serde_json::Value> }
pub fn compile_to_pg(q: &TagQuery, opts: PgCompileOptions<'_>) -> SqlFragment;

// compile_ch.rs
pub struct ChCompileOptions<'a> { pub column: &'a str, pub first_bind: usize }
pub fn compile_to_ch(q: &TagQuery, opts: ChCompileOptions<'_>) -> SqlFragment;
```

## Consumers

| Consumer                       | Uses                                                                                                                          |
|--------------------------------|-------------------------------------------------------------------------------------------------------------------------------|
| `starter-warehouse`            | `TagSet` on entity rows (PG) and history rows (CH). `compile_to_pg` for `/api/entities` filters; `compile_to_ch` for `mart.read` filters that aren't promoted into columns. |
| `starter-store-postgres["dimensions"]` | Stores `TagSet` as JSONB on `entities`, GIN-indexed.                                                                  |
| `starter-store-warehouse`     | Stores `TagSet` as `Map(String, String)` on `samples` / `events` / `documents` / `raw_events` with `bloom_filter` skip index. |
| `starter-insights`             | Tags on `Verdict` and `Dataset`, R-ins-8. `TagQuery` for verdict log filtering and rollup grouping.                            |
| `starter-server`               | `?query=…` parsing on REST endpoints. Authz rules expressed as tag queries (PG side).                                          |
| Pages / SDUI                   | Page bindings carry a `TagQuery`. Renderer passes it through to `mart.read` or `/api/entities`.                                |
| AI agent / MCP                 | `tag_entity(id, tags)`, `query_entities(tagExpression)`, `define_mart(filter: TagQuery, …)`.                                   |

## Non-goals

- ❌ A typed-schema tag system. `TagDefinition` is advisory, not a schema.
- ❌ Regex or glob value matches in `TagQuery`. Use `rule.sql` for that.
- ❌ Tag inheritance ("child entity inherits parent's tags"). Inheritance
  is a graph traversal over `entity_refs`, owned by the warehouse, not
  the tag language.
- ❌ A third storage compilation target (e.g. SQLite, DuckDB). If a
  future backend lands, its compiler gets added next to T8a/T8b and the
  semantic-parity tests grow another column.
- ❌ A built-in tag editor UI. The warehouse and pages get tag editors;
  this crate ships only the types.

## File-size budget

Per workspace rule R1 (≤ 400 lines per file). Expected:

| File                   | Target |
|------------------------|--------|
| `src/set.rs`           | < 150  |
| `src/query.rs`         | < 250  |
| `src/compile_pg.rs`    | < 200  |
| `src/compile_ch.rs`    | < 200  |
| `src/compile_match.rs` | < 150  |
| `src/definition.rs`    | < 200  |
| `src/reserved.rs`      | < 80   |

If any file approaches the limit, split by concept.

## Decisions

### D1 — Flat over nested; no float tags

Tag values are `Bool | Str`. Nested objects, arrays, and floats are
forbidden at the type level. Floats are forbidden because equality on
tag-stringified floats silently disagrees across arithmetic paths
that produce the same physical quantity; measurements live in typed
columns on the row, and discriminants live in `Str`. See T2 for the
full rationale and migration guidance.

### D2 — Refs are *not* tags

Refs live in a real `entity_refs` table with FKs (owned by the
warehouse). Tag-shaped refs (e.g. a `equipRef:"equip_…"` string on a
sample) are an optimisation for fast filtering; the source of truth is
the table. See T4.

### D3 — Advisory dictionary, never refusing writes

`TagDefinition` validates and coerces. It never refuses. Ingest is
sacred; chaos in, structure out, gradually. See T5.

### D4 — One parser dependency (`nom`)

We accept `nom` as a dependency to keep `TagQuery` parsing simple and
allocation-light. The grammar (T7) is small enough that a hand-rolled
parser would also work; `nom` wins on maintenance.

### D5 — Compilation is a pure function

`compile_to_pg` and `compile_to_ch` both return `SqlFragment { sql,
binds }`. They do not touch a database, do not allocate prepared
statements, do not link a DB driver. The dimensions store binds the
PG fragment; the ClickHouse store binds the CH fragment; Insights
binds whichever target its caller is reading from. Same AST, two
SQL flavours, no logic duplicated.

### D6 — Semantic parity across all three targets is a hard invariant

T8a (Postgres), T8b (ClickHouse), and T8c (in-process) MUST agree on
the truth value of any query the grammar can produce, for any
`TagSet` the type system can hold. A test fixture
(`tests/semantic_parity.rs`) runs the same `(query, set)` pairs
through all three and asserts equality. A diverging optimisation
that changes any one target's truth value is a bug, not a
performance win.

`tests/semantic_parity.rs` **must** include explicit coverage for:

- Integer-as-string discriminant: `port:8080` against a set written
  with `Str("8080")`. Must match on all three targets. A set written
  with any other string (`"8081"`, `" 8080"`, `"8080.0"`) must not
  match — exact string equality, no whitespace or numeric
  normalisation.
- Boolean: `flag:true` against `Bool(true)` matches on all three
  targets. **Constructing `TagSet { flag: Str("true") }` is a typed
  error** per T2's Bool/Str reserved-string rule; the fixture
  asserts that `TagValue::try_from(json!({"flag": "true"}))`
  returns `Err`, so the silent ClickHouse encoding collision is
  unreachable.
- Bare-tag sugar: `sensor` (no value) against `Bool(true)`,
  `Bool(false)`, and an absent key. Only `Bool(true)` matches.
- Float-literal rejection at parse time: `value:42.3` must fail to
  parse with a typed error pointing the writer at typed columns
  (`samples.value_num`) — not produce a query that silently never
  matches.
- `tag_value_to_ch_string` round-trip: for every `TagValue` variant,
  serialise to string and assert the string equals what `compile_to_ch`
  would bind as a literal for an equality query against that value.

### D7 — Two backends, one tag namespace

Postgres and ClickHouse store the same tag keys with the same
meanings. There is no per-backend tag dialect. A tag written into an
entity in Postgres has identical semantics when it appears on a
sample in ClickHouse. The serialisation difference (JSONB vs
stringified Map values, per T2) is invisible above the store crates.

The one documented exception is `quality`, which is a typed column
on ClickHouse `samples` (`UInt8`) and not a tag at all on either
side — see [T6](#t6--one-reserved-namespace-documented). `mart.read`
exposes `quality` as a first-class filter parameter, not a
`TagQuery` clause. This is not a per-backend dialect; it is a
deliberate exclusion from the tag namespace on both sides.
