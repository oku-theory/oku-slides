/**
 * Markdown inline d'édition → runs : l'inverse de `runs_to_markdown` côté Rust.
 *
 * Analyse CommonMark de `@lezer/markdown`, blocs désactivés (une ligne de
 * l'éditeur est un paragraphe) : `**gras**`, `*italique*` ou `_italique_`,
 * `[texte](url)`, `\` devant une ponctuation, plus les pastilles
 * `==texte==` / `==texte=={n}` (n-ième couleur de la palette du thème, voir
 * `markdownHighlight.ts`). Le même parcours de l'arbre donne les plages
 * positionnées que l'éditeur met en forme (`markdownSpans`) ; `listPrefix`
 * est la règle unique des préfixes de liste ; `escapeInline` écrit du texte brut
 * pour qu'il se relise tel quel.
 */
import type { SyntaxNode } from "@lezer/common";
import { parser as commonmark } from "@lezer/markdown";
import type { Paragraph, Run } from "./api";
import { Highlight } from "./markdownHighlight";

interface Format {
  bold: boolean;
  italic: boolean;
  link?: string;
  highlight?: number;
}

/** Nature d'une plage : délimiteur, mise en forme ou adresse de lien. */
export type SpanKind = "mark" | "bold" | "italic" | "link" | "url" | "highlight";

/** Plage d'une ligne de markdown (indices de chaîne), mise en forme par l'éditeur. */
export interface Span {
  from: number;
  to: number;
  kind: SpanKind;
  highlight?: number;
}

/** Run sans champs superflus : seuls les formats présents apparaissent. */
function run(t: string, f: Format): Run {
  const r: Run = { t };
  if (f.bold) r.bold = true;
  if (f.italic) r.italic = true;
  if (f.link !== undefined) r.link = f.link;
  if (f.highlight !== undefined) r.highlight = f.highlight;
  return r;
}

/** Runs voisins de même format fusionnés, runs vides retirés. */
function merge(runs: Run[]): Run[] {
  const out: Run[] = [];
  for (const r of runs) {
    if (r.t === "") continue;
    const last = out.at(-1);
    const same =
      !!last?.bold === !!r.bold && !!last?.italic === !!r.italic && last?.link === r.link && last?.highlight === r.highlight;
    if (last && same) {
      last.t += r.t;
    } else {
      out.push({ ...r });
    }
  }
  return out;
}

/** Blocs et inlines CommonMark hors du dialecte d'édition. */
const DISABLED = [
  "LinkReference",
  "IndentedCode",
  "FencedCode",
  "Blockquote",
  "HorizontalRule",
  "BulletList",
  "OrderedList",
  "ATXHeading",
  "HTMLBlock",
  "SetextHeading",
  "HTMLTag",
  "Autolink",
  "InlineCode",
  "Image",
  "Entity",
  "HardBreak",
];

const parser = commonmark.configure([Highlight, { remove: DISABLED }]);

/** `{0}`..`{255}` : la couleur est un u8 côté Rust ({0} prend la dernière, comme Typst). */
const COLOR = /^\{(\d|[1-9]\d|1\d\d|2[0-4]\d|25[0-5])\}/;

/** Étendue d'une construction inline (gras, italique, pastille, lien, échappement), marques comprises. */
export interface Construct {
  from: number;
  to: number;
}

/** Runs, plages et constructions d'une ligne, remplis par le parcours de l'arbre. */
interface Walk {
  line: string;
  runs: Run[];
  spans: Span[];
  constructs: Construct[];
}

function span(w: Walk, from: number, to: number, kind: SpanKind, highlight?: number) {
  w.spans.push(highlight === undefined ? { from, to, kind } : { from, to, kind, highlight });
}

/** Enfants de `node` compris dans `[from, to]` ; le texte entre eux est du texte au format `f`. */
function inline(w: Walk, node: SyntaxNode, from: number, to: number, f: Format) {
  let pos = from;
  for (let c = node.firstChild; c; c = c.nextSibling) {
    if (c.from < pos || c.to > to) continue;
    w.runs.push(run(w.line.slice(pos, c.from), f));
    pos = element(w, c, f);
  }
  w.runs.push(run(w.line.slice(pos, to), f));
}

