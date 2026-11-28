// The deck knowledge base as the user reads it: what awaits approval, a file's text, an approval,
// the tree, a page and its images.

/** A ceiling the knowledge bundle passes. */
export type KnowledgeLimitSlug = "too-many-files" | "file-too-large" | "total-too-large";

/** What the snapshot says of the knowledge base; `null` when nothing awaits and no ceiling is passed. */
export interface KnowledgeSummary {
  /** Files awaiting the user's approval. */
  awaiting: number;
  over_limit: KnowledgeLimitSlug[];
}

/** One knowledge file awaiting approval. */
export interface AwaitingKnowledge {
  id: string;
  /** Its path, cleaned for display. */
  path: string;
  /** Its frontmatter title, cleaned and cut. */
  title: string | null;
  bytes: number;
}

/** The awaiting files by path, at most 200; `more` counts the rest. */
export interface KnowledgeReview {
  files: AwaitingKnowledge[];
  more: number;
  over_limit: KnowledgeLimitSlug[];
}

/** A knowledge file's text for review, with the hash an approval names. */
export interface KnowledgeText {
  id: string;
  path: string;
  /** Hash of the whole text (`sha256:<hex>`). */
  hash: string;
  /** The text, cut at the read ceiling. */
  text: string;
  /** Bytes left out of `text`. */
  truncated: number;
}

/** One file approved, for the text whose hash `knowledgeText` answered. */
export interface KnowledgeApproval {
  id: string;
  hash: string;
}

/** One file of the knowledge tree; title and kind are null while awaiting. */
export interface KnowledgeNode {
  id: string;
  /** Its whole path: the key of links and folders, shortened for display by CSS only. */
  path: string;
  title: string | null;
  kind: string | null;
  awaiting: boolean;
}

/** Every knowledge file by path. */
export interface KnowledgeTree {
  files: KnowledgeNode[];
}

/** A cited source; the url is text, never a link. */
interface KnowledgeSourceView {
  resource: string | null;
  title: string | null;
  url: string | null;
}

/** A page's header, every string cleaned and short. */
export interface KnowledgeHeaderView {
  kind: string | null;
  title: string | null;
  description: string | null;
  status: string | null;
  stale: boolean;
  tags: string[];
  generated_by: string | null;
  generated_at: string | null;
  verified: string | null;
  sources: KnowledgeSourceView[];
}

/** An image a page cites: its bundle path and asset id. */
interface KnowledgeImage {
  path: string;
  asset: string;
}

/** A file as the tab shows it; an awaiting one has no header and no images. */
export interface KnowledgePage {
  id: string;
  /** Its whole path: relative links and images resolve against it. */
  path: string;
  awaiting: boolean;
  header: KnowledgeHeaderView | null;
  /** The approved text opens a frontmatter that does not parse: the body is the raw text. */
  header_invalid: boolean;
  body: string;
  /** Bytes left out past the read ceiling. */
  truncated_bytes: number;
  images: KnowledgeImage[];
}

/** An image's bytes: png, jpeg, gif or webp, base64. */
export interface KnowledgeAsset {
  mime: string;
  data: string;
}

/** The deck's knowledge revision: the tab re-reads when it moves. */
export interface KnowledgeRevision {
  generation: number;
  approvals: number;
  /** The deck tab shows its history: knowledge reads are refused meanwhile. */
  in_history: boolean;
}
