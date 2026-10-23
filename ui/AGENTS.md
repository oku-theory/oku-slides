# ui — agent rules

## Invariants

- Apply editor text edits as a minimal replacement (minimalChange) so CodeMirror never rewrites the whole document. (plan:2026-09-26-editeur-markdown)
- Do not end text editing on blur when document.hasFocus() is false; another window or app took focus. (spec:2026-10-09-agent-detachable-design)
- Route every visible UI string and ARIA label through t() with Fluent keys; never hardcode text. (t-44)
- Ignore quick keys on editable targets, with modifiers, on repeat, during IME composition, or when already handled. (plan:2026-09-26-agent-vocal)
- Build modals on useModalFrame (sole FOCUSABLE, ARMING_MS); confirmations take no Enter or digits and ignore input for 500 ms. (plan:2026-09-26-agent-vocal, t-692)
- Route every agent surface somewhere visible; anything that cannot be placed falls back to the thread. (plan:2026-09-26-agent-vocal)
- Validate every agent-link message with zod and drop invalid ones without logging; they may hold dictated text. (spec:2026-10-09-agent-detachable-design)
- Send draft updates one at a time tagged with the view origin, and ignore echoes of the view's own sends. (spec:2026-10-09-agent-detachable-design)
- Debounce draft typing by 300 ms but send dictation, Send, Discard and open at once; flush it before attaching or closing the agent window. (plan:2026-10-09-agent-detachable, t-701)
- Never switch a mounted AgentPanel between driver and follower mode; mount one panel per tab and window. (plan:2026-10-09-agent-detachable)
- Match shortcut letters by key (layout-aware) and digits by physical code, so AZERTY works. (spec:2026-09-26-onglets-fenetres-design)
- Ignore any WorkspaceView older than the last applied generation; command replies can arrive after newer events. (t-159)
- Defer agent focus during text editing, apply only the last one when editing ends, and ignore unknown ids. (plan:2026-10-09-tickets-debloques-t598, t-210)
- textToParagraphs must keep the existing paragraph, with its formatting, for any line whose text did not change. (plan:2026-09-27-markdown-lezer)
- In ListTable, insert a row only after the cell save succeeds, and revert the cell when the core refuses it. (plan:2026-10-04-suivis-listes, t-534)
- Send every widget file write, rename, delete and create through the single useWidgetFiles queue; show widget SVG previews only in an img element, never in object, iframe or dangerouslySetInnerHTML. (plan:2026-10-03-widgets-onglets)
- Send surface actions to the scope of the displayed surface, never to the current selection. (spec:2026-10-03-widgets-agent-a2ui-design)
- Format grid and list numbers with ui/src/table/numbers.ts: deck language separators, U+2212 minus, shortest form. (t-655)
- In parseCell convert only unambiguous decimals to numbers; keep leading-zero codes, hex and exponents as text. (t-592)
- After removing the current slide, select a neighbour and never fetch a slide id absent from the snapshot. (t-61)
- Treat Backspace like Delete in delete-key handlers so behaviour matches App on macOS. (t-420)
- Import a2ui.css after styles.css and before glass.css so the glass theme can override cards. (plan:2026-10-03-agent-a2ui)
- Keep one MessageProcessor per A2UI surface and feed it new messages instead of rebuilding it. (t-221)
- Choose an element's own panel from snapshot.host_params, never its kind; edit chart or table data only via the dock's host-data surface, sending DataEdit, no patch. (plan:2026-10-18-packs-lot2-base, t-1119)
- Take base and essentiel kinds in UI code only from ui/src/builtinKinds.ts, never as literals; build the add grid from Snapshot.add_packs and accepted kinds from free_slots. (plan:2026-10-19-packs-lot3-essentiel, plan:2026-10-23-packs-lot5-ui)
- Build buttons, icon toggles, selects and labelled fields with ui/src/primitives.tsx, shared with the A2UI catalog; no hand-written .btn or select.select. (t-982)

## Pitfalls

- Never call crypto.randomUUID in ui/src, it throws outside secure contexts; use randomId from randomId.ts. (plan:2026-10-09-suivis-t678-t733, t-695)
- Make every UI effect survive mount, unmount, then mount again, because main.tsx enables React StrictMode. (spec:2026-09-26-editeur-markdown-design)
- Send Content-Type application/json with every POST to oku-slides serve; its anti-CSRF guard rejects others. (plan:2026-09-26-agent-vocal)
- Do not capture Escape in Dictation for the agent; App decides via escapeAction, deselecting before cancelling a turn. (plan:2026-09-26-agent-vocal)
- Prune widget sub-tabs only against statuses of the current revision, never stale or other-pack statuses. (plan:2026-10-03-widgets-onglets)
- Call EditorView.requestMeasure when a hidden CodeMirror sub-tab becomes active again. (plan:2026-10-03-widgets-onglets)
- Set the cursor colour explicitly in every CodeMirror editor; the default black cursor is invisible on dark themes. (t-460)
- Do not rely on tsc -b to typecheck test files; ui/tsconfig.json excludes them and bun test only strips types, so they are never typechecked. (plan:2026-10-08-widget-tableau)
- Never free an A2UI surface in an effect cleanup; it cannot know whether the next message list extends the old one. (plan:2026-10-03-agent-a2ui, t-291)
- Implement drags with pointer events and draggable={false} images; the Tauri webview reserves HTML5 drag-and-drop for file drops. (t-819)

## Testing

- Never pass an async predicate to page.waitForFunction; it resolves at once, so poll from Node instead. (t-570)
- Read widget preview SVGs in e2e with widgetPreviewSvg; each new preview revokes the old blob URL. (t-1087)

## Settled decisions

- Parse rewrite text with textToParagraphs and external paste or dictation with literalToParagraphs, never on the server. (spec:2026-09-27-agent-onglets-design, t-162)
- Keep the main menu to present, open and help, with no save action (changes autosave after 1.5 s, new decks to a default folder); every other app action goes through an agent tool. (t-988, t-257)
- Show shortcuts as Cmd on macOS and as Ctrl, Shift, Alt elsewhere, in help and menus. (t-493)
- Use CodeMirror standardKeymap, never defaultKeymap, whose commands are meant for code editors. (spec:2026-09-26-editeur-markdown-design)
- Keep ListTable fully controlled: hold no local data except the draft of the cell being edited. (spec:2026-10-04-parametre-liste-design)
- Let unsaved keystrokes win over agent writes; an untouched field editor adopts them, a replacing commit warns; agent widget deletion discards them. (spec:2026-10-03-widgets-agent-a2ui-design, t-130)
- After a refused cut-paste move, paste a renewed-id copy only if the element is gone from its source slide. (plan:2026-10-10-deck-en-arbre)
- Derive editor spans and runs from the single lezer markdown parse in markdown.ts; no hand-written parser. (spec:2026-09-26-editeur-markdown-design, t-183)
- Read the number language with useNumberLang from NumberLangContext; never thread a numberLang prop through panels. (plan:2026-10-09-finitions-ids)
