/**
 * Éditeur de texte CodeMirror posé sur la slide.
 *
 * Champ `rich` : le markdown se met en forme pendant la saisie. Champ
 * `plain` : texte brut sur une seule ligne. Tab (⇧Tab) passe au champ
 * suivant (précédent). Le texte validé est celui que montre CodeMirror.
 */
import { useEffectEvent, useImperativeHandle, useLayoutEffect, useRef } from "react";
import { history, historyKeymap, insertNewline, standardKeymap } from "@codemirror/commands";
import type { ChangeSpec } from "@codemirror/state";
import { EditorState } from "@codemirror/state";
import type { KeyBinding } from "@codemirror/view";
import { EditorView, keymap } from "@codemirror/view";
import { markdownDecorations } from "./markdownDecorations";
import { applyEdit, markdownFormatKeys } from "./markdownEditing";
import { insertDictated } from "./textEdits";

/** Ce que la scène peut demander à l'éditeur ouvert. */
export interface TextEditorHandle {
  /** Insère un texte dicté à la sélection, avec les espaces qu'il faut. */
  insertAtCursor(text: string): void;
  /** Texte affiché. */
  text(): string;
  /** Valide comme Entrée ou la perte du focus (une seule fin par ouverture). */
  commit(): void;
}

interface TextEditorProps {
  /** Texte à l'ouverture (lu une seule fois). */
  initial: string;
  /** Champ `plain` (titre…) : texte brut sur une ligne, Entrée passe au suivant. Sinon markdown. */
  plain: boolean;
  /** Couleurs des pastilles (palette des graphiques du thème), lue à l'ouverture. */
  palette: string[];
  /** Nom accessible de la zone éditable. */
  label: string;
  /** Géométrie posée sur le texte rendu (police, taille, position). */
  style: React.CSSProperties;
  /** Texte après chaque modification. */
  onChange: (text: string) => void;
  /** Validation, avec le texte affiché. */
  onCommit: (text: string) => void;
  /** Abandon (Échap). */
  onCancel: () => void;
  /** Validation puis champ suivant (`1`) ou précédent (`-1`), avec le texte affiché. */
  onNext: (text: string, dir: 1 | -1) => void;
  /** Poignée de la scène : insertion de la dictée, texte affiché, validation. */
  ref?: React.Ref<TextEditorHandle>;
}

/**
 * Réglages de CodeMirror : il hérite de la géométrie posée sur l'hôte `.editor`.
 *
 * `&.cm-editor .cm-content` est au moins aussi précis que le thème de base
 * (`&light .cm-content` fixe un curseur noir) : ce thème l'emporte.
 */
const fitHost = EditorView.theme({
  "&": { backgroundColor: "transparent", color: "inherit" },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": { fontFamily: "inherit", lineHeight: "inherit", overflow: "visible" },
  "&.cm-editor .cm-content": { padding: "0", caretColor: "var(--accent)" },
  ".cm-line": { padding: "0", textAlign: "inherit" },
});

/**
 * Champ : un retour à la ligne inséré devient une espace.
 *
 * La transaction d'origine est gardée (historique, annotations) ; le
 * remplacement s'y ajoute, de même longueur.
 */
export const singleLine = EditorState.transactionFilter.of((tr) => {
  let newline = false;
  tr.changes.iterChanges((_fromA, _toA, _fromB, _toB, inserted) => {
    if (inserted.lines > 1) newline = true;
  });
  if (!newline) return tr;
  const text = tr.newDoc.toString();
  const spaces: ChangeSpec[] = [];
  for (let i = text.indexOf("\n"); i >= 0; i = text.indexOf("\n", i + 1)) {
    spaces.push({ from: i, to: i + 1, insert: " " });
  }
  return [tr, { changes: spaces, sequential: true }];
});

/** Ne fait rien : la touche ne tombe pas dans la mise en forme native. */
const nothing = () => true;

