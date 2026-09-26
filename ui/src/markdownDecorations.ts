/**
 * Décorations CodeMirror du markdown d'édition.
 *
 * Les marques restent visibles mais atténuées ; gras, italique, liens et
 * pastilles s'affichent. Les plages viennent de `markdownSpans` et
 * `listPrefix` : l'éditeur suit les règles de la slide.
 */
import type { Extension, Range } from "@codemirror/state";
import type { DecorationSet, EditorView, ViewUpdate } from "@codemirror/view";
import { Decoration, ViewPlugin } from "@codemirror/view";
import { LIGHT_BACKGROUND, luminance } from "./color";
import { listPrefix, markdownSpans } from "./markdown";

/** Décoration d'une plage de ligne : classe CSS, style en ligne pour une pastille. */
export interface LineDecoration {
  from: number;
  to: number;
  className: string;
  style?: string;
}

/** Seules des couleurs hexadécimales entrent dans un attribut `style`. */
const HEX = /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i;

/** Fond de la n-ième couleur (modulo positif, comme Typst) et encre lisible dessus. */
function highlightStyle(palette: string[], n: number): string | undefined {
  const len = palette.length;
  if (len === 0) return undefined;
  const bg = palette[(((n - 1) % len) + len) % len];
  if (!HEX.test(bg)) return undefined;
  const l = luminance(bg);
  const ink = l !== null && l > LIGHT_BACKGROUND ? "var(--t-ink)" : "var(--t-bg)";
  return `background-color: ${bg}; color: ${ink}`;
}

/** Décorations d'une ligne : préfixe de liste, puis plages du markdown inline. */
export function lineDecorations(line: string, palette: string[]): LineDecoration[] {
  const { length } = listPrefix(line);
  const out: LineDecoration[] = length > 0 ? [{ from: 0, to: length, className: "cm-md-mark" }] : [];
  for (const s of markdownSpans(line.slice(length))) {
    const from = s.from + length;
    const to = s.to + length;
    if (s.kind !== "highlight") {
      out.push({ from, to, className: `cm-md-${s.kind}` });
      continue;
    }
    const style = highlightStyle(palette, s.highlight ?? 1);
    out.push(style ? { from, to, className: "cm-md-highlight", style } : { from, to, className: "cm-md-highlight" });
  }
  return out;
}

/** Décorations sans style en ligne, une par classe. */
const PLAIN = new Map(
  ["mark", "url", "bold", "italic", "link", "highlight"].map((kind) => {
    const className = `cm-md-${kind}`;
    return [className, Decoration.mark({ class: className })] as const;
  }),
);

function decoration(d: LineDecoration): Decoration {
  const plain = d.style ? undefined : PLAIN.get(d.className);
  return plain ?? Decoration.mark({ class: d.className, attributes: d.style ? { style: d.style } : undefined });
}

/** Décorations de tout le document (un texte de slide est court). */
function build(view: EditorView, palette: string[]): DecorationSet {
  const ranges: Range<Decoration>[] = [];
  const doc = view.state.doc;
  for (let n = 1; n <= doc.lines; n++) {
    const line = doc.line(n);
    for (const d of lineDecorations(line.text, palette)) {
      ranges.push(decoration(d).range(line.from + d.from, line.from + d.to));
    }
  }
  return Decoration.set(ranges, true);
}

/** Extension CodeMirror : markdown mis en forme, pastilles dans les couleurs de `palette`. */
export function markdownDecorations(palette: string[]): Extension {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      constructor(view: EditorView) {
        this.decorations = build(view, palette);
      }
      update(u: ViewUpdate) {
        if (u.docChanged) this.decorations = build(u.view, palette);
      }
    },
    { decorations: (v) => v.decorations },
  );
}
