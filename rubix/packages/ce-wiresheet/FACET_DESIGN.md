# `__facets` — runtime presentation metadata for CE components

Status: **design agreed, not yet implemented**

A way to attach per-instance, runtime presentation details to a component's
properties — display **labels**, **units**, number **formatting**, and
**dropdown options decided at runtime** (not baked into the manifest/schema).
Originally the intended purpose of the reserved `ROLE_FACETS` property role.

---

## 1. Goal

The schema/manifest defines a component's properties statically (type-level).
But instances need richer, runtime-mutable presentation:

- Friendly **labels** for properties (`in1` → "Setpoint").
- **Units** (`°C`, `kPa`, `%`).
- Number **formatting** (decimals, min/max).
- **Aliases** — label a raw value: bool `true→"compressor running" / false→"…
  stopped"`, or int `0→off / 1→auto / 2→manual`. The value stays native; only the
  display/edit changes.
- **Dropdowns whose options are decided at runtime** — e.g. a hardware component
  discovers its channels and exposes them as selectable options. These can't live
  in the manifest because they don't exist until run time.

This metadata is **per instance** and **mutable**, so it can't be schema. It
lives in a per-component property.

---

## 2. Model

Each component carries **one `__facets` property**:

- **Under the hood: an `input` property of type `string`.**
- **systemRole = `ROLE_FACETS` (2)** — hides it from normal property rows and
  from connect-to candidates, so nobody wires it.
- Its **string value** describes how to present *all the other* properties of the
  component, inline.

Why an *input* string prop:

- **It streams for free.** Config / null-typed props are excluded from the WS
  value decode table; **inputs are not**. So engine-side facet updates (e.g. new
  dropdown options) reach the UI live with **no special streaming work**.
- It's **writable** via the normal property-set path, so the wiresheet can edit it.
- `FlexValue` is scalar-only (`string | number | boolean | null`) — no list type —
  so a single string is the natural container.

### Who writes it

**Both** the engine (the component itself) and the wiresheet (the user) write it.
**Free-for-all, read-modify-write, last-write-wins** — each writer parses the
current value, sets its fields, re-serialises, and writes the whole string. (If a
UI label edit racing an engine options update ever bites, the mitigation is a
per-field write op engine-side; not needed for v1.)

---

## 3. Wire format

Optimised for **parse speed** (a component may carry formatting for many
properties). **Control-char delimited** — collision-free (these bytes never
appear in user text), no escaping, native single-char `split`. Both sides write
it programmatically, so it's never hand-authored.

Four delimiters:

| Char | Code | Separates |
|------|------|-----------|
| `␞` RS | `0x1e` | property **records** |
| `␟` US | `0x1f` | **fields** within a record |
| `␝` GS | `0x1d` | **option items** within the `o` field |
| `␜` FS | `0x1c` | an option's **int code** from its **label** |

**Record** = `<propUid>␟<field>␟<field>…`
- `field[0]` is the **property uid** (decimal string).
- Each later field is `<keyChar><value>` — the key is the **first char**, so no
  `:`/`=` needed.

**Field keys (v1):**

| Key | Meaning | Value |
|-----|---------|-------|
| `l` | label | string |
| `u` | unit | string |
| `d` | decimals | int |
| `n` | min | number |
| `x` | max | number |
| `o` | aliases / enum (value→label), inline | map, see §4 |
| `a` | dynamic options via action | action name the front end calls (on click) to fetch them |
| `h` | hidden | `1` |
| `r` | order | int |

**`o` (enum options):** `code␜label␝code␜label␝…` — the property's **value is the
int code**, never the label.

### Example

```
123456␟u°C␟o0␜Heat␝1␜Cool␝2␜Auto␞789␟lSetpoint␟d1
```

- prop `123456`: unit `°C`, dropdown `{0:Heat, 1:Cool, 2:Auto}` — its stored value
  is an int (`2` renders as "Auto").
- prop `789`: label "Setpoint", 1 decimal place.

---

## 4. Aliases & enum values — no string compares

