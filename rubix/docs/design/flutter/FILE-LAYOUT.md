# FILE LAYOUT — one responsibility per file (Flutter)

The Flutter app inherits the same rule the rest of the repo
follows: small, well-named files are how an AI (or a human reading
cold) finds the right code without burning context on irrelevant
lines.

> **Parent doc:** [`rubix/FILE-LAYOUT.md`](../../../FILE-LAYOUT.md)
> is the canonical Rust-flavoured version. This file is the Dart /
> Flutter counterpart. It mirrors the parent section-by-section so
> a reviewer can diff the two and so an AI reading both gets the
> same mental model. Where Flutter differs (codegen sprawl,
> widgets, Riverpod), the difference is called out inline.
>
> The principles, the limits, and the naming taboos are identical
> to the parent. Only the worked examples change.

---

## 1. The hard limits

| Limit | Value | Hard? |
|---|---|---|
| Lines per file | **400** | Hard. PR blocked above this. |
| Lines per file (warning) | 300 | Soft. Plan the split. |
| Lines per function/method | 50 | Soft. Extract a private method. |
| Public items per library | ~10 | Soft. Split the library. |
| Nesting depth | 4 | Soft. Early return, extract. |
| Widget `build()` cyclomatic depth | 4 levels of widget nesting | Soft. Extract a `_HeaderBar`, `_StatusPill`, etc. |

400 lines is the **ceiling**, not the target. Most files in this
app should sit between 30 and 150 lines.

**Generated files are exempt.** `*.g.dart`, `*.freezed.dart`, and
the `app_database.g.dart` drift output can exceed 400 lines
freely — we don't hand-edit them. The CI check from §9 gates on
extension and excludes the generated suffixes.

---

## 2. The verb-per-file pattern

Group code by **the verb the caller performs**, not by the noun
it operates on. One file = one verb (or one phase of one verb).

### Example — connections feature

**Wrong** — one file per noun:

```
lib/features/connections/
  connections.dart       ← 600 lines: list + add + edit + delete + activate,
                            the screens, the controller, the repository,
                            the DAO calls, the probe network call,
                            and the model.
```

**Correct** — one dir per concern, one file per verb:

```
lib/features/connections/
  domain/
    connection/
      connection.dart               ← the freezed model only
  data/
    connection_repository.dart      ← interface + impl, calls the DAO + probe
  presentation/
    connections_list/
      connections_list_screen.dart  ← list, tap-to-activate, swipe-to-delete
      connections_controller.dart   ← @riverpod, exposes list + active
    add_connection/
      add_connection_screen.dart    ← URL + label form, probe-before-save
      add_connection_controller.dart ← @riverpod, owns the probe + save flow
    edit_connection/
      edit_connection_screen.dart   ← edit label, delete confirm
      edit_connection_controller.dart ← @riverpod, owns the update + delete flow
```

Each subdirectory groups a screen with its controller (and
generated code). The directory name **is** the concept name — no
barrel file needed inside the subdir. A feature-level barrel is
optional:

```dart
// Connections feature — barrel. Re-exports only; no logic.
export 'data/connection_repository.dart';
export 'domain/connection/connection.dart';
export 'presentation/connections_list/connections_list_screen.dart';
export 'presentation/add_connection/add_connection_screen.dart';
export 'presentation/edit_connection/edit_connection_screen.dart';
```

### Why an AI prefers this

| Task | One-file `connections.dart` | Verb-per-file folder |
|---|---|---|
| "fix the probe failure message" | Loads 600 lines. ~30 are relevant. | Opens `add_connection_controller.dart`. 100% relevant. |
| "what does delete cascade do?" | Grep `delete`, scan 20 hits. | Open `edit_connection_controller.dart`. Read top to bottom. |
| "add a widget test for the list screen" | Find the right `testWidgets` in a giant test file. | Create `connections_list_screen_test.dart` next to the source. |
| Two engineers edit connections concurrently | Same file → merge conflict. | Different files → no conflict. |

### Why a human prefers it too

A new engineer reading `lib/features/connections/` learns the
feature's API by reading the filenames before opening a single
file. That is a property no naming convention inside a 600-line
file can match.

---

## 3. When the verb itself is too big

