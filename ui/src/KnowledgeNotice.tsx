// The deck tab's banner: knowledge files await approval, or the knowledge base is past its limits.

import type { KnowledgeSummary } from "./api";
import { locale, t } from "./i18n";
import type { KnowledgeLimitSlug } from "./knowledgeTypes";
import { Button } from "./primitives";

/** Each ceiling's name, as literal message ids. */
const LIMIT: Record<KnowledgeLimitSlug, string> = {
  "too-many-files": "ui-knowledge-limit-too-many-files",
  "file-too-large": "ui-knowledge-limit-file-too-large",
  "total-too-large": "ui-knowledge-limit-total-too-large",
};

/** The over-limit sentence, naming each ceiling passed; an unknown ceiling throws. */
export function overLimitText(slugs: KnowledgeLimitSlug[]): string {
  const names = slugs.map((slug) => {
    if (!Object.hasOwn(LIMIT, slug)) throw new Error(`unknown knowledge limit: ${slug}`);
    return t(LIMIT[slug]);
  });
  const limits = new Intl.ListFormat(locale(), { type: "conjunction" }).format(names);
  return t("ui-knowledge-over-limit", { limits });
}

/** What the banner shows and what Review does. */
interface Props {
  summary: KnowledgeSummary | null;
  onReview: () => void;
}

/** The banner above the slide view; nothing when all is well. */
export default function KnowledgeNotice({ summary, onReview }: Props) {
  if (!summary || (summary.awaiting === 0 && summary.over_limit.length === 0)) return null;
  return (
    <div className="knowledge-notice" role="status">
      {summary.awaiting > 0 && (
        <p className="knowledge-notice-text">
          {t("ui-knowledge-awaiting", { count: summary.awaiting })}
        </p>
      )}
      {summary.over_limit.length > 0 && (
        <p className="knowledge-notice-text">{overLimitText(summary.over_limit)}</p>
      )}
      <Button ghost onClick={onReview}>{t("ui-knowledge-review")}</Button>
    </div>
  );
}
