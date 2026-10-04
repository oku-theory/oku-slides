// Édition du markdown d'édition partagée par l'éditeur de la scène et le
// champ riche des cartes : une édition pure appliquée en un pas d'annulation,
// et les touches de mise en forme (⌘B, ⌘I, ⌘K).

import { isolateHistory } from "@codemirror/commands";
import type { EditorView, KeyBinding } from "@codemirror/view";
import type { Edit } from "./textEdits";
import { insertLink, minimalChange, toggleEmphasis } from "./textEdits";

/** Applique une édition pure : seul le texte changé, en un pas d'annulation à part. */
export function applyEdit(
  view: EditorView,
  edit: (value: string, start: number, end: number) => Edit,
  userEvent: string,
): boolean {
  const { from, to } = view.state.selection.main;
  const before = view.state.doc.toString();
  const e = edit(before, from, to);
  view.dispatch({
    changes: minimalChange(before, e.value),
    selection: { anchor: e.start, head: e.end },
    userEvent,
    annotations: isolateHistory.of("full"),
  });
  return true;
}

/** ⌘B, ⌘I, ⌘K sur le markdown d'édition. */
export function markdownFormatKeys(): KeyBinding[] {
  const format = (edit: (value: string, start: number, end: number) => Edit) => (view: EditorView) =>
    applyEdit(view, edit, "input.format");
  return [
    { key: "Mod-b", run: format((s, a, b) => toggleEmphasis(s, a, b, "bold")) },
    { key: "Mod-i", run: format((s, a, b) => toggleEmphasis(s, a, b, "italic")) },
    { key: "Mod-k", run: format(insertLink) },
  ];
}