If a verb's file approaches 200 lines, split it by **phase of the
verb**, not by reusable helper noun.

```
lib/features/auth/presentation/login/
  login_screen.dart          ← orchestrates UI, ≤80 lines
  login_form.dart            ← the form widget (fields + validation)
  login_submit_button.dart   ← the submit button with loading state
  login_error_banner.dart    ← the inline error rendering
```

Each filename is a **searchable concept**. Never `helpers.dart`,
`utils.dart`, `widgets.dart`, `common.dart`, `parts.dart`. If the
only honest name for a file is "miscellaneous widgets login
needs", the boundary is wrong — those widgets belong inside
`login_form.dart` / `login_submit_button.dart` / wherever they're
actually used, or in a higher-level `lib/shared/widgets/` with a
real concept name.

The same principle applies to a controller that grows too big:

```
lib/features/auth/presentation/login_controller/
  login_controller.dart      ← @riverpod, orchestrates phases
  validate.dart              ← input checks (private to this folder)
  issue.dart                 ← POST /auth/token
  install.dart               ← AuthStrategy install
  mark_used.dart             ← bump connection.lastUsedAt
```

---

## 4. Other layout patterns by code shape

### Widgets vs controllers vs repositories

Three distinct shapes; each gets its own file. Never co-mingled.

```
lib/features/<feature>/
  domain/
    <entity>.dart                  ← freezed model. No I/O, no Flutter imports.
  data/
    <entity>_repository.dart       ← interface + impl. Calls ApiClient, DAOs.
    dto/
      <verb>_request.dart          ← per-verb DTO files (see DTOs below)
      <verb>_response.dart
  presentation/
    <screen>/
      <screen>_screen.dart         ← StatefulWidget/StatelessWidget + its private widgets
      <screen>_controller.dart     ← @riverpod notifier driving the screen
```

The rule: a screen file owns the widget tree and its **private**
helper widgets (those used only by this screen). The controller
file owns state and side effects. The repository file owns I/O.
None of the three reach into another layer's file.

### Routes (go_router)

Route definitions are configuration, not logic. They live in one
place:

```
lib/core/router/
  app_router.dart        ← the GoRouter, redirect hook, refreshListenable
  route_paths.dart       ← string constants for every route name
```

`app_router.dart` may exceed 200 lines because it is one cohesive
configuration. If it crosses 300, split per top-level route family:

```
lib/core/router/
  app_router.dart        ← top-level GoRouter, splash redirect
  routes/
    connection_routes.dart
    auth_routes.dart
    home_routes.dart
```

Each `*_routes.dart` returns a `List<RouteBase>` consumed by
`app_router.dart`.

### Providers (Riverpod)

Providers live next to the thing they expose. **Do not** make a
`lib/providers/` folder.

```
lib/core/storage/
  app_database.dart
  database_providers.dart      ← appDatabaseProvider, daoProviders
lib/core/auth/
  token_store.dart
  token_store_providers.dart   ← tokenStoreProvider with kIsWeb branch
lib/features/connections/presentation/
  connections_controller.dart  ← @riverpod controllers, generated providers
```

Generated `*.g.dart` files for `@riverpod` annotations sit next
to their source — never centralized.

### DTOs (wire types)

In v1, DTOs are hand-written with `freezed`. Mirror the verb shape
of the API:

```
lib/features/auth/data/dto/
  login_request.dart      ← LoginRequest
  login_response.dart     ← LoginResponse
  me_response.dart        ← MeResponse
```

When a DTO is shared across verbs (e.g. a `UserDto` returned by
both `login` and `me`), it gets its own named file:

```
lib/features/auth/data/dto/
  user_dto.dart           ← UserDto — referenced by 2+ verbs
  login_request.dart
  login_response.dart
  me_response.dart
```

**The `shared.dart` escape hatch from the Rust doc does not
apply.** In Dart, a `shared.dart` next to per-verb DTO files
would be either (a) a barrel, in which case call it
`<feature>_dto.dart` or roll into the feature barrel, or (b) a
mixed bag, which violates the no-helpers rule. Name the shared
types by what they model.

In v2, when OpenAPI codegen lands, this whole folder is replaced
by `lib/core/api/generated/` (one file per spec component) and
gets §4-Generated-code exemption from the line limit.

