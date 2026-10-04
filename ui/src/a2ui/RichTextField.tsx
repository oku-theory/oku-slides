// Texte riche d'une carte de réglages : CodeMirror multiligne, markdown
// d'édition mis en forme à la frappe (comme l'éditeur de la scène). Validé à
// la perte du focus ou ⌘↩ : des paragraphes, construits à partir de ceux
// d'origine (couleur et taille d'une ligne inchangée gardées) ; Échap rétablit.

import { useEffect, useLayoutEffect, useRef } from "react";
import { history, historyKeymap, standardKeymap } from "@codemirror/commands";
import type { Extension } from "@codemirror/state";
import { Compartment, EditorState } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import type { Paragraph } from "../api";
import { textToParagraphs } from "../deck";
import { markdownDecorations } from "../markdownDecorations";
import { markdownFormatKeys } from "../markdownEditing";

export interface RichTextFieldProps {
  label: string;
  /** Markdown d'édition montré. */
  value: string;
  /** Paragraphes d'origine, alignés ligne à ligne sur `value`. */
  source: Paragraph[];
  palette: string[];
  /** Validation : le markdown et ses paragraphes. */
  onCommit: (markdown: string, paragraphs: Paragraph[]) => void;
}

/** L'éditeur monté, et sa remise à un texte avec un historique vierge. */
interface Editor {
  view: EditorView;
  reset: (doc: string) => void;
}

/** Champ riche multiligne d'une carte. */
export function RichTextField({ label, value, source, palette, onCommit }: RichTextFieldProps) {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<Editor | null>(null);
  const looks = useRef(new Compartment());
  const shown = useRef({ text: value, paragraphs: source });
  const commitRef = useRef(onCommit);
  useLayoutEffect(() => {
    commitRef.current = onCommit;
  });

  useEffect(() => {
    const parent = host.current;
    if (!parent) return;
    const commit = (v: EditorView) => {
      const text = v.state.doc.toString();
      if (text === shown.current.text) return true;
      const paragraphs = textToParagraphs(text, shown.current);
      shown.current = { text, paragraphs };
      commitRef.current(text, paragraphs);
      return true;
    };
    // Un nouvel état plutôt qu'un remplacement du texte : l'historique de la
    // saisie abandonnée ne doit pas se rejouer sur le texte rétabli.
    const state = (doc: string, current: Extension) =>
      EditorState.create({
        doc,
        extensions: [
          keymap.of([
            { key: "Mod-Enter", run: (target) => (target.contentDOM.blur(), true) },
            { key: "Escape", run: revert },
            ...markdownFormatKeys(),
          ]),
          history(),
          keymap.of([...historyKeymap, ...standardKeymap]),
          EditorView.lineWrapping,
          looks.current.of(current),
          EditorView.domEventHandlers({ blur: (_e, target) => void commit(target) }),
        ],
      });
    const reset = (doc: string) => v.setState(state(doc, looks.current.get(v.state) ?? []));
    const revert = () => {
      reset(shown.current.text);
      v.contentDOM.blur();
      return true;
    };
    const v = new EditorView({ parent, state: state(shown.current.text, []) });
    editor.current = { view: v, reset };
    return () => {
      // Démonté pendant une saisie : le texte tapé part quand même.
      if (v.hasFocus) commit(v);
      editor.current = null;
      v.destroy();
    };
  }, []);

  // Libellé et palette suivent la langue et le thème ; la palette par son
  // contenu (l'appelant peut en passer une copie à chaque rendu).
  const paletteKey = palette.join(",");
  useEffect(() => {
    editor.current?.view.dispatch({
      effects: looks.current.reconfigure([
        markdownDecorations(palette),
        EditorView.contentAttributes.of({ "aria-label": label, "aria-multiline": "true" }),
      ]),
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `palette` suit `paletteKey`.
  }, [label, paletteKey]);

  // Une nouvelle valeur de la session, hors saisie.
  useEffect(() => {
    const e = editor.current;
    if (!e || e.view.hasFocus || value === shown.current.text) return;
    shown.current = { text: value, paragraphs: source };
    e.reset(value);
  }, [value, source]);

  return (
    <div className="note-field rich-field" onKeyDown={(e) => e.stopPropagation()}>
      <span className="a2ui-hint">{label}</span>
      <div ref={host} className="rich-field-editor" />
    </div>
  );
}
