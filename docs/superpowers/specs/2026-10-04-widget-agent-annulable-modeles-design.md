# Undoable widget write in templates mode — design

Date: 2026-10-04 · Task: t-504

## Problem

In templates mode, when the agent modifies a widget's code (`set_widget`),
the undo button stays greyed out. An agent turn must be **one** undo
step, whatever the number of MCP calls it makes.

Cause: `adopt_session_version` (`templates_mode.rs`) moves the
templates document to the new pack version through `apply_silent`.
No step is pushed on this document; the turn's group
(`begin_turn`) has nothing to catch. Only the deck set aside (`stash`)
receives a step, invisible as long as one stays in templates mode. In deck
mode, the same write goes through `apply(SetTemplates)` and joins the turn.

Reproduced by `set_widget_by_agent_in_templates_turn_can_be_undone`
(`widgets_edit_tests.rs`).

## Decisions

1. **A widget write in templates mode is a step of the templates
   document**, as in deck mode: `apply(SetTemplates)` under the label
   `widgets-edit-label`, with the editing session key `adopt_merge_key(id, parent)`.
   Within a turn, the group wins: the whole turn is one step. Outside a
   turn, successive writes with no other action between them merge
   into one step (as in deck mode).
2. **Saving templates (`save_templates`) stays silent**:
   it freezes the state, it does not change it (the edited templates are already
   steps). Out of scope for this task.
3. **The edited version follows the document.** The only source is
   `deck.templates` of the templates document. `TemplateMode` keeps its
   fields (`editing` view and `SavedMode::Templates` resume, format
   unchanged), but after each undo or redo in templates mode,
   `realign_templates` realigns them: `base` = that version;
   `session_version` = that version if it is not `parent`, otherwise
   `None`.
4. **Replaced versions.** The templates document no longer removes any
   version from its sources during the editing session: a step can always
   lead back to it (a redo after a silent save, for example).
   The library keeps "one version per editing session" on write
   (`replace_version`); a version reached by undo or redo is
   put back into it (`add_version`, no effect if it is there). The deck set aside
   keeps its current rule (merged → removed from its sources). A
   version that became unreachable after a new write stays in the
   library (no cleanup, accepted).
5. **The deck set aside follows the templates document.** After an undo
   or a redo in templates mode that changes the pack version, the deck
   set aside moves to that version (`apply` with the same editing session key):
   on closing, the deck embeds the version the user was seeing.
6. **The library keeps the version created** on undo (as in deck
   mode): undo does not change its current version. After an undo, the
   library's current version therefore stays the one that was undone: reopening
   the templates shows that version, as in deck mode.
7. **Interface: nothing to add.** `useTemplateWidgets` re-reads the files
   when `editing.base` changes (`revision`); with decision 3, an undo
   reloads the widget tabs.

Plan: `docs/superpowers/plans/2026-10-04-widget-agent-annulable-modeles.md`.

## Expected behavior

- Agent turn in templates mode with one or more widget
  writes (and possibly template edits) → a single ⌘Z, labeled
  "Agent : …", which brings the code of each widget, and the templates, back to their
  state before the turn.
- ⌘⇧Z redoes the whole turn.
- After undo, closing the templates: the deck embeds the version from before
  the turn.
- A user write on a widget (code tab, saved):
  also becomes a step of the templates document. The code editor's own
  history (CodeMirror) stays that of the typing in the tab.

## Tests (RED first)

Session (`widgets_edit_tests.rs`, `templates_mode_tests.rs`):
- turn with two `set_widget_by_agent` → `can_undo`, one step, label of the
  turn; `undo` → widget source from before the turn, `editing.base` = parent;
- `redo` → source of the turn, `editing.base` = version of the turn;
- undo then close the templates → the deck embeds the earlier version;
- undo, then a new write → new version, the old one stays in
  the library if a step still leads to it;
- two writes outside a turn with no action between them → one step.

MCP (`tests/widgets.rs`): `set_widget` twice in a turn in templates
mode → the turn's `undo_label`.

The existing template editing session tests (`session_version`, one version
per editing session) stay green.
