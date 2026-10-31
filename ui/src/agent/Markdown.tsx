// Le Markdown des réponses de l'agent, rendu en éléments React.
//
// Analyse CommonMark + GFM de `@lezer/markdown` (déjà là pour l'éditeur),
// puis parcours de l'arbre : jamais de HTML injecté, le HTML brut de l'agent
// s'affiche comme du texte. Un texte en cours de flux se relit à chaque
// morceau ; un délimiteur pas encore fermé reste tel quel.

import type { SyntaxNode } from "@lezer/common";
import { GFM, parser as commonmark } from "@lezer/markdown";
import type { ReactNode } from "react";

const parser = commonmark.configure(GFM);

/** Délimiteurs et métadonnées : jamais affichés. */
const HIDDEN = new Set([
  "EmphasisMark", "HeaderMark", "ListMark", "QuoteMark", "CodeMark", "CodeInfo",
  "LinkMark", "LinkTitle", "LinkLabel", "StrikethroughMark", "TableDelimiter", "HardBreak",
]);

/** Conteneurs de blocs : le texte entre leurs enfants n'est que de l'indentation. */
const BLOCKS = new Set(["Document", "BulletList", "OrderedList", "ListItem", "Blockquote", "Table", "TableHeader", "TableRow"]);

/** Seuls ces liens deviennent cliquables. */
const SAFE_LINK = /^(https?:|mailto:)/i;

/** Une réponse de l'agent en Markdown. */
export function Markdown({ text }: { text: string }) {
  const tree = parser.parse(text);
  return <>{children(tree.topNode, text)}</>;
}

/** Les enfants d'un nœud ; dans une ligne, le texte entre eux aussi. */
function children(node: SyntaxNode, text: string, from = node.from, to = node.to): ReactNode[] {
  const out: ReactNode[] = [];
  const inline = !BLOCKS.has(node.name);
  let at = from;
  for (let c = node.firstChild; c; c = c.nextSibling) {
    if (c.to <= from || c.from >= to) continue;
    if (inline && c.from > at) out.push(text.slice(at, c.from));
    const el = element(c, text, out.length);
    if (el !== null) out.push(el);
    at = c.to;
  }
  if (inline && at < to) out.push(text.slice(at, to));
  return out;
}

/** Le texte d'un nœud, entre ses délimiteurs de code. */
function codeText(node: SyntaxNode, text: string): string {
  const parts: string[] = [];
  for (let c = node.firstChild; c; c = c.nextSibling) if (c.name === "CodeText") parts.push(text.slice(c.from, c.to));
  if (parts.length) return parts.join("");
  // Code en ligne : entre la première et la dernière marque.
  const open = node.firstChild;
  const close = node.lastChild;
  return open && close && open !== close ? text.slice(open.to, close.from) : text.slice(node.from, node.to);
}

function element(node: SyntaxNode, text: string, key: number): ReactNode {
  const inner = () => children(node, text);
  switch (node.name) {
    case "Paragraph":
      return <p key={key}>{inner()}</p>;
    case "ATXHeading1": case "ATXHeading2": case "ATXHeading3":
    case "ATXHeading4": case "ATXHeading5": case "ATXHeading6":
    case "SetextHeading1": case "SetextHeading2":
      return <p key={key} className="md-heading">{children(node, text).map(trimEdges)}</p>;
    case "BulletList":
      return <ul key={key}>{inner()}</ul>;
    case "OrderedList": {
      const first = node.getChild("ListItem")?.getChild("ListMark");
      const start = first ? Number.parseInt(text.slice(first.from, first.to), 10) : 1;
      return <ol key={key} start={start !== 1 ? start : undefined}>{inner()}</ol>;
    }
    case "ListItem":
      return <li key={key}>{inner()}</li>;
    case "Blockquote":
      return <blockquote key={key}>{inner()}</blockquote>;
    case "HTMLBlock":
      // Du HTML brut en bloc : un paragraphe de texte, jamais interprété.
      return <p key={key}>{text.slice(node.from, node.to)}</p>;
    case "HorizontalRule":
      return <hr key={key} />;
    case "FencedCode":
    case "CodeBlock":
      return <pre key={key}><code>{codeText(node, text).replace(/\n$/, "")}</code></pre>;
    case "InlineCode":
      return <code key={key}>{codeText(node, text)}</code>;
    case "Emphasis":
      return <em key={key}>{inner()}</em>;
    case "StrongEmphasis":
      return <strong key={key}>{inner()}</strong>;
    case "Strikethrough":
      return <del key={key}>{inner()}</del>;
    case "Link": {
      const url = node.getChild("URL");
      const href = url ? text.slice(url.from, url.to) : "";
      const marks = node.getChildren("LinkMark");
      // Le libellé : entre `[` et `]`.
      const label = marks.length >= 2 ? children(node, text, marks[0].to, marks[1].from) : inner();
      return SAFE_LINK.test(href)
        ? <a key={key} href={href} target="_blank" rel="noreferrer">{label}</a>
        : <span key={key}>{label}</span>;
    }
    case "URL": {
      // Lien nu (autolien GFM) ; celui d'un `[…](…)` est lu par `Link`.
      const href = text.slice(node.from, node.to);
      return SAFE_LINK.test(href) ? <a key={key} href={href} target="_blank" rel="noreferrer">{href}</a> : href;
    }
    case "Escape":
      return text.slice(node.from + 1, node.to);
    case "Image": {
      // Pas d'image chargée depuis une réponse : son texte seul.
      const marks = node.getChildren("LinkMark");
      return marks.length >= 2 ? <span key={key}>{text.slice(marks[0].to, marks[1].from)}</span> : null;
    }
    case "TaskMarker":
      return /x/i.test(text.slice(node.from, node.to)) ? "☑" : "☐";
    case "Table": {
      const head = node.getChild("TableHeader");
      const rows = node.getChildren("TableRow");
      // A wide table scrolls in its own box; `tabIndex` lets the keyboard scroll it too.
      return (
        <div key={key} className="md-table" tabIndex={0}>
          <table>
            {head && <thead>{row(head, text, "th", 0)}</thead>}
            {rows.length > 0 && <tbody>{rows.map((r, i) => row(r, text, "td", i))}</tbody>}
          </table>
        </div>
      );
    }
    default:
      if (HIDDEN.has(node.name)) return null;
      // HTML brut, entités, nœuds inconnus : leur texte, tel quel.
      return node.firstChild ? <span key={key}>{inner()}</span> : text.slice(node.from, node.to);
  }
}

function row(node: SyntaxNode, text: string, cell: "th" | "td", key: number): ReactNode {
  const Cell = cell;
  return <tr key={key}>{node.getChildren("TableCell").map((c, i) => <Cell key={i}>{children(c, text).map(trimEdges)}</Cell>)}</tr>;
}

/** Les blancs de bord d'un titre ou d'une cellule (entre `#` ou `|` et le texte). */
function trimEdges(n: ReactNode, i: number, all: ReactNode[]): ReactNode {
  if (typeof n !== "string") return n;
  let s = n;
  if (i === 0) s = s.trimStart();
  if (i === all.length - 1) s = s.trimEnd();
  return s;
}
