# 02 — Binding substrate gaps

> **Tier:** scope (plan). Lifetime: weeks. Not referenced from code.
> See [README.md](./README.md). Authoritative present-tense docs
> for the existing engine live in
> [`crates/starter-ui-bindings/src/lib.rs`](../../../../crates/starter-ui-bindings/src/lib.rs)
> doc-comments and `DOCS/frontend/sdui/SCOPE.md`.

## Why this file

The starter binding engine is semantically equivalent to the old
`rubix-agent/dashboard-runtime`, but **four polish items** are
missing. Without them, templates only work for visible-text
widgets — charts, tables, actions, and forms see `{{...}}` as
literal strings. This is the single highest-leverage piece of
work in the dashboard plan.

All four ship inside `crates/starter-ui-bindings/` (Rust, no
extra deps). Each is < ~200 LOC.

## Today's grammar (already shipping)

For reference — this part is **not changing**:

```text
binding := source ( "." ident   # slot-read on cursor, then JSON walk
                  | "/" ident   # child-walk (move cursor)
                  )*
source  := "$target" | "$stack" "." alias | "$self"
         | "$user"   | "$page"
```

`{{$target/parent/parent.value}}` ↔ Niagara's
`parent.parent.slot.value`. One parsed `Binding` evaluates against
N targets without re-parsing — load-bearing for templates.

## The four gaps

### G1 — Per-variant `Bindable` dispatch (substitute_tree on non-text)

**State today.** [`substitute_tree`](../../../../crates/starter-ui-bindings/src/substitute.rs)
only walks `Component::Text.content` and
`Component::Heading.content`. Every other variant (`chart`,
`table`, `form`, `action`, `select`, `kpi`, …) is passed through
with literal `{{...}}` in its string fields.

**Why it matters.** A chart whose `node_id` is `"{{$target}}"`
never resolves; the renderer sees the brace-string. **Nothing
useful templates without this.**

**Plan.** The `Bindable` trait already exists in
[`crates/starter-ui-ir/src/bindable.rs`](../../../../crates/starter-ui-ir/src/bindable.rs).
Implement it per-variant and dispatch from `substitute_tree`.

```rust
// crates/starter-ui-ir/src/bindable.rs (extend)
pub trait Bindable {
    /// Visit every `{{...}}`-bearing string field, allowing the
    /// visitor to replace it in place.
    fn visit_bindings<F>(&mut self, visit: &mut F)
    where F: FnMut(&mut String);
}
```

One file per variant under `crates/starter-ui-ir/src/bindable/`:

```
bindable/
  mod.rs          ← dispatcher (match Component { ... })
  text.rs         ← content
  heading.rs      ← content
  chart.rs        ← title, sources[].node_id, sources[].slot, sources[].rsql.query
  table.rs        ← source.query, columns[].field, row_actions[].handler args
  kpi.rs          ← title, value, delta.*
  form.rs         ← fields[].label, fields[].default
  action.rs       ← handler args, target_ref, navigate_to.page_ref
  select.rs       ← options[].label
  tabs.rs         ← tabs[].label, tabs[].id
  repeat.rs       ← source (the array binding), template visited per-iteration after expand
  // ... one file per IR variant that carries bindings
```

`substitute.rs` becomes ~30 lines: walk the tree, call
`visit_bindings` per node with a closure that runs `substitute_text`
on each `&mut String`.

**Test.** A round-trip fixture page with a `kpi` whose
`value = "{{$target/disk.percent}}"` resolves correctly and emits
one subscription subject.

### G2 — Qualifier syntax (`|prefix=` `|suffix=` `|default=`)

**State today.** Missing claims throw `BindingError::UnknownUserClaim`;
empty values render as `""` but with no way to collapse a
surrounding fragment.

**Why it matters.** The old rubix-agent embeds bindings in **RSQL
query strings**:

```text
"kind==project{{$page.status|prefix=';status=='}}"
```

When `$page.status` is empty, the whole `{{...}}` (including the
`;status==` prefix) must collapse to `""`, producing
`"kind==project"`. Without it, every conditional filter becomes
two pages.

