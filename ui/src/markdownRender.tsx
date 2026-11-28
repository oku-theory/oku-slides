// The shared Markdown walk: a `@lezer/markdown` tree turned into React elements.
//
// It never injects HTML: raw HTML shows as text. It never emits an `<a>` or an `<img>` itself:
// links, bare URLs, images and headings go through the caller's hooks, which decide what is
// safe. Reference-style links and images are looked up in the document's definitions; one with
// no definition stays literal text, and a definition itself renders nothing.
//
// Nesting past `MAX_DEPTH` (a thousand `>` or nested lists) renders as plain text, so neither
// the walk nor React's tree grows with it; `MarkdownBoundary` shows a notice and the raw text if
// a render still throws, instead of unmounting the window.

import type { SyntaxNode } from "@lezer/common";
import type { MarkdownParser } from "@lezer/markdown";
import { Component, type ErrorInfo, Fragment, type ReactNode } from "react";

import { t } from "./i18n";

/** What the caller renders itself; the walk never makes a link, an image or a heading. */
export interface MarkdownHooks {
  /** A link with its raw destination (angle brackets removed) and its rendered label. */
  link(href: string, label: ReactNode, key: string): ReactNode;
  /** A bare URL: an autolink, its text being the URL. */
  url(href: string, key: string): ReactNode;
  /** An image with its raw source and its alt text. */
  image(src: string, alt: string, key: string): ReactNode;
  /** A heading with its source level and its trimmed content. */
  heading(level: 1 | 2 | 3 | 4 | 5 | 6, children: ReactNode, key: string): ReactNode;
}

/** Delimiters and metadata: never shown. */
const HIDDEN = new Set([
  "EmphasisMark", "HeaderMark", "ListMark", "QuoteMark", "CodeMark", "CodeInfo",
  "LinkMark", "LinkTitle", "LinkLabel", "StrikethroughMark", "TableDelimiter", "HardBreak",
]);

/** Block containers: the text between their children is only indentation. */
const BLOCKS = new Set(["Document", "BulletList", "OrderedList", "ListItem", "Blockquote", "Table", "TableHeader", "TableRow"]);

const HEADINGS: Record<string, 1 | 2 | 3 | 4 | 5 | 6> = {
  ATXHeading1: 1, ATXHeading2: 2, ATXHeading3: 3, ATXHeading4: 4, ATXHeading5: 5, ATXHeading6: 6,
  SetextHeading1: 1, SetextHeading2: 2,
};

/** The nodes an image's alt leaves out: marks, and a nested link's destination and title. */
const SKIPPED_IN_ALT = /Mark$|^URL$|^LinkTitle$|^LinkLabel$/;

/** The node depth past which a node's text is shown as is. */
const MAX_DEPTH = 64;

interface Walk {
  text: string;
  hooks: MarkdownHooks;
  /** Reference definitions by normalized label; the first one wins. */
  defs: Map<string, string>;
  /** The depth of the node whose children are being walked. */
  depth: number;
}

/** `text` parsed with `parser` and rendered, links, images and headings through `hooks`. */
export function renderMarkdown(text: string, parser: MarkdownParser, hooks: MarkdownHooks): ReactNode {
  const tree = parser.parse(text);
  const defs = new Map<string, string>();
  tree.iterate({
    enter: (ref) => {
      if (ref.name !== "LinkReference") return;
      const label = ref.node.getChild("LinkLabel");
      const url = ref.node.getChild("URL");
      if (!label || !url) return false;
      const name = normalizeLabel(text.slice(label.from + 1, label.to - 1));
      if (!defs.has(name)) defs.set(name, destination(text.slice(url.from, url.to)));
      return false;
    },
  });
  return <>{children(tree.topNode, { text, hooks, defs, depth: 0 })}</>;
}

