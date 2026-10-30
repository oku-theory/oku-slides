/**
 * CodeMirror near-rendering of the editing markdown (t-272).
 *
 * Bold, italic, links and highlights are displayed; the marks of a
 * construct are hidden while the selection does not touch it, and shown
 * dimmed otherwise. A list prefix becomes its bullet or its rank;
 * a heading keeps its hashes, dimmed, and grows.
 * The document stays the source markdown: only the decorations change.
 * The ranges come from `markdownLayout` and `blockPrefix`: the editor follows
 * the slide's rules.
 */
import type { Extension, Range } from "@codemirror/state";
import type { DecorationSet, EditorView, ViewUpdate } from "@codemirror/view";
import { Decoration, ViewPlugin, WidgetType } from "@codemirror/view";
import { LIGHT_BACKGROUND, luminance } from "./color";
import type { Construct } from "./markdown";
import { blockPrefix, markdownLayout } from "./markdown";

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

/** Analyse d'une ligne : décorations, constructions inline et préfixe de liste (positions dans la ligne). */
interface LineParts {
  decorations: LineDecoration[];
  constructs: Construct[];
  prefix: ReturnType<typeof blockPrefix>;
}

function lineParts(line: string, palette: string[]): LineParts {
  const prefix = blockPrefix(line);
  const { length } = prefix;
  const out: LineDecoration[] = [];
  if (prefix.heading) out.push({ from: 0, to: line.length, className: `cm-md-h${prefix.heading}` });
  if (length > 0) out.push({ from: 0, to: length, className: "cm-md-mark" });
  const layout = markdownLayout(line.slice(length));
  for (const s of layout.spans) {
    const from = s.from + length;
    const to = s.to + length;
    if (s.kind !== "highlight") {
      out.push({ from, to, className: `cm-md-${s.kind}` });
      continue;
    }
    const style = highlightStyle(palette, s.highlight ?? 1);
    out.push(style ? { from, to, className: "cm-md-highlight", style } : { from, to, className: "cm-md-highlight" });
  }
  const constructs = layout.constructs.map((c) => ({ from: c.from + length, to: c.to + length }));
  return { decorations: out, constructs, prefix };
}

/** Décorations d'une ligne : préfixe de liste, puis plages du markdown inline. */
export function lineDecorations(line: string, palette: string[]): LineDecoration[] {
  return lineParts(line, palette).decorations;
}

/** Plage de la sélection, en positions dans la ligne. */
export interface Selected {
  from: number;
  to: number;
}

/** Décoration du quasi-rendu : mise en forme, marque cachée, ou préfixe de liste remplacé par `marker`. */
export type PreviewDecoration = LineDecoration | { from: number; to: number; hide: true } | { from: number; to: number; marker: string };

/** La sélection touche `c` (bords compris : le curseur contre une marque la montre). */
const touches = (selection: readonly Selected[], c: Construct) => selection.some((r) => r.from <= c.to && r.to >= c.from);

/** La plus petite construction qui contient la marque `[from, to]`. */
function owner(constructs: readonly Construct[], from: number, to: number): Construct | undefined {
  let best: Construct | undefined;
  for (const c of constructs) {
    if (c.from <= from && to <= c.to && (!best || c.to - c.from < best.to - best.from)) best = c;
  }
  return best;
}

/**
 * Quasi-rendu d'une ligne : les marques (et l'adresse d'un lien) d'une construction que
 * `selection` ne touche pas sont cachées, les plages cachées voisines fusionnées ; un préfixe
 * de liste hors de la sélection devient `• ` ou `ordinal. `.
 */
export function linePreview(line: string, palette: string[], selection: readonly Selected[], ordinal: number): PreviewDecoration[] {
  return preview(lineParts(line, palette), selection, ordinal);
}

function preview({ decorations, constructs, prefix }: LineParts, selection: readonly Selected[], ordinal: number): PreviewDecoration[] {
  const out: PreviewDecoration[] = [];
  for (const d of decorations) {
    const isMark = d.className === "cm-md-mark" || d.className === "cm-md-url";
    if (!isMark) {
      out.push(d);
      continue;
    }
    if (prefix.length > 0 && d.from === 0 && d.to === prefix.length) {
      // The heading hashes stay visible: the level is information.
      if (prefix.heading) {
        out.push(d);
        continue;
      }
      const shown = touches(selection, d);
      out.push(shown ? d : { from: d.from, to: d.to, marker: prefix.list === "bullet" ? "• " : `${ordinal}. ` });
      continue;
    }
    const c = owner(constructs, d.from, d.to);
    if (c && touches(selection, c)) {
      out.push(d);
      continue;
    }
    const last = out.at(-1);
    if (last && "hide" in last && last.to === d.from) last.to = d.to;
    else out.push({ from: d.from, to: d.to, hide: true });
  }
  return out;
}