/** Un nœud inline : ajoute runs et plages, rend la position où reprend le texte. */
function element(w: Walk, n: SyntaxNode, f: Format): number {
  switch (n.name) {
    case "Escape":
      w.constructs.push({ from: n.from, to: n.to });
      span(w, n.from, n.from + 1, "mark");
      w.runs.push(run(w.line.slice(n.from + 1, n.to), f));
      return n.to;
    case "Emphasis":
    case "StrongEmphasis": {
      const open = n.firstChild;
      const close = n.lastChild;
      if (!open || !close) return n.from;
      const kind = n.name === "Emphasis" ? "italic" : "bold";
      w.constructs.push({ from: n.from, to: n.to });
      span(w, open.from, open.to, "mark");
      span(w, open.to, close.from, kind);
      inline(w, n, open.to, close.from, kind === "bold" ? { ...f, bold: true } : { ...f, italic: true });
      span(w, close.from, close.to, "mark");
      return n.to;
    }
    case "Highlight": {
      const open = n.firstChild;
      const close = n.lastChild;
      if (!open || !close) return n.from;
      const color = COLOR.exec(w.line.slice(n.to));
      const highlight = color ? Number(color[1]) : 1;
      const end = n.to + (color?.[0].length ?? 0);
      w.constructs.push({ from: n.from, to: end });
      span(w, open.from, open.to, "mark");
      span(w, open.to, close.from, "highlight", highlight);
      inline(w, n, open.to, close.from, { ...f, highlight });
      span(w, close.from, end, "mark");
      return end;
    }
    case "Link": {
      // Sans adresse (`[a]`, `[a][b]`, `[a]()`), ce n'est pas un lien : texte littéral.
      // Un titre (`[a](u "t")`) ou une adresse entre chevrons (`[a](<u>)`) n'ont
      // pas d'écriture côté Rust : texte littéral aussi, plutôt qu'un titre perdu.
      const url = n.getChild("URL");
      const marks = n.getChildren("LinkMark");
      if (!url || marks.length < 2 || n.getChild("LinkTitle") || w.line[url.from] === "<") return n.from;
      const [open, close] = marks;
      w.constructs.push({ from: n.from, to: n.to });
      span(w, open.from, open.to, "mark");
      span(w, open.to, close.from, "link");
      inline(w, n, open.to, close.from, { ...f, link: w.line.slice(url.from, url.to) });
      span(w, close.from, url.from, "mark");
      span(w, url.from, url.to, "url");
      span(w, url.to, n.to, "mark");
      return n.to;
    }
    default:
      // Document, Paragraph : conteneurs transparents.
      inline(w, n, n.from, n.to, f);
      return n.to;
  }
}

function walk(line: string): Walk {
  const w: Walk = { line, runs: [], spans: [], constructs: [] };
  inline(w, parser.parse(line).topNode, 0, line.length, { bold: false, italic: false });
  return w;
}

/** Une ligne de markdown inline en runs ; une ligne vide donne un run vide. */
export function markdownToRuns(line: string): Run[] {
  const runs = merge(walk(line).runs);
  return runs.length > 0 ? runs : [{ t: "" }];
}

/** Plages non vides d'une ligne, l'englobante avant l'englobée. */
export function markdownSpans(line: string): Span[] {
  return markdownLayout(line).spans;
}

/**
 * Plages (comme `markdownSpans`) et constructions d'une ligne, d'une seule analyse :
 * l'éditeur montre les marques d'une construction quand la sélection la touche.
 */
export function markdownLayout(line: string): { spans: Span[]; constructs: Construct[] } {
  const w = walk(line);
  return {
    spans: w.spans.filter((s) => s.to > s.from).sort((a, b) => a.from - b.from || b.to - a.to),
    constructs: w.constructs,
  };
}

/** Préfixe de liste d'une ligne : `- ` (puce), `1. ` (numéro), sinon rien. */
export function listPrefix(line: string): { list?: NonNullable<Paragraph["list"]>; length: number } {
  if (line.startsWith("- ")) return { list: "bullet", length: 2 };
  const num = /^\d+\. /.exec(line);
  return num ? { list: "number", length: num[0].length } : { length: 0 };
}

/** Classe d'un caractère contre un délimiteur, comme `class` côté Rust (Lezer : une unité UTF-16). */
function charClass(c: string | undefined): "space" | "punct" | "alnum" {
  if (c === undefined || /\s/u.test(c)) return "space";
  if ((c.codePointAt(0) ?? 0) <= 0xffff && /[\p{P}\p{S}]/u.test(c)) return "punct";
  return "alnum";
}

/**
 * Texte brut d'une ligne écrit en markdown d'édition qui se relit tel quel : le miroir de
 * `escape_inline` côté Rust (fixture partagée `fixtures/markdown.json`). `\`, `*`, `[`, `]`
 * sont échappés, `_` sauf au milieu d'un mot, `=` contre un autre `=` ou au bord de la ligne.
 */
export function escapeInline(line: string): string {
  const chars = [...line];
  const alnum = (j: number) => j >= 0 && charClass(chars[j]) === "alnum";
  return chars
    .map((c, i) => {
      const intraword = alnum(i - 1) && alnum(i + 1);
      const equals = [i - 1, i + 1].some((j) => chars[j] === undefined || chars[j] === "=");
      const escaped = c === "\\" || c === "*" || c === "[" || c === "]" || (c === "_" && !intraword) || (c === "=" && equals);
      return escaped ? `\\${c}` : c;
    })
    .join("");
}

/** Adresse telle que `paragraphs_to_text` l'écrit : espace, parenthèses et `\` encodés, ainsi qu'un `<` initial. */
export const encodeLink = (url: string) =>
  url
    .replaceAll(" ", "%20")
    .replaceAll("(", "%28")
    .replaceAll(")", "%29")
    .replaceAll("\\", "%5C")
    .replace(/^</, "%3C");