/** Shows a notice and the raw text in place of a render that throws; a new `text` tries again. */
export class MarkdownBoundary extends Component<{ text: string; children: ReactNode }, { failed: boolean; text: string }> {
  state = { failed: false, text: this.props.text };

  static getDerivedStateFromProps(props: { text: string }, state: { text: string }) {
    return props.text === state.text ? null : { failed: false, text: props.text };
  }

  static getDerivedStateFromError() {
    return { failed: true };
  }

  componentDidCatch(e: unknown, info: ErrorInfo): void {
    console.error("markdown render", e, info.componentStack);
  }

  render(): ReactNode {
    if (!this.state.failed) return this.props.children;
    return (
      <>
        <p className="caption md-failed" role="note">{t("ui-markdown-failed")}</p>
        <pre className="md-failed-text" dir="auto">{this.props.text}</pre>
      </>
    );
  }
}

/**
 * A reference label as CommonMark matches it: case-folded, inner whitespace collapsed.
 *
 * `toLowerCase` stands for Unicode case folding: they differ on a few letters (ß, ẞ), where a
 * label written both ways does not match.
 */
const normalizeLabel = (label: string) => label.trim().replace(/\s+/g, " ").toLowerCase();

/** A link destination without its optional angle brackets. */
const destination = (raw: string) => (raw.startsWith("<") && raw.endsWith(">") ? raw.slice(1, -1) : raw);

/** The text of `node` from `from` to `to` without its marks, as CommonMark gives an image's alt. */
function plainText(node: SyntaxNode, w: Walk, from: number, to: number): string {
  if (w.depth >= MAX_DEPTH) return w.text.slice(from, to);
  let out = "";
  let at = from;
  w.depth++;
  try {
    for (let c = node.firstChild; c; c = c.nextSibling) {
      if (c.to <= from || c.from >= to) continue;
      out += w.text.slice(at, c.from);
      if (c.name === "Escape") out += w.text.slice(c.from + 1, c.to);
      else if (!SKIPPED_IN_ALT.test(c.name)) out += plainText(c, w, c.from, c.to);
      at = c.to;
    }
  } finally {
    w.depth--;
  }
  return out + w.text.slice(at, to);
}

/** A node's children; inside a line, the text between them too. */
function children(node: SyntaxNode, w: Walk, from = node.from, to = node.to): ReactNode[] {
  // Too deep: the text as written, markers included.
  if (w.depth >= MAX_DEPTH) return [w.text.slice(from, to)];
  const out: ReactNode[] = [];
  const inline = !BLOCKS.has(node.name);
  let at = from;
  w.depth++;
  try {
    for (let c = node.firstChild; c; c = c.nextSibling) {
      if (c.to <= from || c.from >= to) continue;
      if (inline && c.from > at) out.push(w.text.slice(at, c.from));
      const el = element(c, w, String(out.length));
      if (el !== null) out.push(el);
      at = c.to;
    }
  } finally {
    w.depth--;
  }
  if (inline && at < to) out.push(w.text.slice(at, to));
  return out;
}

/** A node's text between its code marks. */
function codeText(node: SyntaxNode, text: string): string {
  const parts: string[] = [];
  for (let c = node.firstChild; c; c = c.nextSibling) if (c.name === "CodeText") parts.push(text.slice(c.from, c.to));
  if (parts.length) return parts.join("");
  // Inline code: between the first and the last mark.
  const open = node.firstChild;
  const close = node.lastChild;
  return open && close && open !== close ? text.slice(open.to, close.from) : text.slice(node.from, node.to);
}

/**
 * The destination of a link or an image: its own URL, its reference's, or `null` when it names
 * an undefined reference. `[a]()` has an empty destination.
 */
