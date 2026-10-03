/**
 * Coloration Typst minimale, ligne par ligne : commentaires, chaînes,
 * mots-clés, `#` et descripteurs `p.*`. Pas d'analyse : un commentaire
 * `/* … *\/` sur plusieurs lignes n'est vu que sur sa première.
 */

/** Une plage colorée d'une ligne. */
export interface TypstToken {
  from: number;
  to: number;
  className: string;
}

const KEYWORDS = new Set([
  "let", "set", "show", "import", "include", "if", "else", "for", "in", "while", "return",
  "context", "none", "auto", "true", "false", "and", "or", "not", "as", "break", "continue",
]);

/** Les plages colorées de `line`, dans l'ordre. */
export function typstTokens(line: string): TypstToken[] {
  const out: TypstToken[] = [];
  const push = (from: number, length: number, kind: string) => out.push({ from, to: from + length, className: `cm-typ-${kind}` });
  let i = 0;
  while (i < line.length) {
    const rest = line.slice(i);
    if (rest.startsWith("//")) {
      push(i, line.length - i, "comment");
      break;
    }
    const block = rest.match(/^\/\*.*?(\*\/|$)/);
    const string = rest.match(/^"(?:[^"\\]|\\.)*"?/);
    const param = rest.match(/^p\.[a-z]+/);
    const word = rest.match(/^[A-Za-z_][\w-]*/);
    const glued = i > 0 && /[\w.-]/.test(line[i - 1] ?? "");
    if (block) {
      push(i, block[0].length, "comment");
      i += block[0].length;
    } else if (string) {
      push(i, string[0].length, "string");
      i += string[0].length;
    } else if (rest[0] === "#") {
      push(i, 1, "hash");
      i += 1;
    } else if (param && !glued) {
      push(i, param[0].length, "param");
      i += param[0].length;
    } else if (word) {
      if (KEYWORDS.has(word[0]) && !glued) push(i, word[0].length, "keyword");
      i += word[0].length;
    } else {
      i += 1;
    }
  }
  return out;
}
