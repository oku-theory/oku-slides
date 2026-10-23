// Couleur de chaque série d'un graphique : une pastille par série ; un clic
// propose la palette du thème (apparence) ou « Automatique ».

import { useState } from "react";
import { t } from "../i18n";
import type { SeriesSwatch } from "./ChartPanel";

/** Les pastilles de couleur des séries qu'on peut colorer. */
export function SeriesColors({ series, colors, onColor }: {
  series: readonly SeriesSwatch[];
  /** Palette des graphiques du thème, en `#rrggbb` (validée par la session). */
  colors: readonly string[];
  /** Couleur choisie pour la série `index` ; `null` = la palette. */
  onColor: (index: number, color: string | null) => void;
}) {
  const [open, setOpen] = useState<number | null>(null);
  if (series.length === 0 || colors.length === 0) return null;
  // « Automatique » : les premières couleurs de la palette, en diagonale.
  const auto = {
    background: `repeating-linear-gradient(135deg, ${colors.slice(0, 3).map((c, k) => `${c} ${4 * k}px ${4 * k + 4}px`).join(", ")})`,
  };
  const pick = (i: number, color: string | null) => {
    setOpen(null);
    if (color !== (series[i].color ?? null)) onColor(i, color);
  };
  return (
    <div className="series-colors" role="group" aria-label={t("ui-chart-colors")}>
      {series.map(({ name, color }, i) => {
        const own = color ?? null;
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