**The property keeps its own native value** (bool / int). The facet never stores
the value — it stores how to **display and edit** it: a **value→label map**
("alias"). Same mechanism whether you call it an enum or just aliasing a bool/int:

- bool: `o0␜compressor stopped␝1␜compressor running`  (0 = false, 1 = true)
- int:  `o0␜off␝1␜auto␝2␜manual`

- The **stored value stays native** — the engine stores and compares **ints /
  bools**, never strings; dataflow logic (`mode == 2`) is an int compare.
- The `o` map is used to **display** the value as its alias ("Auto" not `2`) and to
  **edit** it (a dropdown/toggle of the alias labels, writing back the native
  value).
- Codes are the property's own values → inherently stable.
- The **presence of an `o` (or `a`) field** is what makes the UI render the prop
  as a dropdown/toggle rather than a raw number/bool input.

---

## 4b. Where the alias / option set comes from

The property always holds its own current value; the facet only says how to label
and pick it. Two sources:

1. **Inline (`o`) — aliases, stored in the facet.** The whole value→label map
   lives in the facet. For bool and small fixed int enums it's tiny, so this is
   the common case: it labels the current value (closed row) **and** is the pick
   list (open). Written by the user (Details panel) or the engine; arrives/updates
   via the normal property stream like any input prop.

2. **Action-sourced (`a`) — fetched on click, not stored.** For **large or
   runtime-discovered** sets (channels, files), the facet only names *where to get
   them*: `a<actionName>`. The list is **not** in the facet.
   - When the prop is **clicked/opened**, the editor calls
     `POST /call/nodes/uid/{uid}` with that action and renders **what it returns**.
   - The property's **own value** is what shows when closed (raw, since there's no
     stored alias for a big/dynamic set — that's fine, per design).
   - Return convention: a string encoded like `o` (`code␜label␝…`), explicit codes
     (returns are scalar). Cache the last result for instant reopen.
   - *(If you also want a label on the closed row for an `a`-sourced prop, the
     engine can keep a one-entry `o` for the current value — optional.)*

A prop may have **neither** (plain input), **`o`** (alias / fixed enum), or **`a`**
(dynamic pick list). The UI's only parse path is the facet; `a` is the single case
that also does a fetch, and only on open.

---

## 5. Parse + cache

```ts
// lib/facet.ts
type PropFacet = {
  label?: string; unit?: string; decimals?: number;
  min?: number; max?: number; hidden?: boolean; order?: number;
  options?: { code: number; label: string }[];
};
type ComponentFacet = Map<number /*propUid*/, PropFacet>;

function parseFacet(raw: string): ComponentFacet {
  const out: ComponentFacet = new Map();
  for (const rec of raw.split("\x1e")) {
    if (!rec) continue;
    const fields = rec.split("\x1f");
    const uid = +fields[0];
    const f: PropFacet = {};
    for (let i = 1; i < fields.length; i++) {
      const fld = fields[i];
      const v = fld.slice(1);
      switch (fld[0]) {
        case "l": f.label = v; break;
        case "u": f.unit = v; break;
        case "d": f.decimals = +v; break;
        case "n": f.min = +v; break;
        case "x": f.max = +v; break;
        case "h": f.hidden = v === "1"; break;
        case "r": f.order = +v; break;
        case "o":
          f.options = v.split("\x1d").map((o) => {
            const j = o.indexOf("\x1c");
            return { code: +o.slice(0, j), label: o.slice(j + 1) };
          });
          break;
      }
    }
    out.set(uid, f);
  }
  return out;
}
```

**The real perf lever is caching**, not the format. Facet strings change rarely;
rendering happens constantly. Cache **per component** (`componentUid → {raw,
parsed}`) and re-parse only when that component's raw string changes. Cost on the
render path is then ~zero, bounded by the number of components. `serializeFacet`
does the inverse for write-back.

---

## 6. Editor consumption

- **Property row** (`FunctionBlock`): label ← `l` (fallback to prop name); value
  formatted with `u` + `d`; respect `h` (hidden) and `r` (order).
