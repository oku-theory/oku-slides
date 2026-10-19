# "Theme" shortcut: a host-built theme picker — design

Date: 2026-10-19 · Task: t-1071 · Revised after the plan review (15 reviewers).

## Goal

Changing the deck theme should cost no agent tokens. Typing or dictating
**"thème"** (the exact word, localized) in the agent panel opens, at once, an
A2UI box built by the host in the agent dock: the themes shown on the current
slide, with colour swatches. Hovering a theme previews it on the stage;
clicking applies it (one undo step). No ACP turn starts, no agent is launched.

Context: since t-988 the main menu has no theme selector; the agent changes the
theme with the `set_theme` op. This shortcut gives the same result without a
model, and works when no agent is available.

Out of scope: other shortcuts (language, scheme…), progressive rendering,
templates mode, history and help, full-deck-fidelity thumbnails.

## Decisions

1. **Host-side interception**, in `oku-slides-agent` `Stack::handle(Prompt)`,
   after the answer check: a text that answers a pending question stays an
   answer. One place covers typing, dictation, the detached agent window and
   any other prompt source.
2. **Matching**: a prompt longer than 64 bytes never matches. Otherwise it
   is normalized:
   - lowercase;
   - NFD, removing only the combining diacritics U+0300–U+036F (Latin,
     Greek and Cyrillic accents; Indic and Thai vowel signs are kept);
   - `’` read as `'`;
   - inner whitespace collapsed;
   - leading and trailing characters that are neither letters, digits nor
     marks trimmed ("Thème." → "theme").

   The result is compared with the comma-separated words of the Fluent key
   `ui-shortcut-theme` in `ui.ftl`. That file is the one source both sides
   read: the Rust `Localizer` loads it too. Each locale lists its own words
   and the English ones (`thème, thèmes, theme, themes`). Whole prompt only:
   "change le thème en sombre" goes to the agent.
   - **Host:** `oku-slides-agent/src/shortcut.rs`, in the interface locale
     (`LiveLocalizer::current`).
   - **UI:** a mirror, `ui/src/agent/shortcut.ts`, tested on the same cases.
     It lets the draft send a shortcut word when Send would otherwise be
     disabled: no agent ready, a turn running, or a turn running in another
     scope (`elsewhereNotice`). Any other prompt keeps those gates.
3. **When it applies**: the session is in deck mode (`ensure_deck_mode` is
   ok: not templates, history or help). Otherwise, or if building the box
   fails (logged), the prompt goes to the agent as before. A match sends no
   `Command::Prompt`: no agent launch, no turn, a running turn is not
   interrupted.
4. **Cards** (`oku-slides-session`, `theme_picker.rs`):
   `Session::theme_choices(slide)` returns the cards and the active theme, or
   `None` outside deck mode. Library themes come first, then deck-only themes.
   Each `ThemeCard` has the theme id, its name (clipped to 80 characters),
   its swatches and an optional thumbnail.
   - **Swatches**: `color.bg`, `color.surface`, `color.ink`, `color.accent`
     and the first colour of `chart.palette`. They are read from the effective
     tokens (`oku_slides_core::tokens::effective_tokens`). A colour that names
     another colour is resolved one level. Only values `is_hex_color` accepts
     are kept. An unreadable theme gets no swatches and no thumbnail (logged),
     never the defaults.
   - **Thumbnail**: the slide the prompt's `AgentUi.slide` names (else the
     first) is cloned into a one-slide deck. That deck is rendered in the
     `variant_renderer` with the theme's tokens, through
     `Session::render_preview`, which is shared with `variants.rs`. The page
     comes from `layout().page_of(slide)`.
   - **Bounds**: at most `THEMES_RENDERED` = 12 thumbnails. The active theme
     is rendered first and always gets one; the others share a total budget
     of `RENDER_BUDGET` = 4 s. An SVG over `THUMB_BYTES` = 512 KiB is dropped.
     Past a bound, after a failed render or on an empty deck, a card has no
     thumbnail but stays choosable.
   - **Known limit:** the session lock, which is the workspace lock in the
     app, is held while rendering, up to the budget plus one render.
   - **Unreadable themes:** a theme whose manifest cannot be read is never
     offered (`deck_themes.rs` skips it). A version that vanishes between
     listing and reading is logged, and its card has no swatches and no
     thumbnail.
   - **Fidelity**: the one-slide clone may show a different page number or
     table of contents than the full deck (accepted).
   - **Active theme**: `Session::active_theme()` gives the deck's theme id,
     else `DEFAULT_THEME` (the one it renders with).
5. **Surface**: `theme_picker(cards, active, l)` builds `host-theme-picker`:
   a title (`agent-surface-themes`), one `ThemeChoice` per card, and Close.
   It is `Place::Thread`, live in the dock, outside any turn, and the keeper
   does not persist it. The hub builds it with `LiveLocalizer::current()`, so
   its texts follow a hot language switch.