**Plan.** Port the qualifier logic from
[`examples/rubix-agent/crates/dashboard-transport/src/binding_walk.rs`](../../../../examples/rubix-agent/crates/dashboard-transport/src/binding_walk.rs)
into `crates/starter-ui-bindings/src/substitute.rs`. New file:
`qualifiers.rs` (≤120 LOC):

```rust
struct Qualifiers { prefix: Option<String>, suffix: Option<String>, default: Option<String> }

fn split_qualifiers(raw: &str) -> (&str, Qualifiers);
fn apply(value: Option<JsonValue>, q: &Qualifiers) -> String;
```

Rules:

- `{{expr|prefix='x'|suffix='y'|default='z'}}`.
- Quoted values strip exactly one layer of `'…'` or `"…"`.
- Empty / null / missing → use `default` if set, else `""` — the
  whole `{{...}}` block disappears (including its prefix/suffix).
- On evaluation failure: leave the `{{...}}` literal in place so
  downstream RSQL parsing throws a clear error (and the AI
  dry-run flagged it earlier).
- The grammar itself is unchanged; qualifiers live **inside** the
  brace pair, parsed after binding evaluation.

**Test.** The exact fixture from the old code:
`"kind==project{{$page.status|prefix=';status=='}}"` → with
status `""` → `"kind==project"`. With status `"open"` →
`"kind==project;status==open"`.

### G3 — `Component::Repeat` expansion pass

**State today.** [`Component::Repeat`](../../../../crates/starter-ui-ir/src/component.rs)
variant exists (with `source`, `alias`, `template` fields) but
no expander ships. `substitute_tree` doesn't recognise it, so the
renderer would see a literal `Repeat` node.

**Why it matters.** "List of cards, one per item" — the canonical
Grafana-style template repeat — doesn't work without this.

**Plan.** Port the expander shape from
[`examples/rubix-agent/crates/dashboard-transport/src/expand.rs`](../../../../examples/rubix-agent/crates/dashboard-transport/src/expand.rs).
New module `crates/starter-ui-bindings/src/expand.rs` (≤200 LOC):

```rust
pub fn expand_repeats<G: EntityGraph + ?Sized>(
    tree: &mut ComponentTree,
    ctx: &EvalContext<'_, G>,
) -> Result<(), ExpandError>;
```

Behaviour:

1. Walk the tree depth-first.
2. At every `Component::Repeat { source, alias, template }`:
   1. Evaluate `source` against `ctx` → must produce a JSON array
      (`BindingError::WalkThroughNonObject` on anything else).
   2. For each item, clone `template`, and substitute bindings
      with two extra synthetic context fields: `$item` (the item
      JSON), `$index` (zero-based), and — if `alias` is set —
      `$<alias>` aliased to `$item`.
   3. Replace the `Repeat` node in its parent's `children` with
      the expanded copies.
3. Empty source → zero expansions → the `Repeat` disappears.
4. Non-array source → leave the `Repeat` in place (dry-run
   surfaces it).

Expansion runs **before** `substitute_tree` so each instantiated
copy is a normal subtree by the time the standard pass walks it.
The synthetic `$item` / `$index` / `$<alias>` source variants
extend `parse.rs`'s `Source` enum additively.

**Test.** A page with a `Repeat` over `$target/children/devices`
expands once per child, each card binding `{{$item.name}}` and
`{{$index}}`.

### G4 — Synthetic widget ids + layout override in resolve

**State today.** Two small ergonomics gaps:

1. The IR allows `chart` / `table` / `sparkline` to omit `id`.
   When they do, the subscription plan's subjects collide with
   any other unidentified widget — and the client has no key to
   patch against.
2. The resolver takes `page_ref` only — no way to send an
   in-flight buffer (used by the AI builder preview so the
   un-saved edits show in the live preview pane).

**Plan.**

**G4a — Synthetic ids.** Add
`crates/starter-ui-bindings/src/synthetic_ids.rs` (≤80 LOC):

```rust
/// Walks the tree, assigning `chart-<N>`, `table-<N>`,
/// `sparkline-<N>`, `timeline-<N>` to any widget whose id is
/// empty. N is a stable depth-first counter — re-resolving the
/// same body produces the same ids.
pub fn assign_synthetic_ids(tree: &mut ComponentTree);
```

Runs immediately after `PageProvider::lookup_page` and before
binding substitution, so the access log and the rendered tree
carry identical keys.

