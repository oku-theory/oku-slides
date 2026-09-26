/**
 * Markdown inline d'édition → runs : l'inverse de `runs_to_markdown` côté Rust.
 *
 * `**gras**`, `*italique*` ou `_italique_`, `***les deux***`, `[texte](url)`,
 * `==pastille==` ou `==pastille=={n}` (n-ième couleur de la palette du thème) ;
 * `\` échappe une ponctuation. Un délimiteur sans fermeture reste du texte.
 * Le même parcours donne les plages positionnées que l'éditeur met en forme
 * (`markdownSpans`) ; `listPrefix` est la règle unique des préfixes de liste.
 */
import type { Paragraph, Run } from "./api";

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

const PUNCTUATION = /[!-/:-@[-`{-~]/;
const isWord = (c: string | undefined) => c !== undefined && /[\p{L}\p{N}]/u.test(c);

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

/** Longueur de la suite de `ch` qui commence en `i`. */
function repeated(s: string, i: number, end: number, ch: string): number {
  let n = 0;
  while (i + n < end && s[i + n] === ch) n++;
  return n;
}

/** Première position ≥ `from` (hors échappements) où `match` est vrai, sinon -1. */
function scan(s: string, from: number, end: number, match: (j: number) => number | null): number {
  for (let j = from; j < end; ) {
    if (s[j] === "\\" && j + 1 < end) {
      j += 2;
      continue;
    }
    const step = match(j);
    if (step === null) return j;
    j += step;
  }
  return -1;
}

/** Fermeture d'une suite de `n` fois `ch` : une suite d'exactement `n` fois `ch`. */
function closing(s: string, from: number, end: number, ch: string, n: number): number {
  return scan(s, from, end, (j) => {
    if (s[j] !== ch) return 1;
    const m = repeated(s, j, end, ch);
    return m === n ? null : m;
  });
}

/** Fermeture d'un `_` : un `_` qui n'est pas suivi d'une lettre. */
function closingUnderscore(s: string, from: number, end: number): number {
  return scan(s, from, end, (j) => (s[j] === "_" && !isWord(s[j + 1]) ? null : 1));
}

/** `[texte](url)` qui commence en `i` : fin du texte, adresse et fin du lien. */
function link(s: string, i: number, end: number): { textEnd: number; url: string; end: number } | null {
  const textEnd = scan(s, i + 1, end, (j) => (s[j] === "]" ? null : 1));
  if (textEnd < 0 || s[textEnd + 1] !== "(") return null;
  const close = s.indexOf(")", textEnd + 2);
  if (close < 0 || close >= end || close === textEnd + 2) return null;
  return { textEnd, url: s.slice(textEnd + 2, close), end: close + 1 };
}

function parse(s: string, start: number, end: number, f: Format, out: Run[], spans?: Span[]) {
  let text = "";
  const flush = () => {
    out.push(run(text, f));
    text = "";
  };
  const span = (from: number, to: number, kind: SpanKind, highlight?: number) =>
    spans?.push(highlight === undefined ? { from, to, kind } : { from, to, kind, highlight });
  let i = start;
  while (i < end) {
    const c = s[i];
    if (c === "\\" && i + 1 < end && PUNCTUATION.test(s[i + 1])) {
      span(i, i + 1, "mark");
      text += s[i + 1];
      i += 2;
      continue;
    }
    if (c === "*") {
      const n = repeated(s, i, end, "*");
      const close = n <= 3 ? closing(s, i + n, end, "*", n) : -1;
      if (close > i + n) {
        flush();
        span(i, i + n, "mark");
        if (n >= 2) span(i + n, close, "bold");
        if (n !== 2) span(i + n, close, "italic");
        span(close, close + n, "mark");
        parse(s, i + n, close, { ...f, bold: f.bold || n >= 2, italic: f.italic || n !== 2 }, out, spans);
        i = close + n;
      } else {
        text += s.slice(i, i + n);
        i += n;
      }
      continue;
    }
    if (c === "=" && repeated(s, i, end, "=") === 2) {
      const close = closing(s, i + 2, end, "=", 2);
      if (close > i + 2) {
        // `{1}`..`{255}` seulement : la couleur est un u8 non nul côté Rust.
        const color = /^\{([1-9]\d?|1\d\d|2[0-4]\d|25[0-5])\}/.exec(s.slice(close + 2, end));
        const highlight = color ? Number(color[1]) : 1;
        const after = close + 2 + (color?.[0].length ?? 0);
        flush();
        span(i, i + 2, "mark");
        span(i + 2, close, "highlight", highlight);
        span(close, after, "mark");
        parse(s, i + 2, close, { ...f, highlight }, out, spans);
        i = after;
        continue;
      }
    }
    if (c === "_" && !isWord(s[i - 1])) {
      const close = closingUnderscore(s, i + 1, end);
      if (close > i + 1) {
        flush();
        span(i, i + 1, "mark");
        span(i + 1, close, "italic");
        span(close, close + 1, "mark");
        parse(s, i + 1, close, { ...f, italic: true }, out, spans);
        i = close + 1;
        continue;
      }
    }
    if (c === "[") {
      const l = link(s, i, end);
      if (l) {
        flush();
        span(i, i + 1, "mark");
        span(i + 1, l.textEnd, "link");
        span(l.textEnd, l.textEnd + 2, "mark");
        span(l.textEnd + 2, l.end - 1, "url");
        span(l.end - 1, l.end, "mark");
        parse(s, i + 1, l.textEnd, { ...f, link: l.url }, out, spans);
        i = l.end;
        continue;
      }
    }
    text += c;
    i++;
  }
  flush();
}

/** Une ligne de markdown inline en runs ; une ligne vide donne un run vide. */
export function markdownToRuns(line: string): Run[] {
  const out: Run[] = [];
  parse(line, 0, line.length, { bold: false, italic: false }, out);
  const runs = merge(out);
  return runs.length > 0 ? runs : [{ t: "" }];
}

/** Plages non vides d'une ligne, l'englobante avant l'englobée. */
export function markdownSpans(line: string): Span[] {
  const spans: Span[] = [];
  parse(line, 0, line.length, { bold: false, italic: false }, [], spans);
  return spans.filter((s) => s.to > s.from).sort((a, b) => a.from - b.from || b.to - a.to);
}

/** Préfixe de liste d'une ligne : `- ` (puce), `1. ` (numéro), sinon rien. */
export function listPrefix(line: string): { list?: NonNullable<Paragraph["list"]>; length: number } {
  if (line.startsWith("- ")) return { list: "bullet", length: 2 };
  const num = /^\d+\. /.exec(line);
  return num ? { list: "number", length: num[0].length } : { length: 0 };
}

/** Adresse telle que `paragraphs_to_text` l'écrit : espace et parenthèses encodés. */
const encodeLink = (url: string | undefined) =>
  url?.replaceAll(" ", "%20").replaceAll("(", "%28").replaceAll(")", "%29");

/** Même texte et même mise en forme markdown (couleur et taille ignorées). */
export function sameFormatting(a: Run[], b: Run[]): boolean {
  const key = (runs: Run[]) =>
    JSON.stringify(
      merge(
        runs.map((r) =>
          run(r.t, { bold: !!r.bold, italic: !!r.italic, link: encodeLink(r.link), highlight: r.highlight }),
        ),
      ),
    );
  return key(a) === key(b);
}
