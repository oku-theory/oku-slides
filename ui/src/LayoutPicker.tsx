// Sélecteur de layouts : une section par pack de layouts épinglé, une carte par layout.

import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { errorText, type TabApi } from "./api";
import { type LayoutPackView, type LayoutView } from "./packTypes";
import { t } from "./i18n";
import { useModalFrame } from "./modalFrame";

interface Props {
  /** Requêtes de l'onglet : les layouts de ses packs épinglés. */
  api: Pick<TabApi, "layouts">;
  /** Change quand le thème ou les jetons changent : les aperçus sont rechargés. */
  themeKey: string;
  onPick: (layout: LayoutView) => void;
  onClose: () => void;
}

/**
 * Fenêtre modale du bouton « Nouvelle slide » : les layouts de chaque pack épinglé, sous son nom.
 *
 * Dans le cadre de `useModalFrame` (Échap ferme, clics des premières 500 ms ignorés). La première
 * carte prend le focus au chargement ; ← → passent d'une carte à l'autre, d'un pack au suivant ;
 * Entrée ajoute la carte au focus.
 */
export default function LayoutPicker({ api, themeKey, onPick, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const { onClickCapture, armed } = useModalFrame(ref, { onEscape: onClose });
  const [packs, setPacks] = useState<LayoutPackView[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const urls = useSvgUrls(packs);

  useEffect(() => {
    let alive = true;
    setPacks(null);
    setError(null);
    api
      .layouts()
      .then((v) => alive && setPacks(v.packs))
      .catch((e) => alive && setError(errorText(e)));
    return () => {
      alive = false;
    };
  }, [themeKey, api]);

  const cards = () => [...(ref.current?.querySelectorAll<HTMLButtonElement>("button.layout-card") ?? [])];

  // La première carte prend le focus une fois les layouts arrivés (le cadre l'avait posé sur la fenêtre).
  useEffect(() => {
    if (packs) cards()[0]?.focus();
  }, [packs]);

  // Les touches restent dans la fenêtre : rien n'atteint les raccourcis de l'app.
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    e.stopPropagation();
    const step = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (step === 0 || e.nativeEvent.isComposing) return;
    const list = cards();
    const at = list.findIndex((b) => b === document.activeElement);
    const next = list[Math.min(list.length - 1, Math.max(0, at + step))];
    next?.focus();
    next?.scrollIntoView?.({ block: "nearest", inline: "nearest" });
    e.preventDefault();
  };

  return (
    <div className="picker-scrim" onPointerDown={onClose}>
      <div
        ref={ref}
        className="picker layout-picker"
        role="dialog"
        aria-modal="true"
        aria-labelledby="layout-picker-title"
        tabIndex={-1}
        data-armed={armed || undefined}
        onClickCapture={onClickCapture}
        onPointerDown={(e) => e.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <div className="picker-head">
          <h2 id="layout-picker-title">{t("ui-picker-new-slide")}</h2>
          <span className="caption">{t("ui-picker-keys")}</span>
          <button className="object token" aria-label={t("ui-picker-close")} onClick={onClose}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"><path d="M6 6l12 12M18 6L6 18" /></svg>
          </button>
        </div>
        {error && <p className="caption" role="alert">{error}</p>}
        {!packs && !error && <p className="caption">{t("ui-loading")}</p>}
        <div className="layout-packs" role="group" aria-label={t("ui-picker-list")}>
          {(packs ?? []).map((pack) => (
            <section key={pack.namespace} className="layout-pack" aria-labelledby={`layout-pack-${pack.namespace}`}>
              <h3 id={`layout-pack-${pack.namespace}`}>{pack.name}</h3>
              <div className="layout-cards">
                {pack.layouts.map((layout) => (
                  <button key={layout.kind} className="layout-card" onClick={() => onPick(layout)}>
                    <img src={urls.get(layout.kind)} alt="" draggable={false} />
                    <span className="card-name">{layout.label}</span>
                  </button>
                ))}
              </div>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}

/** Adresses des aperçus SVG des layouts, par kind ; révoquées quand les layouts changent. */
function useSvgUrls(packs: LayoutPackView[] | null): Map<string, string> {
  // Une Map, pas un objet : un kind nommé `constructor` ne lit pas une propriété héritée.
  const [urls, setUrls] = useState<Map<string, string>>(() => new Map());
  useEffect(() => {
    const made = new Map<string, string>();
    for (const layout of (packs ?? []).flatMap((p) => p.layouts)) {
      made.set(layout.kind, URL.createObjectURL(new Blob([layout.svg], { type: "image/svg+xml" })));
    }
    setUrls(made);
    return () => made.forEach((u) => URL.revokeObjectURL(u));
  }, [packs]);
  return urls;
}