**G4b — Layout override.** Extend
[`ResolveRequest`](../../../../crates/starter-sdui-routes/src/routes/resolve.rs)
with one optional field:

```rust
pub struct ResolveRequest {
    pub page_ref: String,
    /// In-flight body sent by an authoring client; when present,
    /// the resolver uses this instead of the persisted tree.
    /// Subject to the same R8 caps and capability filter.
    #[serde(default)]
    pub layout: Option<ComponentTree>,
    ...
}
```

In the handler, `lookup_page` is **only** called if `layout`
is `None`. The R8 byte-cap and capability filter run regardless.

**Test.** Resolve with `layout = Some(...)` skips the store; the
draft is rendered with `$user.email` substituted as usual.

### G5 — Portable IR subset (cross-platform contract)

**State today.** `starter-ui-ir` contains several variants that
implicitly assume DOM/CSS:

| Variant / field | Issue | Evidence |
|---|---|---|
| `Page.default_row_gap`, `default_column_gap`, `default_page_padding`, `default_max_width` | Free-form CSS-length strings | [`crates/starter-ui-ir/src/component.rs:259`](../../../../crates/starter-ui-ir/src/component.rs) |
| `Row.gap`, `Col.gap` | Same | same file |
| `JsonTable.max_height_class` | Tailwind class string | [`component.rs:739`](../../../../crates/starter-ui-ir/src/component.rs) |
| `Heading.level 1–6 → <h2>–<h6>` | Doc-comment hard-codes HTML element mapping | [`component.rs:555`](../../../../crates/starter-ui-ir/src/component.rs) |
| `Menu` (uses `<hr>`), `Detail` (uses `<dl>`) | HTML elements baked into the rendering contract | [`component.rs:832`, `:1004`](../../../../crates/starter-ui-ir/src/component.rs) |
| `Custom.props` | Verbatim JS — no RN target | [`component.rs:1494`](../../../../crates/starter-ui-ir/src/component.rs) |

(All line numbers approximate — re-grep before quoting in code.)

**Why it matters.** A future `@nube/starter-ui-sdui-rn` sibling
must know which variants it can implement faithfully and which
must downgrade to `Dangling` via the capability handshake.

**Plan — flag only, don't refactor in v1.** Add to
`crates/starter-ui-ir/src/lib.rs`:

```rust
/// Variants in the IR that a non-web renderer (react-native,
/// SwiftUI, Flutter) can implement without DOM/CSS assumptions.
/// Variants NOT in this list either embed CSS-string fields
/// (`Page.default_row_gap`), use HTML elements as their rendering
/// contract (`Menu`, `Detail`, `Heading`), or carry verbatim
/// platform-specific props (`Custom`). Non-web renderers must
/// either implement them with a documented platform mapping or
/// downgrade to `Dangling` via the capability handshake.
pub const IR_PORTABLE_VARIANTS: &[&str] = &[
    "page", "row", "col", "grid", "card", "tabs", "divider",
    "text", "kpi", "chart", "table", "form", "select", "toggle",
    "slider", "date_range", "ref_picker", "repeat",
];
```

Plus per-variant doc-comments on the non-portable items naming
the specific portability concern.

**Out of scope here.** Tokenising the CSS-length fields against
`starter-ui-theme` (so gaps become `"sm" | "md" | "lg"` instead
of `"12px"`) is a follow-up — additive, no breaking change. Don't
block v1 on it.

**Test.** A unit test in `starter-ui-ir/tests/portable_subset.rs`
asserts the constant is non-empty and that every listed variant
parses through `Component::Tag`'s discriminator.

### G6 — `$msg.<key>` binding source for i18n

**State today.** The five `Source` enum variants (`$target`,
`$self`, `$stack`, `$user`, `$page`) cover entity-graph and
request-context bindings. There is no way to express "this
string is a localised catalogue key".

**Why it matters.** Bundled dashboard pages ship from
`rubix-flows/dashboards/*.json` as **domain content** — and
`docs/design/i18n-prefs/README.md` says domain code never holds a
localised string. Without `$msg`, bundled JSON must either (a)
carry EN literals (contract violation, see Q8) or (b) be
generated per-locale (fan-out + key drift).

**Plan.** Additive grammar change in
`crates/starter-ui-bindings/src/parse.rs`:

