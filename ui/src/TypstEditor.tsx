/**
 * Éditeur CodeMirror d'un fichier de widget : coloration Typst minimale,
 * diagnostics soulignés et marqués dans la marge, à leur ligne.
 *
 * Une vue par fichier : le parent la recrée par `key`. Une frappe efface
 * les diagnostics affichés ; les suivants arrivent après l'écriture, et
 * seulement s'ils portent sur le texte courant (`diagnosticsFor`).
 */
import { useEffect, useEffectEvent, useLayoutEffect, useRef } from "react";
import { history, historyKeymap, indentWithTab, standardKeymap } from "@codemirror/commands";
import type { Range } from "@codemirror/state";
import { Annotation, EditorState, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import type { DecorationSet, ViewUpdate } from "@codemirror/view";
import { Decoration, EditorView, GutterMarker, ViewPlugin, WidgetType, gutter, keymap, lineNumbers } from "@codemirror/view";
import type { WidgetDiagnostic } from "./api";
import { typstTokens } from "./typstHighlight";
import { diagnosticRange } from "./widgetDiagnostics";

const MARKS = new Map<string, Decoration>(
  ["comment", "string", "keyword", "hash", "param"].map((k) => [`cm-typ-${k}`, Decoration.mark({ class: `cm-typ-${k}` })] as const),
);

/** Décorations de coloration des lignes visibles. */
function highlight(view: EditorView): DecorationSet {
  const ranges: Range<Decoration>[] = [];
  for (const { from, to } of view.visibleRanges) {
    for (let pos = from; pos <= to; ) {
      const line = view.state.doc.lineAt(pos);
      for (const tk of typstTokens(line.text)) {
        const mark = MARKS.get(tk.className);
        if (mark && tk.to > tk.from) ranges.push(mark.range(line.from + tk.from, line.from + tk.to));
      }
      pos = line.to + 1;
    }
  }
  return Decoration.set(ranges, true);
}

const typstHighlight = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = highlight(view);
    }
    update(u: ViewUpdate) {
      if (u.docChanged || u.viewportChanged) this.decorations = highlight(u.view);
    }
  },
  { decorations: (v) => v.decorations },
);

/** Changement venu du parent, qui en sait le résultat : pas d'`onChange`. */
const quiet = Annotation.define<boolean>();

const setDiagnostics = StateEffect.define<WidgetDiagnostic[]>();

/** Les diagnostics affichés ; une frappe les efface. */
const diagnosticsField = StateField.define<WidgetDiagnostic[]>({
  create: () => [],
  update(value, tr) {
    for (const e of tr.effects) if (e.is(setDiagnostics)) return e.value;
    return tr.docChanged ? [] : value;
  },
});

const underline = Decoration.mark({ class: "cm-diag-underline" });

/** Marque d'un diagnostic sans caractère à souligner (ligne vide). */
class PointMark extends WidgetType {
  override eq() {
    return true;
  }
  override toDOM() {
    const el = document.createElement("span");
    el.className = "cm-diag-point";
    el.setAttribute("aria-hidden", "true");
    return el;
  }
}

const point = Decoration.widget({ widget: new PointMark(), side: 1 });

const diagnosticMarks = EditorView.decorations.compute([diagnosticsField], (state) => {
  const doc = state.doc.toString();
  const ranges = state
    .field(diagnosticsField)
    .map((d) => diagnosticRange(doc, d))
    .filter((r): r is NonNullable<typeof r> => r !== null)
    .map((r) => (r.to > r.from ? underline.range(r.from, r.to) : point.range(r.from)));
  return Decoration.set(ranges, true);
});

class DiagnosticMarker extends GutterMarker {
  constructor(readonly message: string) {
    super();
  }
  override toDOM() {
    const el = document.createElement("span");
    el.className = "cm-diag-marker";
    el.textContent = "●";
    el.title = this.message;
    return el;
  }
}

/** La marge : un repère par ligne qui porte un diagnostic, ses messages en titre. */
const diagnosticGutter = gutter({
  class: "cm-diag-gutter",
  markers: (view) => {
    const doc = view.state.doc;
    const byLine = new Map<number, string[]>();
    for (const d of view.state.field(diagnosticsField)) {
      if (d.line == null || d.line < 1 || d.line > doc.lines) continue;
      byLine.set(d.line, [...(byLine.get(d.line) ?? []), d.message]);
    }
    const builder = new RangeSetBuilder<GutterMarker>();
    for (const line of [...byLine.keys()].sort((a, b) => a - b)) {
      const at = doc.line(line).from;
      builder.add(at, at, new DiagnosticMarker((byLine.get(line) ?? []).join("\n")));
    }
    return builder.finish();
  },
});

interface Props {
  /** Texte à l'ouverture (lu une fois). */
  text: string;
  /** Diagnostics de ce fichier. */
  diagnostics: WidgetDiagnostic[];
  /** Le texte pour lequel ils ont été calculés ; autre que le texte courant : ignorés. */
  diagnosticsFor?: string | null;
  /** Nom accessible de la zone éditable. */
  label: string;
  /** Texte après chaque modification. */
  onChange: (text: string) => void;
  /**
   * Remplacement à faire dans le texte courant, à chaque nouvel objet reçu
   * après l'ouverture : la première occurrence de `find` devient `replace`,
   * comme une frappe (`onChange` suit), ou sans `onChange` si `silent` (le
   * parent sait déjà le texte obtenu). Sert au nom déclaré après un renommage.
   */
  rewrite?: { find: string; replace: string; silent?: boolean } | null;
}

export default function TypstEditor({ text, diagnostics, diagnosticsFor, label, onChange, rewrite }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const changed = useEffectEvent((t: string) => onChange(t));
  useLayoutEffect(() => {
    const parent = host.current;
    if (!parent) return;
    const v = new EditorView({
      parent,
      state: EditorState.create({
        doc: text,
        extensions: [
          lineNumbers(),
          diagnosticGutter,
          history(),
          keymap.of([indentWithTab, ...standardKeymap, ...historyKeymap]),
          typstHighlight,
          diagnosticsField,
          diagnosticMarks,
          EditorView.contentAttributes.of({ "aria-label": label }),
          EditorView.updateListener.of((u) => {
            if (u.docChanged && !u.transactions.some((tr) => tr.annotation(quiet))) changed(u.state.doc.toString());
          }),
        ],
      }),
    });
    view.current = v;
    return () => {
      v.destroy();
      view.current = null;
    };
    // Le texte n'est lu qu'à l'ouverture : le parent recrée la vue par `key`.
  }, []);
  useEffect(() => {
    const v = view.current;
    if (!v) return;
    // Calculés pour un texte dépassé (frappe depuis) : leurs positions mentiraient.
    if (diagnosticsFor != null && v.state.doc.toString() !== diagnosticsFor) return;
    v.dispatch({ effects: setDiagnostics.of(diagnostics) });
  }, [diagnostics, diagnosticsFor]);
  // Celui reçu à l'ouverture visait une vue précédente : ignoré.
  const applied = useRef(rewrite);
  useEffect(() => {
    const v = view.current;
    if (!v || !rewrite || rewrite === applied.current) return;
    applied.current = rewrite;
    const at = v.state.doc.toString().indexOf(rewrite.find);
    if (at < 0) return;
    v.dispatch({
      changes: { from: at, to: at + rewrite.find.length, insert: rewrite.replace },
      annotations: rewrite.silent ? quiet.of(true) : [],
    });
  }, [rewrite]);
  return <div className="typst-editor" ref={host} />;
}
