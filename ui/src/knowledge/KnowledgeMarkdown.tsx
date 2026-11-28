// A knowledge file body, rendered read-only: CommonMark with GFM tables and strikethrough.
//
// No task lists and no bare-URL autolinks. Headings shift down one level (the file title is the
// pane's `h1`). Every link goes through `resolveLink`: external ones open in a new window,
// bundle ones open a sub-tab, broken ones are marked, inert ones are plain text. An external
// link shows its destination as its title. An image shows only when it names a bundle file that
// `image` maps to a `blob:` URL; otherwise its alt text. A render that throws shows a notice.

import { parser as commonmark, Strikethrough, Table } from "@lezer/markdown";
import { createElement, Fragment, type ReactNode, useMemo } from "react";

import { t } from "../i18n";
import { encodeLink } from "../markdown";
import { MarkdownBoundary, type MarkdownHooks, renderMarkdown } from "../markdownRender";
import { Button } from "../primitives";
import { resolveBundlePath, resolveLink } from "./knowledgeLinks";

const parser = commonmark.configure([Table, Strikethrough]);

interface Props {
  /** The file's body, Markdown. */
  text: string;
  /** The path of the file `text` comes from; relative links resolve against its directory. */
  from: string;
  /** The bundle's file paths: a bundle link to one absent is broken. */
  paths: ReadonlySet<string>;
  /** Opens (or focuses) the bundle file at `path`. */
  onOpen(path: string): void;
  /** The `blob:` URL of the image at the resolved bundle `path`, or `null` for its alt text. */
  image(path: string): string | null;
}

/** An email autolink's address: no scheme, one `@`. */
const EMAIL = /^[^\s:@<>/]+@[^\s:@<>/]+$/;

/** The body, parsed again only when one of its props changes. */
function Body({ text, from, paths, onOpen, image }: Props) {
  const body = useMemo(() => {
    const link = (href: string, label: ReactNode, key: string): ReactNode => {
      const to = resolveLink(href, from, paths);
      switch (to.kind) {
        case "external":
          return <a key={key} href={encodeLink(to.url)} title={to.url} target="_blank" rel="noopener noreferrer">{label}</a>;
        case "bundle":
          return <Button key={key} className="knowledge-link" onClick={() => onOpen(to.path)}>{label}</Button>;
        case "broken":
          return <span key={key} className="md-link-broken" title={t("ui-knowledge-link-broken")}>{label}</span>;
        case "inert":
          return <Fragment key={key}>{label}</Fragment>;
      }
    };
    const hooks: MarkdownHooks = {
      link,
      // `<a@b.com>` is an email autolink: CommonMark sends it to `mailto:`.
      url: (href, key) => link(EMAIL.test(href) ? `mailto:${href}` : href, href, key),
      image: (src, alt, key) => {
        const path = resolveBundlePath(src, from);
        const url = path === null ? null : image(path);
        return url === null
          ? <span key={key} className="knowledge-image-alt">{alt}</span>
          : <img key={key} src={url} alt={alt} draggable={false} />;
      },
      heading: (level, children, key) => createElement(`h${Math.min(level + 1, 6)}`, { key }, children),
    };
    return renderMarkdown(text, parser, hooks);
  }, [text, from, paths, onOpen, image]);
  return <div className="md knowledge-md">{body}</div>;
}

/**
 * A knowledge file's body, read-only: its links resolved against `from` and `paths`, its images
 * through `image`. A render that throws shows a notice in its place.
 */
export function KnowledgeMarkdown(props: Props) {
  return <MarkdownBoundary text={props.text}><Body {...props} /></MarkdownBoundary>;
}