/** Éditeur CodeMirror d'une zone de texte ; se remonte pour chaque cible. */
export default function TextEditor({ ref, ...props }: TextEditorProps) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  // Validation de l'ouverture en cours, gardée par sa fin unique.
  const validate = useRef<(() => void) | null>(null);
  const change = useEffectEvent((text: string) => props.onChange(text));
  const commit = useEffectEvent((text: string) => props.onCommit(text));
  const cancel = useEffectEvent(() => props.onCancel());
  const advance = useEffectEvent((text: string, dir: 1 | -1) => props.onNext(text, dir));

  useLayoutEffect(() => {
    const parent = host.current;
    if (!parent) return;
    const { initial, plain, palette, label } = props;
    // Une seule fin par montage ; StrictMode monte deux fois, l'état reste local à l'effet.
    let done = false;
    const finish = (view: EditorView, validate: boolean) => {
      if (done) return true;
      done = true;
      if (validate) commit(view.state.doc.toString());
      else cancel();
      return true;
    };
    const next = (view: EditorView, dir: 1 | -1) => {
      if (done) return true;
      done = true;
      advance(view.state.doc.toString(), dir);
      return true;
    };
    // CodeMirror ignore déjà l'Entrée qui confirme une conversion IME (y compris
    // celle que WebKit envoie juste après compositionend) : pas de garde ici.
    const keys: KeyBinding[] = [
      { key: "Escape", run: (view) => finish(view, false) },
      { key: "Mod-Enter", run: (view) => finish(view, true) },
      { key: "Mod-u", run: nothing },
      { key: "Tab", run: (view) => next(view, 1), shift: (view) => next(view, -1) },
      ...(plain
        ? [
            { key: "Enter", run: (view: EditorView) => next(view, 1), shift: (view: EditorView) => next(view, 1) },
            { key: "Mod-b", run: nothing },
            { key: "Mod-i", run: nothing },
            { key: "Mod-k", run: nothing },
          ]
        : [
            { key: "Enter", run: insertNewline, shift: insertNewline },
            ...markdownFormatKeys(),
          ]),
    ];
    const doc = plain ? initial.replaceAll("\n", " ") : initial;
    // Pas de limite de hauteur : l'éditeur grandit avec le texte (overflow visible).
    // Collage : CodeMirror ne lit que text/plain.
    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc,
        selection: { anchor: doc.length },
        extensions: [
          // Nos touches d'abord : ⌘Entrée valide au lieu d'insérer une ligne.
          keymap.of(keys),
          history(),
          keymap.of([...historyKeymap, ...standardKeymap]),
          fitHost,
          EditorView.lineWrapping,
          EditorView.perLineTextDirection.of(true),
          EditorView.contentAttributes.of({
            "aria-label": label,
            "aria-multiline": plain ? "false" : "true",
            spellcheck: "true",
            autocorrect: "on",
          }),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) change(u.state.doc.toString());
          }),
          EditorView.domEventHandlers({
            blur: (_event, target) => {
              finish(target, true);
            },
          }),
          plain ? singleLine : markdownDecorations(palette),
        ],
      }),
    });
    viewRef.current = view;
    validate.current = () => finish(view, true);
    view.focus();
    return () => {
      done = true;
      viewRef.current = null;
      validate.current = null;
      view.destroy();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- monté une fois par cible (la scène donne une `key`)
  }, []);

  useImperativeHandle(ref, () => ({
    insertAtCursor(text: string) {
      const view = viewRef.current;
      if (!view) return;
      applyEdit(view, (s, a, b) => insertDictated(s, a, b, text), "input.dictation");
      view.focus();
    },
    text() {
      return viewRef.current?.state.doc.toString() ?? "";
    },
    commit() {
      validate.current?.();
    },
  }), []);

  return (
    <div
      ref={host}
      className="editor"
      style={props.style}
      onPointerDown={(e) => e.stopPropagation()}
      onMouseDown={(e) => {
        // Un clic dans la marge (hors du texte) garde le focus : il ne valide pas.
        if (!(e.target as HTMLElement).closest(".cm-content")) {
          e.preventDefault();
          viewRef.current?.focus();
        }
      }}
      onKeyDown={(e) => e.stopPropagation()}
    />
  );
}
