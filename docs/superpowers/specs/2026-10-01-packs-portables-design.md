# Portable packs: identity, versions and embedded pack — design

Date: 2026-10-01 · Task: t-328

## Goal

A deck renders everywhere as it does on its author's machine. The file embeds the
exact version of the template pack it uses (slide templates and
widgets); the user's library becomes versioned and
identifies each pack by a unique id.

"Portable widgets" programme: 0. bounded compilation (t-327, shipped);
1. **portable packs** (this spec); 2. widget creation by the agent and
Typst editor (t-329).

Out of scope: pruning versions, choosing an arbitrary version
of a pack (beyond the banner's two actions), widget creation.

## Decisions

- **Reference and cache.** The deck references a pack (id) at a version;
  the file embeds that version. Rejected: the pack living in the
  Loro replica (Typst code merged character by character).
- **The cache is authoritative for rendering.** The deck renders with the version it
  embeds, never with the library's. Another version in the
  library gives a banner, never a different render.
- **Every deck embeds its pack**, built-in or not. A built-in pack is a
  preinstalled pack with a known id.
- **Versioned library.** A pack is a set of content-addressed versions
  under one id; each version knows its
  parent. One **current** version per pack, which changes only through a
  user action.
- **Received versions are added silently** to the library, without
  touching the current version.
- **Id = UUID v4**: dependency `uuid` (features `v4`, `serde`), already
  present transitively in `Cargo.lock`, declared in the workspace.
- **No backward compatibility.** `DECK_VERSION` goes to 3,
  `SOURCES_VERSION` to 4; v2 decks and slug-format packs are no
  longer read.

## Model (core)

```rust
pub struct PackRef { pub id: Uuid, pub version: PackVersion }
pub struct PackVersion(String);   // "sha256:<hex>", canonical content
pub struct Deck { …, pub templates: Option<PackRef>, … }
```

- `templates: None`: the default pack (`essentiel`, constant id). On
  load, the library's current version is embedded in the
  bundle and referenced explicitly (without an undo step).
- **Canonical content** of a version: `pack.json` serialized as JSON with
  sorted keys, a `0` byte, then `widgets.typ` as is (empty if it is
  absent). `PackVersion` is its sha-256. Name and description are in
  `pack.json`: changing them creates a version.
- `pack.json`: `{ id, name, description, parent: Option<PackVersion>,
  created, templates }`. The slug goes away.
- In the Loro replica, `templates` is an atom (last writer wins).

## File (sources zip, version 4)

```
manifest.json     { format: "oku-slides-sources", version: 4 }
deck.loro
assets/<hex>
fonts/<name>
packs/<hex>/pack.json
packs/<hex>/widgets.typ    (absent if the pack has no widgets)
```

- `SourceBundle.packs: BTreeMap<PackVersion, PackFiles>`, append-only,
  like `assets` and `fonts`. `merge` takes the union; `deck.templates`
  follows the replica merge.
- On save, the bundle keeps every version referenced by the current
  deck or by the replica history (history mode also restores
  the pack of a past version).
- On read, a version whose hash does not match the content is
  the only thing discarded and reported (`Issue::PackVersionMismatch`); the rest of the
  file opens. An invalid `packs/` path (folder other than a `<hex>`
  of 64 lowercase hexadecimal digits, subfolder) refuses the whole zip
  (`SourcesErrorKind::BadName`). The `packs/` and
  `packs/<hex>/` directory entries, and unknown files in a version folder, are
  ignored. Packs count towards `max_sources_mib`.

## Library (session)

```
<data>/templates/<uuid>/current            # PackVersion of the current version
<data>/templates/<uuid>/versions/<hex>/pack.json
<data>/templates/<uuid>/versions/<hex>/widgets.typ
```

- `Library`: `packs()` (id, name of the current version, built-in or not),
  `versions(id)`, `version(id, v) -> Option<PackFiles>`,
  `current(id) -> Option<PackVersion>`, `add_version(id, files)` (without
  touching `current`), `set_current(id, v)`, `relation(id, a, b) ->
  {Same, Ancestor, Descendant, Sibling}` (through the chain of `parent`s).
  Atomic write, through `Storage`.
- **Built-in packs** (`templates/essentiel`, `templates/grilles` from the repo,
  embedded in the app): constant id in their `pack.json`, version read
  read-only. Their current version is the app's as long as
  the user has no version of their own (no `current` file); as soon
  as they have one, it is theirs, and a new app version only adds
  a sibling version.
- `current` pointing to an absent version: the most recent by
  `created`, warning logged. An unreadable version folder is
  logged and ignored.

## Flows

### Rendering

- `resolve_widgets` reads the pack from the deck's sources:
  `bundle.packs[templates.version]`, and loads it through
  `RenderWorker::probe(PackWidgets::new(<hex>, widgets.typ))` — the pack id
  in the Typst world becomes the hash.
- Referenced version absent from the bundle (development JSON deck,
  damaged file): base widgets and notice `session-pack-missing`.
- No `widgets.typ`: base widgets, without a notice.

### Opening a file

1. Each embedded version absent from the library is added to it
   (`add_version`). An unknown id creates the pack, with this version as
   current.
2. If the deck's version differs from the current version of its pack, the
   session sets a **banner** (view `pack_notice`): pack, relation
   (`Ancestor`: older; `Descendant`: newer; `Sibling`:
   parallel), and two actions:
   - **Passer à ma version** (Switch to my version): `SetTemplates` to the current version,
     in an undoable transaction; the version is added to the bundle.
   - **Faire de la sienne ma version courante** (Make theirs my current version): `Library::set_current`.
     Offered only if the library has the deck's version (otherwise,
     embedded copy discarded or failed add: `make_current` is `null`
     and the button hidden).
   The banner closes for the session; it never blocks opening.