- **Edit control** (override editor): `o` present → dropdown / toggle of the alias
  labels, writing back the **native value**; `a` present → fetch the list via the
  action on open (§4b). Numeric → number input with `n`/`x` bounds + unit suffix.
- **Details panel** (node right-click → *Details…*): edit `l`/`u`/… per property,
  then **read-modify-write** the `__facets` value (parse, set fields, re-serialise,
  PATCH the property — preserve fields you didn't touch).

---

## 7. CE-side requirements

1. Each component carries a `__facets` property: **input**, type **string**,
   systemRole **`ROLE_FACETS`**, **writable** via the property-set path.
2. Components may write their own `__facets` at runtime (to populate `o` options).
3. (No streaming work needed — an input prop already streams.)

---

## 8. Phasing

1. **Static presentation** — `lib/facet.ts` (parse + cache + serialize); apply
   label/unit/decimals/hidden/order to rows; Details panel to edit them. Pure
   client; highest value; works as soon as `__facets` is readable + writable.
2. **Aliases / enums (`o`)** — value→label map → dropdown/toggle, writing the
   native value; labels the value when closed. Engine- or user-set, streamed.
3. **Dynamic options (`a`)** — editor calls the named action on click and renders
   what it returns (§4b). Needs the component to expose an options-returning
   action.

---

## 9. Exposed ports — child props on a parent (folder)

Goal: a container (e.g. a **folder**, which has no props of its own) shows
selected **input/output props from its children** as its OWN ports, and edges to
them look like normal edges.

**Architecture: UI projection via `__facets` — no engine change.** The exposure
lives in the parent's facet; the props and edges are the real cross-folder ones.
The UI just renders the off-canvas child end at a named **port** on the folder
instead of as a **ghost**.

Why edges "just work": edges already use the **prop uid as the handle id**, and a
cross-folder edge to a child prop already exists (it renders as a ghost today). So
exposing = replace that ghost with a real port handle whose id is the child prop's
uid; the existing edge attaches to it and looks normal.

**Facet record (exposed port)** — keyed by the **child prop's uid** (globally
unique):
- `e<side>` — marks an exposed port + its side on the parent: `ei` = input, `eo` =
  output.
- `l` label (fallback: child prop name), `r` order — reused from the normal fields.

```
5001␟eo␟lTemp Out␞5002␟ei␟lSetpoint
```
exposes child prop 5001 as an output port "Temp Out" and 5002 as an input port
"Setpoint". Records **without** `e` are own-prop metadata (uid = own prop); records
**with** `e` are exposed ports (uid = a child prop). Same string, told apart by `e`.

**Rendering + edges:**
- The folder renders a port row per `e` record, with a handle `id =
  String(childUid)` on the correct side.
- Build a reverse index `childUid → {parentUid, side}` from the **visible**
  components' facets. In the edge/ghost pass, when a cross-folder edge's off-canvas
  end is an exposed child prop, route that end to the **parent's port handle**
  (node = parent, handle = childUid) instead of a ghost. Underlying edge unchanged.

**Value + dataType:** the child prop is off-canvas, so add the exposed child prop
uids to the **value subscription** to show the live value at the port; infer
dataType from the value (or carry a `t` field later).

**Expose UX (both):**
- Inside the folder, **right-click a child's prop → "Expose on `<folder>`"** → adds
  an `e` record to the folder's (`currentParentUid`'s) `__facets`.
- The folder's **Details… panel** lists its children's props with expose / side /
  label / order controls to manage them.

**Touch points (all client):** facet `e` field · folder render (port rows +
handles) · value subscription (exposed child uids) · edge/ghost pass (route to
port vs ghost) · expose UX (right-click + Details). No engine changes.

---

## 10. Notes / future

- Free-for-all writes can race; per-field engine write op is the escape hatch if
  needed.
- Possible later fields: `g` group, `c` color/icon, validation/step, `t` dataType
  for exposed ports.
- If structured facets ever get heavy, a native list/map value type in the engine
  would let the facet be typed end-to-end instead of a string — not needed now.