### State (FSMs, enums)

One file per state transition is overkill for typical Flutter
work. A small `enum` lives with the model:

```dart
// lib/features/connections/domain/connection.dart
enum ConnectionHealth { unknown, reachable, unreachable }

@freezed
class Connection with _$Connection { ... }
```

If a feature has a real state machine (e.g. a multi-step
onboarding flow), split per transition exactly like the Rust doc
prescribes:

```
lib/features/onboarding/state/
  onboarding_state.dart    ← sealed class hierarchy (or enum)
  start.dart               ← initial → addingConnection
  add_connection.dart      ← addingConnection → loggingIn
  log_in.dart              ← loggingIn → ready
  fault.dart               ← * → error
```

### Errors

One file per error domain. If `connections` and `auth` both have
their own typed exceptions, they get separate files. Never one
mega `errors.dart`.

```
lib/features/auth/data/auth_exception.dart
lib/features/connections/data/connection_exception.dart
```

A `core/network/network_exception.dart` covers transport-layer
failures (timeout, DNS, TLS) that any feature might surface.

### Tests

One test file per source file, mirroring the tree under `test/`.

```
lib/features/connections/data/connection_repository.dart
test/features/connections/data/connection_repository_test.dart

lib/features/connections/presentation/connections_list_screen.dart
test/features/connections/presentation/connections_list_screen_test.dart
```

Integration tests live separately (Flutter requires this):

```
integration_test/smoke_test.dart                ← Block 5 home→login flow
integration_test/connection_lifecycle_test.dart ← add/edit/delete (future)
```

If a source file has > 5 tests, split the test file by scenario:

```
test/features/connections/data/
  connection_repository_probe_test.dart
  connection_repository_crud_test.dart
```

### Generated code

