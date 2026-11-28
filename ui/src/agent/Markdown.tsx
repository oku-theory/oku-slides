// The Markdown of the agent's replies, rendered as React elements.
//
// CommonMark + GFM from `@lezer/markdown` (already there for the editor), walked by the shared
// `renderMarkdown`: no HTML is ever injected, the agent's raw HTML shows as text. A streaming
// text is re-read at each chunk; a delimiter not yet closed stays as is. A link shows its
// destination as its title; a web link the window's opener refuses is shown broken.

import { GFM, parser as commonmark } from "@lezer/markdown";
import type { ReactNode } from "react";

import { t } from "../i18n";
import { encodeLink } from "../markdown";
import { MarkdownBoundary, type MarkdownHooks, renderMarkdown } from "../markdownRender";

const parser = commonmark.configure(GFM);

/** The links a window's opener opens: the agent window's refuses `http:`. */
const OPENED = { workspace: /^(https?:|mailto:)/i, agent: /^(https:|mailto:)/i };

/** A web link: one the opener refuses is shown broken rather than as a dead click. */
const WEB = /^https?:/i;

/** A link to `href`, which the window's opener accepts. */
const external = (href: string, label: ReactNode, key: string) => (
  <a key={key} href={encodeLink(href)} title={href} target="_blank" rel="noopener noreferrer">{label}</a>
);

/** A web link the window cannot open: styled as a broken link, titled as blocked. */
const broken = (label: ReactNode, key: string) => (
  <span key={key} className="md-link-broken" title={t("ui-markdown-link-blocked")}>{label}</span>
);

/** The hooks of a window whose opener accepts `opened`. */
function hooksFor(opened: RegExp): MarkdownHooks {
  const link = (href: string, label: ReactNode, key: string) =>
    opened.test(href) ? external(href, label, key) : WEB.test(href) ? broken(label, key) : null;
  return {
    link: (href, label, key) => link(href, label, key) ?? <span key={key}>{label}</span>,
    url: (href, key) => link(href, href, key) ?? href,
    // No image is loaded from a reply: its text alone.
    image: (_src, alt, key) => <span key={key}>{alt}</span>,
    // A heading stays discreet in a chat card.
    heading: (_level, children, key) => <p key={key} className="md-heading">{children}</p>,
  };
}

const HOOKS = { workspace: hooksFor(OPENED.workspace), agent: hooksFor(OPENED.agent) };

/** An agent reply's Markdown, walked once per text. */
function Body({ text, agentWindow }: Props) {
  return renderMarkdown(text, parser, agentWindow ? HOOKS.agent : HOOKS.workspace);
}

interface Props {
  text: string;
  /** Rendered in the agent window, whose opener refuses `http:` links. */
  agentWindow: boolean;
}

/** An agent reply in Markdown; a render that throws shows a notice and the text instead. */
export function Markdown({ text, agentWindow }: Props) {
  return <MarkdownBoundary text={text}><Body text={text} agentWindow={agentWindow} /></MarkdownBoundary>;
}