```rust
pub enum Source {
    Target,
    SelfNode,
    Stack { alias: String },
    User,
    Page,
    /// `$msg.<key>` — catalogue lookup against the request locale.
    /// The first `.ident` step is the dotted MessageKey; the
    /// evaluator resolves it via the host-supplied
    /// `MessageCatalogue` trait. Further `.ident` steps walk into
    /// the resolved value as JSON (rare; supports structured
    /// catalogue entries).
    Msg,
}
```

Evaluator (`eval.rs`) seed:

```rust
Source::Msg => {
    // First Slot step is the catalogue key; collapse all
    // remaining steps into the lookup path.
    let key = binding.steps.first().and_then(/* … */).ok_or(BindingError::MsgNeedsKey)?;
    let value = ctx.catalogue.lookup(key, ctx.locale)?;
    Ok((None, Some(value)))
}
```

The `MessageCatalogue` trait lives next to `EntityGraph`:

```rust
pub trait MessageCatalogue {
    fn lookup(&self, key: &str, locale: &str) -> Option<JsonValue>;
}
```

Rubix's impl wraps the existing `starter-i18n::MessageBundle`
(one file `rubix-agent/src/sdui/catalogue.rs`, ≤ 60 LOC).

**EvalContext.locale.** Add a `locale: &'a str` field to
`EvalContext`; the resolver populates it from the `Accept-Language`
header → `$user.language` cascade (see
[03-host-glue.md](./03-host-glue.md)).

**Test.** A bundled page with
`{{$msg.rubix.dashboard.overview.title}}` resolves to the EN
string when `locale="en"` and the ES string when `locale="es"`.

## File layout summary (Rust)

```
crates/starter-ui-bindings/src/
  expand.rs              ← G3
  synthetic_ids.rs       ← G4a
  qualifiers.rs          ← G2 (called from substitute.rs)
  catalogue.rs           ← G6: MessageCatalogue trait + NullCatalogue
  substitute.rs          ← extended for G1 dispatch
  parse.rs               ← additive $item / $index / $<alias> / $msg sources
  eval.rs                ← G6: EvalContext.locale + Msg seed
  lib.rs                 ← re-exports for the new symbols

crates/starter-ui-ir/src/
  bindable/              ← G1: one file per variant
    mod.rs
    text.rs … repeat.rs
  portable.rs            ← G5: IR_PORTABLE_VARIANTS constant + doc

crates/starter-sdui-routes/src/
  routes/
    resolve.rs           ← G4b: ResolveRequest.layout, locale threading
  state.rs               ← G6: catalogue field on SduiState
```

Per FILE-LAYOUT, every new file ≤ 200 lines. `mod.rs` is the
barrel; no logic in the barrels.

## Acceptance for this slice

1. A page with a `kpi { value: "{{$target/disk.percent}}" }` resolves
   the binding and the subscription plan contains the corresponding
   `(entity, slot)` subject.
2. The RSQL prefix qualifier fixture from G2 passes.
3. A `Repeat` over an array of three items expands to three
   identical subtrees, each with its own `$item` / `$index`
   substituted.
4. Three chart widgets without explicit ids get
   `chart-0` / `chart-1` / `chart-2`; re-resolving produces the
   same ids.
5. `ResolveRequest { page_ref: "x", layout: Some(tree), ... }`
   renders `tree` without touching the store.
6. `IR_PORTABLE_VARIANTS` exists, listed variants are documented
   as portable, and the non-portable ones carry a doc-comment
   naming the specific concern.
7. `{{$msg.rubix.dashboard.overview.title}}` resolves to the EN
   catalogue value when `locale="en"`, the ES value when
   `locale="es"`, and an empty string + `Diagnostic` when the
   key is missing.

## What this scope does **not** include

- The Phase-4 `FetchPlan` batched historical pull
  ([07-fetch-plan.md](./07-fetch-plan.md)) — separate slice.
- Action handlers and the `/action` endpoint expansion — covered
  by host glue in [03-host-glue.md](./03-host-glue.md).
- Page sessions / renderer-id hash caching (W7 wire optimisation
  from old rubix-agent) — defer to v2.
- Tokenising `Page.default_row_gap` etc. against
  `starter-ui-theme` — flagged via G5, refactored in v2.
