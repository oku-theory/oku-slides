// Recherche dans l'aide : une zone de saisie du bandeau, les slides trouvées
// en liste dessous. ↑ ↓ choisissent, Entrée (ou un clic) montre la slide,
// Échap efface. La session cherche (requête `search`) ; ici, le délai de
// frappe et la liste.

import { forwardRef, useEffect, useId, useImperativeHandle, useRef, useState } from "react";
import type { SearchHit } from "./api";
import { t } from "./i18n";

/** Délai entre la dernière frappe et la recherche (ms). */
const DELAY = 150;

export interface HelpSearchHandle {
  focus: () => void;
}

const HelpSearch = forwardRef<HelpSearchHandle, {
  search: (query: string) => Promise<SearchHit[]>;
  onShow: (slide: string) => void;
  /** Délai de frappe (ms) ; les tests le mettent à 0. */
  delay?: number;
}>(function HelpSearch({ search, onShow, delay = DELAY }, ref) {
  const [query, setQuery] = useState("");
  // `null` : rien cherché (requête vide) ; sinon les résultats de la dernière recherche.
  const [hits, setHits] = useState<SearchHit[] | null>(null);
  const [active, setActive] = useState(0);
  const [open, setOpen] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const list = useId();
  useImperativeHandle(ref, () => ({ focus: () => input.current?.select() }), []);

  // La dernière frappe l'emporte : une réponse arrivée après une autre frappe est ignorée.
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setHits(null);
      return;
    }
    let live = true;
    const timer = setTimeout(() => {
      search(q)
        .then((h) => {
          if (!live) return;
          setHits(h);
          setActive(0);
          setOpen(true);
        })
        .catch(() => live && setHits([]));
    }, delay);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [query, search, delay]);

  const show = (hit: SearchHit | undefined) => {
    if (!hit) return;
    onShow(hit.slide);
    setOpen(false);
  };

  const onKey = (e: React.KeyboardEvent<HTMLInputElement>) => {
    const n = hits?.length ?? 0;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      if (!n) return;
      e.preventDefault();
      setOpen(true);
      setActive((i) => (i + (e.key === "ArrowDown" ? 1 : n - 1)) % n);
    } else if (e.key === "Enter") {
      e.preventDefault();
      show(hits?.[active]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      setQuery("");
      setOpen(false);
    }
  };

  const shown = open && hits !== null && hits.length > 0;
  const optionId = (i: number) => `${list}-${i}`;
  return (
    <div className="help-search">
      <input
        ref={input}
        type="search"
        role="combobox"
        aria-label={t("ui-help-search")}
        aria-expanded={shown}
        aria-controls={list}
        aria-autocomplete="list"
        aria-activedescendant={shown ? optionId(active) : undefined}
        placeholder={t("ui-help-search-placeholder")}
        value={query}
        onChange={(e) => setQuery(e.currentTarget.value)}
        onKeyDown={onKey}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
      />
      {/* Le nombre de résultats, pour un lecteur d'écran ; « Aucun résultat » se voit aussi. */}
      <span className={`help-search-status${open && hits?.length === 0 ? " empty object" : ""}`} role="status">
        {hits === null ? "" : hits.length ? t("ui-help-search-count", { count: hits.length }) : t("ui-help-search-empty")}
      </span>
      {shown && (
        <ul id={list} className="help-search-results object" role="listbox" aria-label={t("ui-help-search")}>
          {hits.map((h, i) => (
            <li
              key={h.slide}
              id={optionId(i)}
              role="option"
              aria-selected={i === active}
              // Avant le `blur` de la saisie, qui fermerait la liste.
              onMouseDown={(e) => {
                e.preventDefault();
                show(h);
              }}
              onMouseEnter={() => setActive(i)}
            >
              <strong>{h.title ?? h.slide}</strong>
              {h.excerpt && <span className="caption">{h.excerpt}</span>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
});

export default HelpSearch;