6. **Hub** (`oku-slides-mcp`, `hub/themes.rs`): `Shown::Themes(ThemePicker
   { slide, cards, active })` keeps the rendered cards. A new
   `show_themes` replaces the box in place (same id, no close-then-open).
   - **`theme {theme}`**: `picked_theme_id(context)` parses the uuid. Then
     `Session::pick_theme(id)` runs `ensure_deck_mode`, then `set_theme`, one
     undo step labelled `session-undo-theme`. It also notes
     `theme picked: "<name>" (<id>)` for the agent, with the name quoted.
     The hub emits `McpEvent::Changed`, then the box rebuilt from the cached
     cards with the new active theme. Nothing is rendered again.
   - **`dismiss`**: closes the box.
   - **`prune`** (after every deck-changing request, undo included) refreshes
     a live box. Outside deck mode it closes it. When the active theme
     changed (undo, agent, history), it re-emits the box from the cached
     cards. Thumbnails are not refreshed after slide edits (accepted: a new
     "thème" rebuilds them).
7. **Catalog**: new `ComponentKind::ThemeChoice { svg: Option<String>, name,
   slide, swatches: Vec<String>, active: bool, action }`, reserved to the
   host (`agent_allowed` false, refused by the agent `render` tool). It is
   changed in Rust and in the UI in the same commit. Zod checks each swatch
   with `Hex`, the SVG at most 512 KiB, and the name at most 200 characters.
   The catalog id stays v3.
8. **Card rendering** (`ui/src/a2ui/ThemeChoice.tsx`):
   - The thumbnail is an `<img>` with an SVG data URL, never inline SVG.
   - Below it: the swatches, then the name in `<bdi>`.
   - `aria-pressed` marks the active theme.
   - Two cards per row in the dock.
   - On hover, only the shadow changes (t-1069); the active card has an
     outline.
9. **Keys**: in the dock, `quickTarget` gives the keys to a waiting question
   first, then to `host-theme-picker`, never marked as a question so a prompt
   never "answers" it. Keys 1–4 pick the first four cards. Escape closes the
   box only when focus is in the dock, like dock questions. The keys carry no
   label.
10. **Hover preview**: a small store (`ui/src/a2ui/stagePreview.ts`) created
    once by `App` and provided by context.
    - **Setting it**: `ThemeChoice` sets `{slide, url}` on hover or focus and
      clears it on leave, blur or unmount, only if it set it.
    - **Showing it**: `Stage` shows the image over the slide only when that
      slide is the one displayed, in deck mode, outside templates and
      read-only, and while no text is being edited.
    - **Rendering cost**: only the overlay component re-renders on hover.
    - **Detached agent window**: it has no stage and no store, so it shows no
      preview.
11. **i18n**: `ui-shortcut-theme` (the words, in `ui.ftl`) and
    `agent-surface-themes` (the box title, in `agent.ftl`) in the 20 locales,
    English as reference.
12. **Dock announcement**: `host-theme-picker` is not one of the host
    settings surfaces: the dock announces it and never folds it behind
    "Réglages".

## Data flow

```
prompt "Thème." ─► Stack::handle(Prompt)
   ├─ answers a pending question? ─► answer (unchanged)
   ├─ matches agent-shortcut-theme?
   │     └─► hub.show_themes(ui.slide) ─► session.theme_choices ─► Shown::Themes ─► dock
   │         (Ok(false) outside deck mode, or Err: logged) ─► falls through
   └─ otherwise ─► Command::Prompt (agent turn, unchanged)

hover a card ─► preview store {slide, url} ─► Stage overlay (same slide only)
click / 1–4  ─► SurfaceAction theme{theme} ─► session.pick_theme (undo step + note)
              ─► McpEvent::Changed + box from cached cards
deck request (undo…) ─► hub.prune ─► box closed (mode) or re-emitted (active)
```

## Testing

- Rust, matcher (rstest): accents, punctuation, case, `’`, a sentence, an
  over-long prompt, fr/de/ja/hi/th, a Hindi near-word that must not match.
- Rust, `Stack` with the fake agent: "thème" shows the box and sends no
  prompt (checked with a sentinel prompt after it); an answer wins; a running
  turn is not cancelled; outside deck mode the prompt goes to the agent.
- Rust, session:
  - order: library themes, then deck-only themes;
  - the default theme is active;
  - swatches: references resolved, hex only;
  - the requested slide is rendered;
  - the render cap;
  - the active theme is rendered even past the budget;
  - an oversized SVG gets no thumbnail;
  - a failing render;
  - an empty deck;
  - the deck is left unchanged.
- TS, matcher mirror: the same cases as the Rust table. Draft: a shortcut
  word is sendable with no agent ready, during a turn and with a notice;
  other text is not.
- Rust, hub (`tests/themes.rs`): show outside any turn and not persisted,
  in-place replacement, `Ok(false)` outside deck mode, a pick (theme, one
  undo step, note, `Changed`, rebuilt box with one active card), a bad id
  (`Incomplete`, box kept), dismiss, `prune` (active refreshed, closed in
  templates mode).
- Contract: `every_form_matches_the_ui_fixture` covers `ThemeChoice`;
  `catalog.test.tsx` lists it; the agent `render` tool refuses it.
- UI: card rendering, click, hover/focus set and clear the preview, unmount,
  no preview without thumbnail, StrictMode; `quickTarget`; the Stage overlay
  (same slide only, hidden while editing).
- e2e `230-raccourci-theme.cjs`: type "Thème." in the agent draft; the box
  appears and no prompt reaches the agent; hover shows the preview; click
  applies the theme; Cmd+Z restores it and the active card follows; Close.