function target(node: SyntaxNode, w: Walk, labelFrom: number, labelTo: number): string | null {
  const url = node.getChild("URL");
  if (url) return destination(w.text.slice(url.from, url.to));
  const ref = node.getChild("LinkLabel");
  if (ref) {
    // `[a][r]` names `r`; the collapsed `[a][]` names `a`.
    const inner = w.text.slice(ref.from + 1, ref.to - 1);
    return w.defs.get(normalizeLabel(inner.trim() ? inner : w.text.slice(labelFrom, labelTo))) ?? null;
  }
  const marks = node.getChildren("LinkMark");
  // `[a]()`: an inline link to nothing.
  if (marks.length >= 3 && w.text.slice(marks[2].from, marks[2].to) === "(") return "";
  // The shortcut `[a]` names `a`.
  return w.defs.get(normalizeLabel(w.text.slice(labelFrom, labelTo))) ?? null;
}

function element(node: SyntaxNode, w: Walk, key: string): ReactNode {
  const { text, hooks } = w;
  const inner = () => children(node, w);
  const level = HEADINGS[node.name];
  if (level) return hooks.heading(level, children(node, w).map(trimEdges), key);
  switch (node.name) {
    case "Paragraph":
      return <p key={key}>{inner()}</p>;
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
      // Raw block HTML: a paragraph of text, never interpreted.
      return <p key={key}>{text.slice(node.from, node.to)}</p>;
    case "LinkReference":
      // A reference definition: read by the links that name it, shown nowhere.
      return null;
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
      const marks = node.getChildren("LinkMark");
      if (marks.length < 2) return <span key={key}>{inner()}</span>;
      // The label: between `[` and `]`.
      const label = children(node, w, marks[0].to, marks[1].from);
      const href = target(node, w, marks[0].to, marks[1].from);
      if (href === null) return <Fragment key={key}>{"["}{label}{text.slice(marks[1].from, node.to)}</Fragment>;
      return hooks.link(href, label, key);
    }
    case "URL":
      // A bare URL (an autolink); the one of a `[…](…)` is read by `Link`.
      return hooks.url(text.slice(node.from, node.to), key);
    case "Escape":
      return text.slice(node.from + 1, node.to);
    case "Image": {
      const marks = node.getChildren("LinkMark");
      if (marks.length < 2) return null;
      const src = target(node, w, marks[0].to, marks[1].from);
      if (src === null) return text.slice(node.from, node.to);
      return hooks.image(src, plainText(node, w, marks[0].to, marks[1].from), key);
    }
    case "TaskMarker":
      return /x/i.test(text.slice(node.from, node.to)) ? "☑" : "☐";
    case "Table": {
      const head = node.getChild("TableHeader");
      const rows = node.getChildren("TableRow");
      // A wide table scrolls in its own box; `tabIndex` lets the keyboard scroll it too,
      // so the box is a named region: a tab stop says what it holds.
      return (
        <div key={key} className="md-table" tabIndex={0} role="region" aria-label={t("ui-agent-md-table")}>
          <table>
            {head && <thead>{row(head, w, "th", 0)}</thead>}
            {rows.length > 0 && <tbody>{rows.map((r, i) => row(r, w, "td", i))}</tbody>}
          </table>
        </div>
      );
    }
    default:
      if (HIDDEN.has(node.name)) return null;
      // Inline HTML, entities, unknown nodes: their text, as is.
      return node.firstChild ? <span key={key}>{inner()}</span> : text.slice(node.from, node.to);
  }
}

function row(node: SyntaxNode, w: Walk, cell: "th" | "td", key: number): ReactNode {
  const Cell = cell;
  return <tr key={key}>{node.getChildren("TableCell").map((c, i) => <Cell key={i}>{children(c, w).map(trimEdges)}</Cell>)}</tr>;
}

/** The edge blanks of a heading or a cell (between `#` or `|` and the text). */
function trimEdges(n: ReactNode, i: number, all: ReactNode[]): ReactNode {
  if (typeof n !== "string") return n;
  let s = n;
  if (i === 0) s = s.trimStart();
  if (i === all.length - 1) s = s.trimEnd();
  return s;
}