/**
 * Rang de l'item numéroté de la ligne `n` (1 à `n`) dans sa liste, comme au rendu : tout
 * autre paragraphe clôt la liste, le numéro écrit ne compte pas. `line(k)` : texte de la ligne k.
 */
export function listOrdinal(line: (k: number) => string, n: number): number {
  let k = n;
  while (k > 1 && blockPrefix(line(k - 1)).list === "number") k--;
  return n - k + 1;
}

/** Décorations sans style en ligne, une par classe. */
const PLAIN = new Map(
  ["mark", "url", "bold", "italic", "link", "highlight", "h1", "h2", "h3", "h4", "h5", "h6"].map((kind) => {
    const className = `cm-md-${kind}`;
    return [className, Decoration.mark({ class: className })] as const;
  }),
);

/** Marque cachée : le texte reste dans le document, hors de la vue. */
const HIDE = Decoration.replace({});

/** Puce ou rang d'un item de liste, à la place de son préfixe source. */
class MarkerWidget extends WidgetType {
  constructor(readonly text: string) {
    super();
  }
  eq(other: MarkerWidget) {
    return other.text === this.text;
  }
  toDOM() {
    const el = document.createElement("span");
    el.className = "cm-md-marker";
    el.textContent = this.text;
    return el;
  }
}

function decoration(d: PreviewDecoration): Decoration {
  if ("hide" in d) return HIDE;
  if ("marker" in d) return Decoration.replace({ widget: new MarkerWidget(d.marker) });
  const plain = d.style ? undefined : PLAIN.get(d.className);
  return plain ?? Decoration.mark({ class: d.className, attributes: d.style ? { style: d.style } : undefined });
}

/** Analyses de lignes gardées : au-delà, le cache repart de zéro. */
const CACHE_SIZE = 512;

/** Décorations des lignes visibles, pour la sélection courante ; `cache` garde l'analyse de chaque texte de ligne. */
function build(view: EditorView, palette: string[], cache: Map<string, LineParts>): DecorationSet {
  const ranges: Range<Decoration>[] = [];
  const { doc, selection } = view.state;
  const text = (k: number) => doc.line(k).text;
  for (const { from, to } of view.visibleRanges) {
    const first = doc.lineAt(from).number;
    const last = doc.lineAt(to).number;
    // Rang de l'item précédant la plage, puis compté ligne à ligne.
    let ordinal = first > 1 && blockPrefix(text(first - 1)).list === "number" ? listOrdinal(text, first - 1) : 0;
    for (let n = first; n <= last; n++) {
      const line = doc.line(n);
      let parts = cache.get(line.text);
      if (!parts) {
        if (cache.size >= CACHE_SIZE) cache.clear();
        parts = lineParts(line.text, palette);
        cache.set(line.text, parts);
      }
      ordinal = parts.prefix.list === "number" ? ordinal + 1 : 0;
      const local = selection.ranges
        .filter((r) => r.from <= line.to && r.to >= line.from)
        .map((r) => ({ from: r.from - line.from, to: r.to - line.from }));
      for (const d of preview(parts, local, ordinal)) {
        ranges.push(decoration(d).range(line.from + d.from, line.from + d.to));
      }
    }
  }
  return Decoration.set(ranges, true);
}

/** Extension CodeMirror : quasi-rendu du markdown, pastilles dans les couleurs de `palette`. */
export function markdownDecorations(palette: string[]): Extension {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      cache = new Map<string, LineParts>();
      constructor(view: EditorView) {
        this.decorations = build(view, palette, this.cache);
      }
      update(u: ViewUpdate) {
        if (u.docChanged || u.selectionSet || u.viewportChanged) this.decorations = build(u.view, palette, this.cache);
      }
    },
    { decorations: (v) => v.decorations },
  );
}
