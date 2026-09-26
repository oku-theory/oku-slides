// Couleur de chaque série d'un graphique : une pastille par série ; un clic
// propose la palette du thème (apparence) ou « Automatique ».

import { useState } from "react";
import type { Element } from "../api";
import { t } from "../i18n";
import { seriesColor, seriesNames } from "./chartDeck";

export function SeriesColors({ element, colors, onColor }: {
  element: Element;
  /** Palette des graphiques du thème, déjà validée (voir paletteOf). */
  colors: string[];
  /** Couleur choisie pour la série `index` ; `null` = la palette. */
  onColor: (index: number, color: string | null) => void;
}) {
  const [open, setOpen] = useState<number | null>(null);
  const names = seriesNames(element);
  if (names.length === 0 || colors.length === 0) return null;
  // « Automatique » : les premières couleurs de la palette, en diagonale.
  const auto = {
    background: `repeating-linear-gradient(135deg, ${colors.slice(0, 3).map((c, k) => `${c} ${4 * k}px ${4 * k + 4}px`).join(", ")})`,
  };
  const pick = (i: number, color: string | null) => {
    setOpen(null);
    if (color !== seriesColor(element, i)) onColor(i, color);
  };
  return (
    <div className="series-colors" role="group" aria-label={t("ui-chart-colors")}>
      {names.map((name, i) => {
        const own = seriesColor(element, i);
        return (
          <div key={i} className="series-color">
            <button
              className={`series-chip${open === i ? " on" : ""}`}
              aria-expanded={open === i}
              aria-label={t("ui-chart-color-of", { name })}
              onClick={() => setOpen(open === i ? null : i)}
            >
              <span className="swatch" style={{ background: own ?? colors[i % colors.length] }} />
              {name}
            </button>
            {open === i && (
              <div className="swatches" role="group" aria-label={t("ui-chart-color-of", { name })}>
                <button className={`swatch${own ? "" : " on"}`} style={auto} aria-pressed={!own} title={t("ui-chart-color-auto")} aria-label={t("ui-chart-color-auto")} onClick={() => pick(i, null)} />
                {colors.map((c, n) => (
                  <button
                    key={c}
                    className={`swatch${own === c ? " on" : ""}`}
                    style={{ background: c }}
                    aria-pressed={own === c}
                    aria-label={t("ui-chart-color-pick", { n: n + 1 })}
                    onClick={() => pick(i, c)}
                  />
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
