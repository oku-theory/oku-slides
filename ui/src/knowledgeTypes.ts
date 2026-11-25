// The deck knowledge base as the user reviews it: what awaits approval, a file's text, an approval.

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
