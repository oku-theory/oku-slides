// An approved knowledge file's header: its title, description and frontmatter fields.
//
// Every string comes from the file, cleaned and cut by the session, and shows as text only: a
// source's url never becomes a link, and nothing is used as a key, an id or an attribute.

import type { ReactNode } from "react";
import { t } from "../i18n";
import type { KnowledgeHeaderView } from "./types";

/** One field: its label, from Fluent, then its value. */
function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <>
      <dt>{label}</dt>
      <dd>{children}</dd>
    </>
  );
}

/** The title (`fallback`, the file name, without one), the description, then each field present. */
export function KnowledgeHeader({ header, fallback }: { header: KnowledgeHeaderView | null; fallback: string }) {
  const generated = header && (header.generated_by !== null || header.generated_at !== null);
  return (
    <header className="knowledge-header">
      <h1><bdi>{header?.title ?? fallback}</bdi></h1>
      {header?.stale && <span className="knowledge-stale">{t("ui-knowledge-header-stale")}</span>}
      {header && header.description !== null && <p className="knowledge-description" dir="auto">{header.description}</p>}
      {header && (
        <dl className="knowledge-fields">
          {header.kind !== null && <Field label={t("ui-knowledge-header-type")}><bdi>{header.kind}</bdi></Field>}
          {header.status !== null && <Field label={t("ui-knowledge-header-status")}><bdi>{header.status}</bdi></Field>}
          {header.tags.length > 0 && (
            <Field label={t("ui-knowledge-header-tags")}>
              <ul className="knowledge-tags">
                {header.tags.map((tag, i) => <li key={i}><bdi>{tag}</bdi></li>)}
              </ul>
            </Field>
          )}
          {generated && (
            <Field label={t("ui-knowledge-header-generated")}>
              {header.generated_by !== null && <bdi>{header.generated_by}</bdi>}
              {header.generated_at !== null && <bdi>{header.generated_at}</bdi>}
            </Field>
          )}
          {header.verified !== null && <Field label={t("ui-knowledge-header-verified")}><bdi>{header.verified}</bdi></Field>}
          {header.sources.length > 0 && (
            <Field label={t("ui-knowledge-header-sources")}>
              <ul className="knowledge-sources">
                {header.sources.map((s, i) => (
                  <li key={i}>
                    {s.title !== null && <bdi className="knowledge-source-title">{s.title}</bdi>}
                    {s.resource !== null && <bdi>{s.resource}</bdi>}
                    {s.url !== null && <bdi className="knowledge-source-url">{s.url}</bdi>}
                  </li>
                ))}
              </ul>
            </Field>
          )}
        </dl>
      )}
    </header>
  );
}