Exempt from the 400-line limit (we don't hand-edit it). In Dart
this is identified by **filename suffix**, not by folder:

| Suffix | Source | Exempt? |
|---|---|---|
| `*.g.dart` | freezed JSON, riverpod_generator, drift_dev, retrofit_generator | yes |
| `*.freezed.dart` | freezed | yes |
| `*_test.g.dart` | mocktail / build_runner test mocks | yes |
| `lib/core/i18n/l10n/*.dart` | flutter gen-l10n | yes |

The §9 CI check globs `lib/**/*.dart` and `test/**/*.dart`
excluding those suffixes.

---

## 5. File-naming rules

| Never | Always |
|---|---|
| `utils.dart` | Name the concept: `retry.dart`, `token_cache.dart` |
| `helpers.dart` | Name the concept: `slot_coerce.dart`, `url_builder.dart` |
| `common.dart` | Name what's shared: `pagination.dart`, `cursor.dart` |
| `misc.dart` / `support.dart` | Don't create. Trash drawers grow forever. |
| `widgets.dart` (with bodies) | Per-widget files, or a barrel that re-exports |
| `<feature>.dart` doing every verb | `<feature>/<layer>/<verb>.dart` per verb |
| `types.dart` / `models.dart` | Name them by what they model: `connection.dart`, `session.dart` |
| `index.dart` with 30 exports of bodies | A barrel (`<feature>.dart`) that re-exports |
| `constants.dart` | Per-concept: `route_paths.dart`, `cache_keys.dart` |

**Dart-specific:**

- **No `index.dart`.** The Dart convention is a barrel named
  after the folder (`connections/connections.dart`). `index.dart`
  is a JavaScript-ism that pollutes import paths
  (`import '.../connections/index.dart'` is uglier than
  `import '.../connections/connections.dart'`).
- **`part of` is forbidden** except for the generated `*.g.dart`
  / `*.freezed.dart` files where build_runner requires it. Hand-
  written `part of` makes a single logical file straddle multiple
  physical files, which defeats the entire reason this doc exists.
- **`snake_case.dart` always.** Dart's lint enforces it; mention
  it here because the Rust file naturally uses `snake_case.rs`
  and it's easy to drift in mixed-language repos.

If you cannot describe the file's job in one sentence without
"and" — it's two files.

---

## 6. The split heuristic

Identical to the parent. When you sit down to write a file or open
one to edit, ask in order:

1. **One-sentence test.** Can I describe this file's job in one
   short sentence with no "and"? If no → it's two or more files.
2. **Blast-radius test.** If this file changes, what else might
   break? If the answer mentions things unrelated to this concept
   → it's mixed, split it.
3. **Filename test.** Would someone searching by filename find
   what they expect? `auth_interceptor.dart` → yes.
   `auth_stuff.dart` → no.
4. **Edit-locality test.** If two PRs both touch this file, do
   they touch the same lines or different concerns? If different
   concerns → split.

If you're about to write more than **~150 lines** in a new file,
pause and split first. Adding lines is cheap once the boundary is
right; refactoring after the fact is expensive.

**Flutter-specific addition:** if a `build()` method is about to
exceed ~80 lines, extract private widget classes (`class
_StatusPill extends StatelessWidget`) *into the same file* —
that is not a split, it is a refactor for readability. Only when
those extracted widgets reach two or three usages does the
extract-to-its-own-file rule kick in.

---

## 7. When NOT to split

Discipline cuts both ways. Don't fragment for its own sake.

- A widget and its `_PrivateSubWidget` belong in the same file
  when the sub-widget is used only here. Promote when a second
  caller appears.
- A `StatefulWidget` and its `State<>` always live together.
  They are one logical unit; splitting them is malpractice.
- A small `freezed` model and its `fromJson` / `toJson` belong
  together. Generated code stays in the same folder.
- A `@riverpod` notifier and the private function it calls (that
  no other file uses) may live together — until a second caller
  appears.
- A `ThemeData` light/dark pair belongs in the same file
  (`app_theme.dart`), not two files. They are one configuration.
- Trait/extension methods for foreign types
  (`extension on Duration`) live with the file that produces or
  consumes them — *not* in a `lib/extensions/` grab bag.

Rule of thumb: split when there are **two distinct caller-visible
responsibilities**. Two private widgets that always render
together in one screen's tree are not two responsibilities.

---

## 8. Migrating an existing oversized file

A practical sequence when you find an offender:

1. **Inventory.** List the distinct responsibilities. If you
   write down "and" while listing, that's a split point.
2. **Create the layer folders** if they don't exist yet
   (`data/`, `domain/`, `presentation/`).
3. **Move one concept per commit.** Each commit:
   - moves one verb/widget/concept out into its own file,
   - updates the feature barrel (`<feature>.dart`),
   - runs `dart run build_runner build --delete-conflicting-outputs`
     if codegen is involved,
   - runs `flutter analyze`,
   - runs `flutter test`.
4. **Co-locate tests** in the same sequence. Each source file's
   tests move into a sibling test file under `test/`.
5. **Delete dead code** along the way. If a private widget is no
   longer referenced after the split, it was scaffolding —
   remove it.
6. **Update imports** as you go. Dart's import paths are file-
   relative; an unmoved consumer keeps working only if the barrel
   still re-exports the old name. Add a `// TODO(<your-handle>):
   prune` comment if you defer the import update — never longer
   than one PR.

Do not attempt a single mega-commit. Reviewers can't read it and
`git bisect` can't reach into it.

---

## 9. Enforcement

- **Author-time.** Every PR description states whether any file
  approaches 400 lines and why if so.
- **Review-time.** A reviewer who sees a `*.dart` file over 300
  lines asks about a split.
- **CI** (planned): `tools/check_file_size.dart` (or the same
  shell script the Rust side uses, extended) that fails if any
  tracked `lib/**/*.dart` or `test/**/*.dart` file exceeds 400
  lines, excluding the generated suffixes listed in §4.
- **`flutter analyze`.** `riverpod_lint` and `very_good_analysis`
  catch a different class of issue (provider misuse, style) — they
  do not enforce file size. The two checks are complementary.

Until the CI check lands, the rule is enforced by reviewers and
by this doc.

---

## 10. One-line summary

**One verb / one widget / one concern per file. Folder-of-verbs
over file-of-nouns. ≤400 lines hard, ~100 lines typical. Names
are concepts, never shapes (`utils`, `helpers`, `widgets`).
Generated `*.g.dart` and `*.freezed.dart` are exempt; everything
else counts.**
