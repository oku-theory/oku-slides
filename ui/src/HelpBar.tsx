// Bandeau de l'aide : le deck livré avec l'app, en lecture seule. Il dit où
// l'on est, cherche dans les slides (⌘F), fait passer d'une slide à l'autre
// (boutons et clavier) et ferme l'onglet. Même habit que le bandeau de
// l'historique (`history-bar`).

import { useEffect, useRef } from "react";
import type { SearchHit } from "./api";
import { slideStep, slideTarget } from "./slideKeys";
import HelpSearch, { type HelpSearchHandle } from "./HelpSearch";
import { t } from "./i18n";
import { Icon } from "./icons";
import { IconButton } from "./primitives";

const TEXT_ENTRY = "input, textarea, select, [contenteditable]";

export default function HelpBar({ title, slides, current, onShow, search, onClose }: {
  title: string;
  slides: readonly { id: string }[];
  /** Slide montrée ; `null` avant la première. */
  current: string | null;
  onShow: (id: string) => void;
  /** Les slides où un texte apparaît (requête `search` de la session). */
  search: (query: string) => Promise<SearchHit[]>;
  /** Fermer l'onglet ; absent : il ne se ferme pas d'ici (le dernier onglet reste). */
  onClose?: () => void;
}) {
  const index = slides.findIndex((s) => s.id === current);
  const go = (to: number | null) => to !== null && onShow(slides[to].id);

  // Les touches de navigation, sauf dans une saisie ; Espace reste au bouton qui a le focus.
  const keys = useRef({ index, count: slides.length, go });
  keys.current = { index, count: slides.length, go };
  const finder = useRef<HelpSearchHandle>(null);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (e.defaultPrevented) return;
      // ⌘F (Ctrl+F) : la recherche de l'aide, partout dans l'onglet.
      if ((e.metaKey || e.ctrlKey) && !e.altKey && !e.shiftKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        finder.current?.focus();
        return;
      }
      if (target?.closest?.(TEXT_ENTRY)) return;
      const k = keys.current;
      const to = slideTarget(e, k.index, k.count, { space: !target?.closest?.("button, a, [role=button]") });
      if (to === null) return;
      e.preventDefault();
      k.go(to);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const shown = Math.max(0, index);
  return (
    <section className="history-bar help-bar" aria-label={t("ui-help")}>
      <div className="history-version" role="status">
        <span className="brand">{t("ui-help")}</span>
        <span className="history-title">{title}</span>
        <span className="caption">{t("ui-help-caption", { index: shown + 1, count: slides.length })}</span>
      </div>
      <HelpSearch ref={finder} search={search} onShow={onShow} />
      <div className="history-actions">
        <button className="object token" aria-label={t("ui-slide-previous")} title={t("ui-slide-previous")} disabled={slideStep(index, -1, slides.length) === null} onClick={() => go(slideStep(index, -1, slides.length))}>{Icon.previous}</button>
        <button className="object token" aria-label={t("ui-slide-next")} title={t("ui-slide-next")} disabled={slideStep(index, 1, slides.length) === null} onClick={() => go(slideStep(index, 1, slides.length))}>{Icon.next}</button>
        {onClose && <IconButton className="object token" label={t("ui-help-close")} icon={Icon.close} onClick={onClose} />}
      </div>
    </section>
  );
}
