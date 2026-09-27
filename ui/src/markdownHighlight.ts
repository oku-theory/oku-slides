/**
 * Extension @lezer/markdown des pastilles : `==texte==`.
 *
 * `==` n'est pas du CommonMark : ses règles sont les nôtres, plus simples
 * que le « flanking » de `*`. Il ouvre s'il n'est pas suivi d'une espace et
 * ferme s'il n'est pas précédé d'une espace, si bien qu'une pastille en gras
 * collée à une lettre (`上==**a**==です`) s'écrit. Une suite de trois `=`
 * n'ouvre ni ne ferme rien ; un `\=` échappé juste avant ne compte pas.
 * La couleur `{n}` qui suit la fermeture est lue par `markdown.ts`.
 */
import type { MarkdownConfig } from "@lezer/markdown";

const EQUALS = 61;
const BACKSLASH = 92;
const DELIMITER = { resolve: "Highlight", mark: "HighlightMark" };

/** Nœuds `Highlight` et `HighlightMark` pour `parser.configure`. */
export const Highlight: MarkdownConfig = {
  defineNodes: ["Highlight", "HighlightMark"],
  parseInline: [
    {
      name: "Highlight",
      parse(cx, next, pos) {
        if (next !== EQUALS || cx.char(pos + 1) !== EQUALS) return -1;
        const equalsBefore = cx.char(pos - 1) === EQUALS && cx.char(pos - 2) !== BACKSLASH;
        if (equalsBefore || cx.char(pos + 2) === EQUALS) return -1;
        const spaceBefore = /\s|^$/.test(cx.slice(pos - 1, pos));
        const spaceAfter = /\s|^$/.test(cx.slice(pos + 2, pos + 3));
        return cx.addDelimiter(DELIMITER, pos, pos + 2, !spaceAfter, !spaceBefore);
      },
      after: "Emphasis",
    },
  ],
};
