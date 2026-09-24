// Carrousel des modèles de slides, posé sur la table au-dessus de la feuille.

import { useEffect, useMemo, useRef, useState } from "react";
import type { TemplateView } from "./api";
import { api } from "./api";

interface Props {
  /** Change quand le thème ou les jetons changent : les aperçus sont rechargés. */
  themeKey: string;
  onPick: (t: TemplateView) => void;
  onClose: () => void;
}

export default function TemplatePicker({ themeKey, onPick, onClose }: Props) {
  const [items, setItems] = useState<TemplateView[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [focus, setFocus] = useState(0);
  const strip = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let alive = true;
    api.templates().then((t) => alive && setItems(t)).catch((e) => alive && setError(String(e)));
    return () => {
      alive = false;
    };
  }, [themeKey]);

  const urls = useMemo(() => {
    const list = (items ?? []).map((t) => URL.createObjectURL(new Blob([t.svg], { type: "image/svg+xml" })));
    return list;
  }, [items]);
  useEffect(() => () => urls.forEach((u) => URL.revokeObjectURL(u)), [urls]);

  // La carte au focus reste visible dans la bande.
  useEffect(() => {
    strip.current?.children[focus]?.scrollIntoView({ behavior: "smooth", inline: "center", block: "nearest" });
  }, [focus, items]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const n = items?.length ?? 0;
      if (e.key === "Escape") onClose();
      else if (e.key === "ArrowRight") setFocus((f) => Math.min(n - 1, f + 1));
      else if (e.key === "ArrowLeft") setFocus((f) => Math.max(0, f - 1));
      else if (e.key === "Enter" && items?.[focus]) onPick(items[focus]);
      else return;
      e.preventDefault();
      e.stopPropagation();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [items, focus, onPick, onClose]);

  return (
    <div className="picker-scrim" onPointerDown={onClose}>
      <div className="picker" role="dialog" aria-modal="true" aria-labelledby="picker-title" onPointerDown={(e) => e.stopPropagation()}>
        <div className="picker-head">
          <h2 id="picker-title">Nouvelle slide</h2>
          <span className="caption">← → pour parcourir · Entrée pour ajouter · Échap pour fermer</span>
          <button className="object token" aria-label="Fermer" onClick={onClose}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"><path d="M6 6l12 12M18 6L6 18" /></svg>
          </button>
        </div>
        {error && <p className="caption">{error}</p>}
        <div className="picker-row">
          <button className="object token picker-nav" aria-label="Modèle précédent" disabled={focus === 0} onClick={() => setFocus((f) => Math.max(0, f - 1))}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M15 6l-6 6 6 6" /></svg>
          </button>
          <div className="picker-strip" ref={strip} role="listbox" aria-label="Modèles de slides">
            {(items ?? Array.from({ length: 6 }, () => null)).map((t, i) => (
              <button
                key={t?.id ?? i}
                role="option"
                aria-selected={i === focus}
                className={`template${i === focus ? " focused" : ""}`}
                disabled={!t}
                onMouseEnter={() => setFocus(i)}
                onFocus={() => setFocus(i)}
                onClick={() => t && onPick(t)}
              >
                {t ? <img src={urls[i]} alt="" draggable={false} /> : <span className="template-empty" />}
                <span className="template-name">{t?.name ?? "…"}</span>
                <span className="caption">{t?.description ?? ""}</span>
              </button>
            ))}
          </div>
          <button className="object token picker-nav" aria-label="Modèle suivant" disabled={!items || focus >= items.length - 1} onClick={() => setFocus((f) => Math.min((items?.length ?? 1) - 1, f + 1))}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M9 6l6 6-6 6" /></svg>
          </button>
        </div>
      </div>
    </div>
  );
}