### Templates mode

- It modifies the **current** version of the deck's pack. Saving creates a
  version (`parent` = the old current one, `created` = now), makes it
  current, adds it to the bundle of the deck set aside, which switches to it
  (`SetTemplates`).
- **One version per editing session.** The first save (manual
  or automatic) of a templates mode session creates the version (`parent` =
  the current one at the start of the session) and makes it current; each
  later save in the same session **replaces** it (same `parent`):
  the intermediate version is deleted from the library (the deck's bundle
  stays append-only), until templates mode is exited.
- `PackMakeCurrent` and `PackUseMine` are refused during templates mode.
  The undo merge key gives a single undo step per
  editing session.
- "Enregistrer comme copie" (Save as copy): a new id, without `parent`; the session
  continues on the copy's version.
- Modifying a built-in pack creates the user's version under the
  built-in pack's id (no more separate copy).

### New deck

Like any deck without `templates`: the current version of the default pack
is added to its bundle and referenced from load.

### CLI

`render` and `save` of a deck whose pack has widgets go through the
render process (`RenderWorker`): the code of a received pack is isolated there
(spec t-327). Without pack widgets, the in-memory `Renderer` stays.

`open --out` also extracts `packs/<hex>/{pack.json,widgets.typ}`. `save`
takes the referenced version first from `<deck folder>/packs/<hex>`
(checked by `PackFiles::matches`), then from the library;
found nowhere: `save` fails (`session-pack-version-missing`).

### MCP and agent

The `widgets` tool describes the widgets of the embedded version. No new
tool.

## Errors

- core: `Issue::PackVersionMismatch { version }` (wrong hash: only the version
  is discarded), `Issue::PackVersionMissing { version }`; invalid
  `packs/` path: the whole zip is refused (`SourcesErrorKind::BadName`).
- session: `UnknownPack { id }`, `PackVersionMissing { id, version }`,
  `PackIdMismatch { id, found }` (`SetTemplates` to a version of a
  pack other than `id`, refused) (Snafu, backtrace, lowercase).
- Fluent keys, 20 languages: errors above, `session-pack-missing`,
  banner (`pack-notice-older`, `pack-notice-newer`, `pack-notice-parallel`,
  `pack-notice-use-mine`, `pack-notice-make-current`).

## Repository

- `templates/essentiel/pack.json`, `templates/grilles/pack.json`: constant
  id, `parent: null`, fixed `created`.
- Fixtures, CLI, MCP, session and agent tests, and the `typst/oku-slides` README
  rewritten for v3 (`templates`: `PackRef`, bundles with `packs/`).
- `Library` rewritten; `free_id`, `slug`, `is_builtin` by slug
  go away.
- UI: `api.ts` (`PackRef`, `pack_notice`), banner in the deck view,
  pack list by id.

## Tests

In build order, with TDD:

1. **Core**: `PackVersion` of the canonical content stable and independent of
   key order; `PackRef` round trip; zip: intact version read,
   wrong hash discarded and reported, invalid path refused; merging two
   bundles = union of the versions; versions referenced by the history
   kept on save.
2. **Library**: `add_version` does not change `current`;
   `set_current`; `relation` (same, ancestor, descendant, sibling); built-in
   version read-only and current by default; `current` pointing to an
   absent version.
3. **Session**: rendering uses the embedded version even if the
   library has another one; opening: received version added
   silently; banner and its two actions (including undoing "Passer à ma
   version"); templates mode: new version whose `parent` is
   the old current one; copy: new id; history: restoring a
   past version restores its pack; version absent from the bundle: base and
   notice.
4. **CLI**: `render` of a PDF with an embedded pack goes through the render
   process; a looping pack does not freeze it.
5. **Regression**: a received deck with the same pack id but different
   content never changes the library's current version.
6. **i18n**: new keys in the 20 languages.
